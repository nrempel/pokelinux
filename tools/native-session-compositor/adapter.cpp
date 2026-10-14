// Matching Hyprland API glue exports facts to the Rust observer. No policy,
// grants, launch, unlock command, file output or publication authority lives here.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/managers/SeatManager.hpp>
#include <hyprland/src/managers/SessionLockManager.hpp>
#include <hyprland/src/protocols/SessionLock.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/state/MonitorState.hpp>
#include <hyprland/src/output/Monitor.hpp>
#include <hyprland/src/desktop/view/View.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <sys/socket.h>
#include <asm-generic/socket.h>
#include <poll.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <charconv>
#include <cstdio>
#include <cstring>
#include <utility>
#include <cstdlib>
#include <limits>
#include <stdexcept>
#include <string>
#include <memory>
#include <optional>
#include <vector>

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
constexpr size_t MAX_CANDIDATES = 16;
constexpr size_t MAX_LOCK_SURFACES = 16;
struct LockCandidate {
    WP<CSessionLock> object;
    std::vector<WP<CSessionLockSurface>> surfaces;
    CHyprSignalListener surfaceListener, unlockListener, destroyListener;
    uint64_t generation = 0;
    bool eligible = true, destroyed = false;
};
struct ProtocolUnlock {
    uint64_t generation, sequence, observedAt;
};
std::vector<std::unique_ptr<LockCandidate>> candidates;
LockCandidate* currentLock = nullptr;
LockCandidate* pendingUnlock = nullptr;
uint64_t acceptedGeneration = 0;
std::optional<ProtocolUnlock> protocolUnlock;
void emit();

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
struct NativePeer {
    std::string metadata = "null";
    int pidfd = -1;
    NativePeer() = default;
    NativePeer(std::string value, int owned) : metadata(std::move(value)), pidfd(owned) {}
    NativePeer(const NativePeer&) = delete;
    NativePeer& operator=(const NativePeer&) = delete;
    NativePeer(NativePeer&& other) noexcept : metadata(std::move(other.metadata)), pidfd(std::exchange(other.pidfd, -1)) {}
    ~NativePeer() { if (pidfd >= 0) close(pidfd); }
};
NativePeer nativePeer(wl_client* client, bool locker) {
    if (!client) return {};
    pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
    wl_client_get_credentials(client, &pid, &uid, &gid);
    if (pid <= 0) return {};
    int peerFd = -1;
    socklen_t length = sizeof(peerFd);
    // Failure does not transfer a descriptor even if Linux copied a prospective
    // number before a later copyout error. Never close output on syscall failure.
    if (getsockopt(wl_client_get_fd(client), SOL_SOCKET, SO_PEERPIDFD, &peerFd, &length) != 0)
        return {};
    if (peerFd < 0) return {};
    const int descriptorFlags = fcntl(peerFd, F_GETFD);
    pollfd item{peerFd, POLLIN, 0};
    const bool live = length == sizeof(peerFd)
        && descriptorFlags >= 0 && (descriptorFlags & FD_CLOEXEC) != 0
        && poll(&item, 1, 0) == 0 && item.revents == 0;
    if (!live) { close(peerFd); return {}; }
    char focus[256];
    const int n = snprintf(focus, sizeof(focus),
        "{%s\"pid\":%d,\"uid\":%u,\"gid\":%u}", locker ? "\"state\":\"mapped\"," : "", pid, uid, gid);
    if (n <= 0 || static_cast<size_t>(n) >= sizeof(focus)) { close(peerFd); return {}; }
    return {std::string(focus, static_cast<size_t>(n)), peerFd};
}
NativePeer nativeFocus(bool locked) {
    if (locked) return {};
    auto surface = g_pSeatManager->m_state.keyboardFocus.lock();
    if (!surface) return {};
    auto hlSurface = surface->m_hlSurface.lock();
    auto view = hlSurface ? hlSurface->view() : SP<Desktop::View::IView>{};
    if (!view || view->type() != Desktop::View::VIEW_TYPE_WINDOW) return {};
    auto window = Desktop::View::CWindow::fromView(view);
    if (!window || window->m_isX11 || !window->m_isMapped || !window->m_xdgSurface) return {};
    return nativePeer(surface->client(), false);
}
NativePeer nativeLocker(bool locked) {
    if (!locked) return {};
    wl_client* owner = nullptr;
    for (const auto& monitor : State::monitorState()->monitors()) {
        if (!monitor->m_enabled || !monitor->m_dpmsStatus) continue;
        const auto wrapper = g_pSessionLockManager->getSessionLockSurfaceForMonitor(monitor->m_id).lock();
        const auto protocol = wrapper ? wrapper->surface.lock() : SP<CSessionLockSurface>{};
        const auto surface = protocol ? protocol->surface() : SP<CWLSurfaceResource>{};
        if (!wrapper || !wrapper->mapped || !surface || !surface->client()) return {};
        if (owner && owner != surface->client()) return {};
        owner = surface->client();
    }
    // No active output, an unmapped output or multiple clients cannot establish
    // one mapped owner. The kernel handle comes from the actual Wayland socket.
    // Mapped ownership and clientLocked remain distinct from secure presentation.
    return nativePeer(owner, true);
}
// Exact public protocol objects, never PIDs or addresses, establish which
// observed candidate the manager accepted. Late attachment has no candidate.
bool currentlyMapped(const LockCandidate& candidate) {
    size_t active = 0;
    for (const auto& monitor : State::monitorState()->monitors()) {
        if (!monitor->m_enabled || !monitor->m_dpmsStatus) continue;
        ++active;
        const auto wrapper = g_pSessionLockManager->getSessionLockSurfaceForMonitor(monitor->m_id).lock();
        const auto surface = wrapper ? wrapper->surface.lock() : SP<CSessionLockSurface>{};
        if (!wrapper || !wrapper->mapped || !surface) return false;
        bool belongs = false;
        for (const auto& observed : candidate.surfaces) {
            const auto selected = observed.lock();
            if (selected && selected == surface) belongs = true;
        }
        if (!belongs) return false;
    }
    return active > 0;
}
void abandonAssociation() {
    for (auto& candidate : candidates) candidate->eligible = false;
    currentLock = nullptr;
    pendingUnlock = nullptr;
}
void reconcileProtocol(bool locked, bool mappedPeer) {
    // An old replaced object's request can clear the global protocol flag
    // without the current manager's unlock listener. That supplies no cause.
    // Preserve a genuine pending association for the synchronous exact listener.
    if (!PROTO::sessionLock->isLocked()) {
        if (currentLock) currentLock->eligible = false;
        currentLock = nullptr;
        return;
    }
    if (!locked || !mappedPeer || currentLock || pendingUnlock) return;
    for (auto& candidate : candidates) {
        if (!candidate->eligible || candidate->destroyed || candidate->object.expired()
            || candidate->generation != 0 || !currentlyMapped(*candidate)) continue;
        if (acceptedGeneration == std::numeric_limits<uint64_t>::max()) { closeOutput(); return; }
        candidate->generation = ++acceptedGeneration;
        currentLock = candidate.get();
        return;
    }
}
void observeLock(const SP<CSessionLock>& lock) {
    // Even a refused new request cannot preserve an earlier authentication
    // association. It never mints a generation without actual manager mapping.
    abandonAssociation();
    protocolUnlock.reset();
    admissionDenied = true;
    if (candidates.size() >= MAX_CANDIDATES) { closeOutput(); return; }
    auto candidate = std::make_unique<LockCandidate>();
    auto* selected = candidate.get();
    selected->object = lock;
    selected->surfaceListener = lock->m_events.newLockSurface.listen([selected](const SP<CSessionLockSurface>& surface) {
        if (selected->surfaces.size() >= MAX_LOCK_SURFACES) { closeOutput(); return; }
        selected->surfaces.emplace_back(surface);
    });
    selected->unlockListener = lock->m_events.unlockAndDestroy.listen([selected] {
        // The manager's earlier listener emits generic unlock before this exact
        // object's listener. A forced unlock makes the original object inert;
        // its later request cannot emit this event. Replaced objects are ineligible.
        if (pendingUnlock != selected || !selected->eligible || selected->destroyed
            || selected->generation == 0 || PROTO::sessionLock->isLocked()) return;
        uint64_t observed = 0;
        if (!boottime(observed) || sequence == std::numeric_limits<uint64_t>::max()) { closeOutput(); return; }
        protocolUnlock = ProtocolUnlock{selected->generation, sequence + 1, observed};
        selected->eligible = false;
        pendingUnlock = nullptr;
        emit();
    });
    selected->destroyListener = lock->m_events.destroyed.listen([selected] {
        selected->destroyed = true;
        selected->eligible = false;
        if (currentLock == selected) currentLock = nullptr;
        if (pendingUnlock == selected) pendingUnlock = nullptr;
        // Destruction supplies no positive cause; preserve a prior exact event.
        emit();
    });
    candidates.push_back(std::move(candidate));
    emit();
}
void emit() {
    if (output < 0) return;
    uint64_t sampled = 0;
    if (!boottime(sampled) || sequence == std::numeric_limits<uint64_t>::max()) { closeOutput(); return; }
    const bool locked = admissionDenied || PROTO::sessionLock->isLocked();
    const bool complete = locked && g_pSessionLockManager->clientLocked();
    const auto focus = nativeFocus(locked);
    const auto locker = nativeLocker(locked);
    reconcileProtocol(locked, locker.pidfd >= 0);
    if (output < 0) return;
    const std::string generation = currentLock ? std::to_string(currentLock->generation) : "null";
    std::string unlock = "null";
    if (protocolUnlock) {
        unlock = "{\"generation\":" + std::to_string(protocolUnlock->generation)
            + ",\"sequence\":" + std::to_string(protocolUnlock->sequence)
            + ",\"observed_at_ns\":" + std::to_string(protocolUnlock->observedAt) + "}";
    }
    const int peerFd = locked ? locker.pidfd : focus.pidfd;
    char frame[MAX_FRAME];
    const int n = snprintf(frame, sizeof(frame),
        "{\"source\":\"%s\",\"sequence\":%llu,\"sampled_at_ns\":%llu,\"locked\":%s,\"lock_completed\":%s,\"focus\":%s,\"locker\":%s,\"lock_generation\":%s,\"protocol_unlock\":%s}",
        incarnation.c_str(), static_cast<unsigned long long>(++sequence), static_cast<unsigned long long>(sampled),
        locked ? "true" : "false", complete ? "true" : "false", focus.metadata.c_str(),
        locker.pidfd >= 0 ? locker.metadata.c_str() : "{\"state\":\"unavailable\"}", generation.c_str(), unlock.c_str());
    if (n <= 0 || static_cast<size_t>(n) >= sizeof(frame)) { closeOutput(); return; }
    // One nonblocking atomic packet attempt. No queue, retries, detached worker
    // or renewed incarnation on failure; receiver expiration fences old facts.
    iovec bytes{frame, static_cast<size_t>(n)};
    msghdr packet{};
    packet.msg_iov = &bytes;
    packet.msg_iovlen = 1;
    alignas(cmsghdr) char control[CMSG_SPACE(sizeof(int))]{};
    if (peerFd >= 0) {
        packet.msg_control = control;
        packet.msg_controllen = sizeof(control);
        auto* rights = CMSG_FIRSTHDR(&packet);
        rights->cmsg_level = SOL_SOCKET;
        rights->cmsg_type = SCM_RIGHTS;
        rights->cmsg_len = CMSG_LEN(sizeof(int));
        std::memcpy(CMSG_DATA(rights), &peerFd, sizeof(peerFd));
    }
    // The selected NativePeer owns the exact socket-derived handle until this atomic send.
    // Its temporary copy closes after success and every failure; the receiver
    // owns the SCM_RIGHTS reference. No pidfs identity or PID reopening is used.
    if (sendmsg(output, &packet, MSG_DONTWAIT | MSG_NOSIGNAL) != n) closeOutput();
}
int tick(void*) {
    // Timer reconciliation runs after the original onLock callback has returned.
    // Clearing this local latch is not clearing any trusted actor lock intent.
    admissionDenied = false;
    if (pendingUnlock) pendingUnlock->eligible = false;
    pendingUnlock = nullptr;
    // Reclaim listener storage only outside signal callbacks. Protocol objects
    // are weak references; Source never extends their lifetime.
    std::erase_if(candidates, [](const auto& c) {
        return c.get() != currentLock && (!c->eligible || c->destroyed || c->object.expired());
    });
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
    lockListener = g_pSessionLockManager->m_events.lock.listen([] {
        abandonAssociation(); protocolUnlock.reset(); admissionDenied = true; emit();
    });
    unlockListener = g_pSessionLockManager->m_events.unlock.listen([] {
        pendingUnlock = currentLock; currentLock = nullptr; admissionDenied = false; emit();
    });
    admissionListener = PROTO::sessionLock->m_events.newLock.listen(observeLock);
    timer = wl_event_loop_add_timer(g_pCompositor->m_wlEventLoop, tick, nullptr);
    if (!timer) { closeOutput(); throw std::runtime_error("session timer unavailable"); }
    emit();
    if (output >= 0 && wl_event_source_timer_update(timer, PERIOD_MS) != 0) closeOutput();
    return {"pokelinux-session-facts", "Bounded native facts for the Rust lifecycle observer", "Pokelinux", "0"};
}
APICALL EXPORT void PLUGIN_EXIT() {
    focusListener.reset(); lockListener.reset(); unlockListener.reset(); admissionListener.reset();
    if (timer) { wl_event_source_remove(timer); timer = nullptr; }
    currentLock = nullptr; pendingUnlock = nullptr; candidates.clear(); protocolUnlock.reset();
    closeOutput();
}
