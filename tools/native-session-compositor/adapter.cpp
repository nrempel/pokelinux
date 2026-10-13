// Matching Hyprland API glue exports facts to the Rust observer. No policy,
// grants, launch, unlock command, file output or publication authority lives here.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/managers/SeatManager.hpp>
#include <hyprland/src/managers/SessionLockManager.hpp>
#include <hyprland/src/protocols/SessionLock.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/desktop/view/View.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <asm-generic/socket.h>
#include <poll.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <charconv>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <stdexcept>
#include <string>

namespace {
int output = -1;
std::string incarnation;
uint64_t sequence = 0;
// Signals run before the original protocol flag is assigned. Keep denial
// throughout that callback stack, including its intervening null-focus event.
bool admissionDenied = false;
wl_event_source* timer = nullptr;
CHyprSignalListener focusListener, lockListener, unlockListener, admissionListener;
constexpr int PERIOD_MS = 25;
constexpr size_t MAX_FRAME = 1024;
void closeOutput() {
    if (output >= 0) { close(output); output = -1; }
}
bool boottime(uint64_t& ns) {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0 || ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L)
        return false;
    const auto sec = static_cast<uint64_t>(ts.tv_sec);
    if (sec > (std::numeric_limits<uint64_t>::max() - static_cast<uint64_t>(ts.tv_nsec)) / 1000000000ULL)
        return false;
    ns = sec * 1000000000ULL + static_cast<uint64_t>(ts.tv_nsec);
    return true;
}
std::string nativeFocus(bool locked) {
    if (locked) return "null";
    auto surface = g_pSeatManager->m_state.keyboardFocus.lock();
    if (!surface) return "null";
    auto hlSurface = surface->m_hlSurface.lock();
    auto view = hlSurface ? hlSurface->view() : SP<Desktop::View::IView>{};
    if (!view || view->type() != Desktop::View::VIEW_TYPE_WINDOW) return "null";
    auto window = Desktop::View::CWindow::fromView(view);
    if (!window || window->m_isX11 || !window->m_isMapped || !window->m_xdgSurface) return "null";
    pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
    wl_client_get_credentials(surface->client(), &pid, &uid, &gid);
    if (pid <= 0) return "null";
    int peerFd = -1;
    socklen_t length = sizeof(peerFd);
    // Failure does not transfer a descriptor even if Linux copied a prospective
    // number before a later copyout error. Never close output on syscall failure.
    if (getsockopt(wl_client_get_fd(surface->client()), SOL_SOCKET, SO_PEERPIDFD, &peerFd, &length) != 0)
        return "null";
    if (peerFd < 0) return "null";
    struct stat st{}; struct statfs fs{};
    pollfd item{peerFd, POLLIN, 0};
    const bool live = length == sizeof(peerFd)
        && (fcntl(peerFd, F_GETFD) & FD_CLOEXEC) != 0
        && fstat(peerFd, &st) == 0 && fstatfs(peerFd, &fs) == 0
        && static_cast<unsigned long>(fs.f_type) == 0x50494446UL
        && st.st_ino != 0 && poll(&item, 1, 0) == 0 && item.revents == 0;
    close(peerFd);
    if (!live) return "null";
    char focus[256];
    const int n = snprintf(focus, sizeof(focus),
        "{\"pid\":%d,\"uid\":%u,\"gid\":%u,\"pidfs_device\":%llu,\"pidfs_inode\":%llu}",
        pid, uid, gid, static_cast<unsigned long long>(st.st_dev), static_cast<unsigned long long>(st.st_ino));
    if (n <= 0 || static_cast<size_t>(n) >= sizeof(focus)) return "null";
    return std::string(focus, static_cast<size_t>(n));
}
void emit() {
    if (output < 0) return;
    uint64_t sampled = 0;
    if (!boottime(sampled) || sequence == std::numeric_limits<uint64_t>::max()) { closeOutput(); return; }
    const bool locked = admissionDenied || PROTO::sessionLock->isLocked();
    const bool complete = locked && g_pSessionLockManager->clientLocked();
    const auto focus = nativeFocus(locked);
    char frame[MAX_FRAME];
    const int n = snprintf(frame, sizeof(frame),
        "{\"source\":\"%s\",\"sequence\":%llu,\"sampled_at_ns\":%llu,\"locked\":%s,\"lock_completed\":%s,\"focus\":%s}",
        incarnation.c_str(), static_cast<unsigned long long>(++sequence), static_cast<unsigned long long>(sampled),
        locked ? "true" : "false", complete ? "true" : "false", focus.c_str());
    if (n <= 0 || static_cast<size_t>(n) >= sizeof(frame)) { closeOutput(); return; }
    // One nonblocking atomic packet attempt. No queue, retries, detached worker
    // or renewed incarnation on failure; receiver expiration fences old facts.
    if (send(output, frame, static_cast<size_t>(n), MSG_DONTWAIT | MSG_NOSIGNAL) != n) closeOutput();
}
int tick(void*) {
    // Timer reconciliation runs after the original onLock callback has returned.
    // Clearing this local latch is not clearing any trusted actor lock intent.
    admissionDenied = false;
    emit();
    if (output >= 0 && wl_event_source_timer_update(timer, PERIOD_MS) != 0) closeOutput();
    return 0;
}
int provisionedFd() {
    const char* raw = getenv("POKELINUX_SESSION_FD");
    const char* source = getenv("POKELINUX_SESSION_SOURCE");
    if (!raw || !source) throw std::runtime_error("missing installed private session channel");
    incarnation = source;
    if (incarnation.size() != 32 || incarnation.find_first_not_of("0123456789abcdef") != std::string::npos)
        throw std::runtime_error("invalid session incarnation");
    int fd = -1;
    const std::string number(raw);
    const auto parsed = std::from_chars(number.data(), number.data() + number.size(), fd);
    if (parsed.ec != std::errc{} || parsed.ptr != number.data() + number.size() || fd < 3)
        throw std::runtime_error("invalid inherited descriptor");
    int type = 0; socklen_t length = sizeof(type);
    ucred creator{}; socklen_t creatorLength = sizeof(creator);
    const int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || getsockopt(fd, SOL_SOCKET, SO_TYPE, &type, &length) != 0 || length != sizeof(type)
        || type != SOCK_SEQPACKET
        || getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &creator, &creatorLength) != 0
        || creatorLength != sizeof(creator) || creator.uid != 0
        || fcntl(fd, F_SETFL, flags | O_NONBLOCK) != 0 || fcntl(fd, F_SETFD, FD_CLOEXEC) != 0) {
        close(fd); throw std::runtime_error("invalid root-provisioned private channel");
    }
    unsetenv("POKELINUX_SESSION_FD"); unsetenv("POKELINUX_SESSION_SOURCE");
    return fd;
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE) {
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash())
        throw std::runtime_error("exact installed compositor API mismatch");
    output = provisionedFd();
    focusListener = g_pSeatManager->m_events.keyboardFocusChange.listen([] { emit(); });
    lockListener = g_pSessionLockManager->m_events.lock.listen([] { admissionDenied = true; emit(); });
    unlockListener = g_pSessionLockManager->m_events.unlock.listen([] { emit(); });
    admissionListener = PROTO::sessionLock->m_events.newLock.listen([](const SP<CSessionLock>&) { admissionDenied = true; emit(); });
    timer = wl_event_loop_add_timer(g_pCompositor->m_wlEventLoop, tick, nullptr);
    if (!timer) { closeOutput(); throw std::runtime_error("session timer unavailable"); }
    emit();
    if (output >= 0 && wl_event_source_timer_update(timer, PERIOD_MS) != 0) closeOutput();
    return {"pokelinux-session-facts", "Bounded native facts for the Rust lifecycle observer", "Pokelinux", "0"};
}
APICALL EXPORT void PLUGIN_EXIT() {
    focusListener.reset(); lockListener.reset(); unlockListener.reset(); admissionListener.reset();
    if (timer) { wl_event_source_remove(timer); timer = nullptr; }
    closeOutput();
}
