// Test-only: load only in the separate SHM boundary epoch, never normal/RED profile.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/desktop/view/WLSurface.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/protocols/core/Seat.hpp>
#include <hyprland/src/render/SHMSnapshot.hpp>
#include <array>
#include <charconv>
#include <cstring>
#include <limits>
#include <sstream>
#include <stdexcept>
#include <string>
#include <time.h>
#include <type_traits>
#include <utility>
#include <sys/types.h>

namespace {
struct Held {
    Render::CSHMSnapshot snapshot;
    PrShmDescription description{};
    uint64_t deadline = 0;
    pid_t peer = 0;
    uint32_t surfaceId = 0;
};
std::array<Held, 3> held; // bounded: one window, cursor, unassigned handle; no live-object escrow
SP<SHyprCtlCommand> command;
HANDLE pluginHandle = nullptr;
uint64_t nowNs() {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) || ts.tv_sec < 0)
        return 0;
    return static_cast<uint64_t>(ts.tv_sec) * 1000000000ULL + static_cast<uint64_t>(ts.tv_nsec);
}
std::string failure(const char* reason) { return std::string("{\"ok\":false,\"reason\":\"") + reason + "\"}"; }
bool parseNumber(const std::string& s, uint64_t& value) {
    if (s.empty()) return false;
    const auto result = std::from_chars(s.data(), s.data() + s.size(), value);
    return result.ec == std::errc{} && result.ptr == s.data() + s.size();
}
bool sameDescription(const PrShmDescription& a, const PrShmDescription& b) {
    return a.resource_id == b.resource_id && a.generation == b.generation && a.commit_sequence == b.commit_sequence &&
        a.drm_format == b.drm_format && a.width == b.width && a.height == b.height && a.stride == b.stride &&
        a.bytes_per_pixel == b.bytes_per_pixel && a.reserved == b.reserved;
}
uint32_t expectedPixel(uint32_t x, uint32_t y, uint32_t w, uint32_t h, bool cursor) {
    constexpr uint32_t colors[]{0xff142c4cU, 0xff386084U, 0xff74a0bcU, 0xffb0c8e0U};
    return colors[(y >= h / 2 ? 2U : 0U) + (x >= w / 2 ? 1U : 0U)] ^ (cursor ? 0x00303030U : 0U);
}
bool originalPattern(const PrSnapshotView& view, bool cursor) {
    const auto& d = view.description;
    if (!view.pixels || !d.resource_id || !d.generation || !d.commit_sequence || !d.width || !d.height ||
        d.bytes_per_pixel != 4 || d.stride != static_cast<uint64_t>(d.width) * 4 ||
        view.pixel_bytes != static_cast<uint64_t>(d.stride) * d.height || (cursor && (d.width != 32 || d.height != 32))) return false;
    for (uint32_t y = 0; y < d.height; ++y)
        for (uint32_t x = 0; x < d.width; ++x) {
            uint32_t pixel = 0;
            std::memcpy(&pixel, view.pixels + static_cast<uint64_t>(y) * d.stride + static_cast<uint64_t>(x) * 4, 4);
            if (pixel != expectedPixel(x, y, d.width, d.height, cursor)) return false;
        }
    return true;
}
SP<CWLSurfaceResource> liveSurface(unsigned which, pid_t expectedPeer, uint32_t surfaceId = 0) {
    if (which == 2) {
        SP<CWLSurfaceResource> found;
        unsigned count = 0;
        PROTO::compositor->forEachSurface([&](SP<CWLSurfaceResource> candidate) {
            if (candidate->id() != surfaceId || !candidate->m_role || candidate->m_role->role() != SURFACE_ROLE_UNASSIGNED) return;
            pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
            wl_client_get_credentials(candidate->client(), &pid, &uid, &gid);
            if (pid == expectedPeer && uid == 1000) { found = candidate; ++count; }
        });
        return count == 1 ? found : nullptr;
    }
    const auto window = Desktop::focusState()->window();
    if (!window || window->m_class != "shm-boundary-client" || !window->wlSurface()) return nullptr;
    const auto surface = window->wlSurface()->resource();
    if (!surface) return nullptr;
    pid_t pid = 0; uid_t uid = 0; gid_t gid = 0;
    wl_client_get_credentials(surface->client(), &pid, &uid, &gid);
    if (pid != expectedPeer || uid != 1000) return nullptr;
    if (which == 0) return surface;
    SP<CWLSurfaceResource> found;
    unsigned count = 0;
    PROTO::compositor->forEachSurface([&](SP<CWLSurfaceResource> candidate) {
        if (candidate->client() == surface->client() && candidate->m_role && candidate->m_role->role() == SURFACE_ROLE_CURSOR) {
            ++count;
            found = candidate;
        }
    });
    return count == 1 ? found : nullptr;
}
unsigned heldCount() {
    unsigned count = 0;
    for (const auto& value : held) if (value.snapshot) ++count;
    return count;
}
std::string report(unsigned which, bool live) {
    auto& h = held[which];
    const auto now = nowNs();
    if (!now || now >= h.deadline) { h = {}; return failure("snapshot deadline expired"); }
    const auto view = h.snapshot.view();
    if (!view || !sameDescription(view->description, h.description) || !originalPattern(*view, which == 1))
        return failure("retained snapshot content or metadata mismatch");
    bool cursorMatches = false;
    bool textureOk = false;
    bool bufferReleased = false;
    bool bufferRetained = false;
    bool identityStable = true;
    PrSnapshotView currentView{};
    if (live) {
        const auto surface = liveSurface(which, h.peer, h.surfaceId);
        if (!surface) return failure("actual same-peer live surface missing");
        const auto current = surface->m_current.shmSnapshot.view();
        if (!current || !originalPattern(*current, which == 1)) return failure("live snapshot pattern missing");
        currentView = *current;
        identityStable = sameDescription(current->description, h.description);
        if (which != 2 && !identityStable) return failure("live content generation changed");
        textureOk = surface->m_current.texture && surface->m_current.texture->ok();
        bufferRetained = static_cast<bool>(surface->m_current.buffer);
        bufferReleased = !bufferRetained;
        // ok() is a returned texture-object/GLuint validity check only. This
        // witness cannot establish GL upload success or GPU completion.
        if (!textureOk || (which == 2 ? !bufferRetained : !bufferReleased)) return failure("texture validity or buffer lifecycle precondition missing");
        if (which == 1) {
            const auto data = CCursorSurfaceRole::cursorPixelData(surface);
            cursorMatches = data.size() == view->pixel_bytes && std::memcmp(data.data(), view->pixels, data.size()) == 0;
            if (!cursorMatches) return failure("actual cursor CPU bytes differ from Rust snapshot");
        }
    }
    const auto& d = view->description;
    return std::string("{\"ok\":true,\"kind\":\"") + (which == 2 ? "unassigned" : which ? "cursor" : "window") +
        "\",\"at_ns\":\"" + std::to_string(now) + "\",\"peer_pid\":" + std::to_string(h.peer) +
        ",\"resource_id\":\"" + std::to_string(d.resource_id) + "\",\"generation\":\"" + std::to_string(d.generation) +
        "\",\"commit_sequence\":\"" + std::to_string(d.commit_sequence) + "\",\"width\":" + std::to_string(d.width) +
        ",\"height\":" + std::to_string(d.height) + ",\"stride\":" + std::to_string(d.stride) +
        ",\"drm_format\":" + std::to_string(d.drm_format) + ",\"pixel_bytes\":\"" + std::to_string(view->pixel_bytes) +
        "\",\"original_pattern_every_pixel\":true,\"live_checked\":" + (live ? "true" : "false") +
        ",\"texture_ok\":" + (textureOk ? "true" : "false") + ",\"client_buffer_released\":" + (bufferReleased ? "true" : "false") +
        ",\"cursor_cpu_matches\":" + (cursorMatches ? "true" : "false") +
        ",\"surface_id\":" + std::to_string(h.surfaceId) + ",\"uid_verified\":1000,\"current_buffer_retained\":" + (bufferRetained ? "true" : "false") +
        ",\"current_description_matches_retained\":" + (identityStable ? "true" : "false") +
        ",\"current_resource_id\":\"" + std::to_string(currentView.description.resource_id) + "\",\"current_generation\":\"" + std::to_string(currentView.description.generation) +
        "\",\"current_commit_sequence\":\"" + std::to_string(currentView.description.commit_sequence) + "\",\"current_pixel_bytes\":\"" + std::to_string(currentView.pixel_bytes) +
        "\",\"current_original_pattern_every_pixel\":" + (live ? "true" : "false") + ",\"held_handles\":" + std::to_string(heldCount()) + "}";
}
std::string handle(eHyprCtlOutputFormat, std::string request) {
    std::istringstream input(request);
    std::string name, op, kind, peerText, deadlineText, surfaceText, extra;
    input >> name >> op;
    if (name != "shm_snapshot_witness") return failure("wrong command");
    if (op == "close") {
        if (input >> extra) return failure("trailing arguments");
        const auto count = heldCount();
        held = {};
        return "{\"ok\":true,\"released_handles\":" + std::to_string(count) + ",\"held_handles\":0}";
    }
    input >> kind;
    const unsigned which = kind == "window" ? 0U : kind == "cursor" ? 1U : kind == "unassigned" ? 2U : 3U;
    if (which > 2) return failure("unknown resource kind");
    if (op == "retain") {
        input >> peerText >> deadlineText;
        uint64_t peer = 0, deadline = 0, surfaceId = 0;
        if (which == 2) {
            input >> surfaceText;
            if (!parseNumber(surfaceText, surfaceId) || !surfaceId || surfaceId > UINT32_MAX) return failure("invalid actual surface id");
        }
        const auto now = nowNs();
        if ((input >> extra) || !parseNumber(peerText, peer) || !parseNumber(deadlineText, deadline) || peer == 0 ||
            peer > static_cast<uint64_t>(std::numeric_limits<pid_t>::max()) || now == 0 || deadline <= now || deadline - now > 30000000000ULL)
            return failure("invalid peer or bounded deadline");
        if (held[which].snapshot) return failure("already retained");
        const auto surface = liveSurface(which, static_cast<pid_t>(peer), static_cast<uint32_t>(surfaceId));
        if (!surface) return failure("same-peer real surface missing");
        const auto view = surface->m_current.shmSnapshot.view();
        if (!view || !originalPattern(*view, which == 1)) return failure("real snapshot pattern missing");
        Held candidate;
        candidate.snapshot = surface->m_current.shmSnapshot; // actual separate Rust clone handle
        candidate.description = view->description;
        candidate.deadline = deadline;
        candidate.peer = static_cast<pid_t>(peer);
        candidate.surfaceId = surface->id();
        held[which] = std::move(candidate);
        return report(which, true);
    }
    if (input >> extra) return failure("trailing arguments");
    if (op == "verify-live") return report(which, true);
    if (op == "verify-retained") return report(which, false);
    return failure("unknown operation");
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE handleValue) {
    static_assert(std::is_nothrow_move_constructible_v<PLUGIN_DESCRIPTION_INFO>);
    PLUGIN_DESCRIPTION_INFO description{"shm-snapshot-witness", "Bounded real-surface Rust import witness; test epoch only", "Pokelinux", "0"};
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash()) throw std::runtime_error("matching SDK required");
    pluginHandle = handleValue;
    command = HyprlandAPI::registerHyprCtlCommand(pluginHandle, SHyprCtlCommand{.name = "shm_snapshot_witness", .exact = false, .fn = handle});
    if (!command) throw std::runtime_error("could not register test witness command");
    return description;
}
APICALL EXPORT void PLUGIN_EXIT() {
    held = {};
    if (command) HyprlandAPI::unregisterHyprCtlCommand(pluginHandle, command);
    command.reset();
}
