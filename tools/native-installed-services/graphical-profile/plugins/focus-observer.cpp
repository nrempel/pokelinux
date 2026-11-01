// Private read-only native-focus witness. Does not receive or publish Source packets.
#include "startup_event_ring.hpp"
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/event/EventBus.hpp>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/managers/SeatManager.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/desktop/view/View.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <asm-generic/socket.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <poll.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <stdexcept>
#include <string>
namespace {
HANDLE owner;
CHyprSignalListener changed;
startup_events::Mapping timing;
CHyprSignalListener preReload, reloaded, propsRefreshed;
SP<SHyprCtlCommand> query;
int logFd = -1;
uint64_t sequence = 0;
constexpr uint64_t MAX_ROWS = 512;
constexpr size_t MAX_ROW = 768;
std::string observe(const char* reason) {
    if (logFd < 0 || sequence >= MAX_ROWS) throw std::runtime_error("bounded native-focus observer unavailable");
    timespec clock{};
    if (clock_gettime(CLOCK_BOOTTIME, &clock) != 0 || clock.tv_sec < 0 || clock.tv_nsec < 0 || clock.tv_nsec >= 1000000000L
        || static_cast<uint64_t>(clock.tv_sec) > (std::numeric_limits<uint64_t>::max() - clock.tv_nsec) / 1000000000ULL)
        throw std::runtime_error("native-focus clock unavailable");
    const auto stamp = static_cast<uint64_t>(clock.tv_sec) * 1000000000ULL + clock.tv_nsec;
    pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
    bool eligible = false, live = false, cloexec = false;
    int peer = -1, type = -1;
    auto surface = g_pSeatManager->m_state.keyboardFocus.lock();
    if (surface) {
        wl_client_get_credentials(surface->client(), &pid, &uid, &gid);
        socklen_t length = sizeof(peer);
        if (getsockopt(wl_client_get_fd(surface->client()), SOL_SOCKET, SO_PEERPIDFD, &peer, &length) == 0 && peer >= 0) {
            const int flags = fcntl(peer, F_GETFD); pollfd item{peer, POLLIN, 0};
            cloexec = flags >= 0 && (flags & FD_CLOEXEC) != 0;
            live = length == sizeof(peer) && cloexec && poll(&item, 1, 0) == 0 && item.revents == 0;
        }
        auto hl = surface->m_hlSurface.lock();
        auto view = hl ? hl->view() : SP<Desktop::View::IView>{};
        if (view) {
            type = static_cast<int>(view->type());
            if (view->type() == Desktop::View::VIEW_TYPE_WINDOW) {
                auto window = Desktop::View::CWindow::fromView(view);
                eligible = window && !window->m_isX11 && window->m_isMapped && window->m_xdgSurface && live;
            }
        }
    }
    if (peer >= 0) close(peer);
    char bytes[MAX_ROW];
    const int count = snprintf(bytes, sizeof(bytes),
        "{\"schema\":1,\"sequence\":%llu,\"reason\":\"%s\",\"boottime_ns\":%llu,\"pid\":%d,\"uid\":%u,\"gid\":%u,\"view_type\":%d,\"keyboard_surface_present\":%s,\"native_window_eligible\":%s,\"peer_pidfd_live\":%s,\"peer_pidfd_cloexec\":%s}\n",
        static_cast<unsigned long long>(++sequence), reason, static_cast<unsigned long long>(stamp), pid, uid, gid, type,
        surface ? "true" : "false", eligible ? "true" : "false", live ? "true" : "false", cloexec ? "true" : "false");
    if (count <= 0 || static_cast<size_t>(count) >= sizeof(bytes) || write(logFd, bytes, count) != count) {
        close(logFd); logFd = -1; throw std::runtime_error("native-focus bounded file write failed");
    }
    return std::string(bytes, count);
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE handle) {
    owner = handle;
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash() || getuid() != 1000)
        throw std::runtime_error("selected native-focus API/UID mismatch");
    struct stat parent{};
    if (lstat("/run/user/1000", &parent) != 0 || !S_ISDIR(parent.st_mode) || parent.st_uid != 1000 || (parent.st_mode & 0777) != 0700)
        throw std::runtime_error("private native-focus parent mismatch");
    logFd = open("/run/user/1000/native-audio-focus.jsonl", O_CREAT | O_EXCL | O_WRONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (logFd < 0) throw std::runtime_error("fresh private native-focus output unavailable");
    changed = g_pSeatManager->m_events.keyboardFocusChange.listen([] { observe("keyboard-focus-change"); });
    query = HyprlandAPI::registerHyprCtlCommand(owner, {"pokelinux_audio_focus", true, [](eHyprCtlOutputFormat, std::string) { return observe("query"); }});
    observe("init");
    timing.openAndPrefault();
    try {
        preReload = Event::bus()->m_events.config.preReload.listen([] { timing.publish(startup_events::Kind::PreReload); });
        reloaded = Event::bus()->m_events.config.reloaded.listen([] { timing.publish(startup_events::Kind::Reloaded); });
        propsRefreshed = Event::bus()->m_events.config.props_refreshed.listen([](const bool scheduled) { timing.publish(startup_events::Kind::PropsRefreshed, scheduled ? 1U : 0U); });
        timing.publish(startup_events::Kind::WitnessArmed);
    } catch (...) {
        timing.retire();
        preReload.reset(); reloaded.reset(); propsRefreshed.reset();
        timing.closeMapping();
        throw;
    }
    return {"pokelinux-audio-readonly-focus", "Private native keyboard focus observation only", "Pokelinux", "1"};
}
APICALL EXPORT void PLUGIN_EXIT() {
    timing.retire(); // Sticky before any timing-listener reset or unmap.
    preReload.reset(); reloaded.reset(); propsRefreshed.reset();
    timing.closeMapping();
    changed.reset();
    if (query) HyprlandAPI::unregisterHyprCtlCommand(owner, query);
    query.reset();
    if (logFd >= 0) { close(logFd); logFd = -1; }
}
