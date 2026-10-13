#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/managers/SeatManager.hpp>
#include <hyprland/src/managers/SessionLockManager.hpp>
#include <hyprland/src/protocols/SessionLock.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/desktop/view/View.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <fcntl.h>
#include <sys/socket.h>
#include <asm-generic/socket.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <poll.h>
#include <cerrno>
#include <unistd.h>
#include <cstdio>
#include <stdexcept>
#include <vector>

static HANDLE handle;
static CHyprSignalListener focusListener, lockListener, unlockListener, newLockListener;
static std::vector<CHyprSignalListener> protocolListeners;
static std::vector<SP<SHyprCtlCommand>> commands;
static wl_event_source* timerSource = nullptr;
static int logFd = -1;
static unsigned long seq = 0;
static std::string lastPeriodic;

static std::string snapshot(const char* reason, bool withSeq = true) {
    pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
    int type = -1; bool eligible = false;
    int peerFd = -1, peerErrno = 0; bool peerValid = false, peerLive = false;
    unsigned long long peerDev = 0, peerIno = 0, peerFsMagic = 0;
    static_assert(sizeof(ino_t) >= sizeof(uint64_t));
    static_assert(SO_PEERPIDFD == 77);
    auto surf = g_pSeatManager->m_state.keyboardFocus.lock();
    if (surf) {
        wl_client_get_credentials(surf->client(), &pid, &uid, &gid);
        socklen_t fdLen = sizeof(peerFd);
        if (getsockopt(wl_client_get_fd(surf->client()), SOL_SOCKET, SO_PEERPIDFD, &peerFd, &fdLen) == 0) {
            struct stat st{}; struct statfs fs{};
            if (fdLen == sizeof(peerFd) && peerFd >= 0 && fstat(peerFd, &st) == 0 && fstatfs(peerFd, &fs) == 0) {
                peerDev = static_cast<unsigned long long>(st.st_dev);
                peerIno = static_cast<unsigned long long>(st.st_ino);
                peerFsMagic = static_cast<unsigned long long>(fs.f_type);
                peerValid = peerFsMagic == 0x50494446ULL && peerIno != 0;
                pollfd pfd{peerFd, POLLIN, 0};
                peerLive = peerValid && poll(&pfd, 1, 0) == 0 && pfd.revents == 0;
            } else peerErrno = errno;
        } else peerErrno = errno;
        auto hs = surf->m_hlSurface.lock();
        auto view = hs ? hs->view() : SP<Desktop::View::IView>{};
        if (view) {
            type = static_cast<int>(view->type());
            if (type == Desktop::View::VIEW_TYPE_WINDOW) {
                auto window = Desktop::View::CWindow::fromView(view);
                eligible = window && !window->m_isX11 && window->m_isMapped && window->m_xdgSurface;
            }
        }
    }
    eligible = eligible && peerValid && peerLive;
    char buf[1536];
    snprintf(buf, sizeof(buf), "{\"reason\":\"%s\",\"seq\":%lu,\"locked\":%s,\"complete\":%s,\"pid\":%d,\"uid\":%u,\"gid\":%u,\"type\":%d,\"eligible\":%s,\"peer_pidfd_fd\":%d,\"peer_pidfd_valid\":%s,\"peer_pidfd_live\":%s,\"peer_pidfd_errno\":%d,\"peer_pidfs_dev\":%llu,\"peer_pidfs_ino\":%llu,\"peer_pidfs_magic\":%llu}",
             reason, withSeq ? seq : 0, PROTO::sessionLock->isLocked() ? "true" : "false", g_pSessionLockManager->clientLocked() ? "true" : "false", pid, uid, gid, type, eligible ? "true" : "false", peerFd, peerValid ? "true" : "false", peerLive ? "true" : "false", peerErrno, peerDev, peerIno, peerFsMagic);
    if (peerFd >= 0) close(peerFd);
    return buf;
}
static void emit(const char* reason) {
    ++seq;
    auto s = snapshot(reason) + "\n";
    if (logFd >= 0) (void)write(logFd, s.c_str(), s.size());
}
static int tick(void*) {
    auto value = snapshot("periodic", false);
    if (value != lastPeriodic) { lastPeriodic = value; emit("periodic"); }
    wl_event_source_timer_update(timerSource, 50);
    return 0;
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE h) {
    handle = h;
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash()) throw std::runtime_error("exact build API mismatch");
    logFd = open("/tmp/native-session-build-probe.ndjson", O_CREAT | O_APPEND | O_WRONLY | O_NONBLOCK | O_CLOEXEC, 0600);
    focusListener = g_pSeatManager->m_events.keyboardFocusChange.listen([] { emit("keyboardFocusChange"); });
    lockListener = g_pSessionLockManager->m_events.lock.listen([] { emit("managerLock"); });
    unlockListener = g_pSessionLockManager->m_events.unlock.listen([] { emit("managerUnlock"); });
    newLockListener = PROTO::sessionLock->m_events.newLock.listen([](const SP<CSessionLock>& lock) {
        emit("protocolNewLock");
        protocolListeners.emplace_back(lock->m_events.unlockAndDestroy.listen([] { emit("protocolUnlockAndDestroy"); }));
        protocolListeners.emplace_back(lock->m_events.destroyed.listen([] { emit("protocolDestroyed"); }));
    });
    commands.emplace_back(HyprlandAPI::registerHyprCtlCommand(handle, {"nativebuildprobe", true, [](eHyprCtlOutputFormat, std::string) { emit("commandSnapshot"); return snapshot("query"); }}));
    commands.emplace_back(HyprlandAPI::registerHyprCtlCommand(handle, {"nativebuildclearfocus", true, [](eHyprCtlOutputFormat, std::string) { g_pSeatManager->setKeyboardFocus(nullptr); return snapshot("clearFocus"); }}));
    commands.emplace_back(HyprlandAPI::registerHyprCtlCommand(handle, {"nativebuildforceunlock", true, [](eHyprCtlOutputFormat, std::string) { g_pSessionLockManager->forceUnlock(); return snapshot("forceUnlock"); }}));
    timerSource = wl_event_loop_add_timer(g_pCompositor->m_wlEventLoop, tick, nullptr);
    wl_event_source_timer_update(timerSource, 50);
    emit("init");
    return {"native-session-build-probe", "Private exact-version ABI/API execution probe", "Pokelinux", "0"};
}
APICALL EXPORT void PLUGIN_EXIT() {
    focusListener.reset(); lockListener.reset(); unlockListener.reset(); newLockListener.reset(); protocolListeners.clear();
    if (timerSource) { wl_event_source_remove(timerSource); timerSource = nullptr; }
    for (auto& c : commands) HyprlandAPI::unregisterHyprCtlCommand(handle, c);
    commands.clear();
    emit("exit");
    if (logFd >= 0) close(logFd);
}
