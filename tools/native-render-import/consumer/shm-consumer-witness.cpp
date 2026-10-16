// Test epoch only: real selected SHM surfaces and the linked production Cairo consumer.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/desktop/view/WLSurface.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/protocols/core/Seat.hpp>
#include <hyprland/src/pointer/PointerManager.hpp>
#include <hyprland/src/pointer/CursorImage.hpp>
#include <hyprland/src/render/SHMSnapshot.hpp>
#include <array>
#include <charconv>
#include <cstring>
#include <limits>
#include <memory>
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
std::array<Held, 2> held;
SP<SHyprCtlCommand> command;
HANDLE pluginHandle = nullptr;
uint64_t nowNs() {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) || ts.tv_sec < 0) return 0;
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
    if (cursor) return 0xff000000U | (((17U + 7U*x + 3U*y) & 255U) << 16) |
        (((29U + 5U*x + 11U*y) & 255U) << 8) | ((43U + 13U*x + 17U*y) & 255U);
    constexpr uint32_t colors[]{0xff142c4cU, 0xff386084U, 0xff74a0bcU, 0xffb0c8e0U};
    return colors[(y >= h/2 ? 2U : 0U) + (x >= w/2 ? 1U : 0U)];
}
bool originalPattern(const PrSnapshotView& v, bool cursor) {
    const auto& d = v.description;
    if (!v.pixels || !d.resource_id || !d.generation || !d.commit_sequence || !d.width || !d.height ||
        d.width > 16384 || d.height > 16384 || d.bytes_per_pixel != 4 || d.reserved != 0 ||
        d.stride != static_cast<uint64_t>(d.width)*4 + (cursor ? 16 : 0) ||
        v.pixel_bytes != static_cast<uint64_t>(d.stride)*d.height || v.pixel_bytes > 64*1024*1024 ||
        d.drm_format != (cursor ? 875713089U : 875713112U) ||
        (cursor && (d.width != 32 || d.height != 32))) return false;
    for (uint32_t y = 0; y < d.height; ++y) {
        for (uint32_t x = 0; x < d.width; ++x) {
            uint32_t pixel = 0;
            std::memcpy(&pixel, v.pixels + static_cast<uint64_t>(y)*d.stride + static_cast<uint64_t>(x)*4, 4);
            if (pixel != expectedPixel(x, y, d.width, d.height, cursor)) return false;
        }
        for (uint32_t x = d.width*4; x < d.stride; ++x)
            if (v.pixels[static_cast<uint64_t>(y)*d.stride+x] != 0xa5) return false;
    }
    return true;
}
SP<CWLSurfaceResource> liveSurface(unsigned which, pid_t peer, uint32_t id) {
    SP<CWLSurfaceResource> surface;
    if (which == 0) {
        const auto window = Desktop::focusState()->window();
        if (!window || window->m_class != "shm-boundary-client" || !window->wlSurface()) return nullptr;
        surface = window->wlSurface()->resource();
    } else {
        if (!Pointer::mgr()) return nullptr;
        const auto selected = Pointer::mgr()->currentCursorImage().surface.lock();
        if (!selected) return nullptr;
        surface = selected->resource();
        if (!surface || !surface->m_role || surface->m_role->role() != SURFACE_ROLE_CURSOR) return nullptr;
    }
    if (!surface || surface->id() != id) return nullptr;
    pid_t actual = 0; uid_t uid = 0; gid_t gid = 0;
    wl_client_get_credentials(surface->client(), &actual, &uid, &gid);
    if (actual != peer || uid != 1000) return nullptr;
    return surface;
}
unsigned heldCount() { return (held[0].snapshot ? 1U : 0U) + (held[1].snapshot ? 1U : 0U); }
struct Consumer {
    bool invoked = false, cairoOk = false, pixelsMatch = false;
    int inputStride = 0, outputStride = 0;
    uint32_t checked = 0, mismatches = 0, firstX = 0, firstY = 0, firstExpected = 0, firstActual = 0;
};
Consumer consume(SP<CWLSurfaceResource> surface, const PrSnapshotView& view) {
    Consumer r;
    auto& bytes = CCursorSurfaceRole::cursorPixelData(surface);
    // This is the actual linked production function also used by PointerManager.
    using Surface = std::unique_ptr<cairo_surface_t, decltype(&cairo_surface_destroy)>;
    Surface input(Pointer::cursorCpuInputSurface(bytes.data(), static_cast<int>(view.description.width),
        static_cast<int>(view.description.height), static_cast<int>(view.description.stride)), cairo_surface_destroy);
    r.invoked = true;
    if (!input || cairo_surface_status(input.get()) != CAIRO_STATUS_SUCCESS) return r;
    r.inputStride = cairo_image_surface_get_stride(input.get());
    Surface output(cairo_image_surface_create(CAIRO_FORMAT_ARGB32, static_cast<int>(view.description.width),
        static_cast<int>(view.description.height)), cairo_surface_destroy);
    if (!output || cairo_surface_status(output.get()) != CAIRO_STATUS_SUCCESS) return r;
    std::unique_ptr<cairo_t, decltype(&cairo_destroy)> cr(cairo_create(output.get()), cairo_destroy);
    if (!cr || cairo_status(cr.get()) != CAIRO_STATUS_SUCCESS) return r;
    cairo_set_operator(cr.get(), CAIRO_OPERATOR_SOURCE);
    cairo_set_source_surface(cr.get(), input.get(), 0, 0);
    cairo_paint(cr.get());
    cairo_surface_flush(output.get());
    if (cairo_status(cr.get()) != CAIRO_STATUS_SUCCESS || cairo_surface_status(output.get()) != CAIRO_STATUS_SUCCESS) return r;
    r.cairoOk = true;
    r.outputStride = cairo_image_surface_get_stride(output.get());
    const auto* pixels = cairo_image_surface_get_data(output.get());
    for (uint32_t y = 0; y < view.description.height; ++y)
        for (uint32_t x = 0; x < view.description.width; ++x) {
            uint32_t actual = 0;
            std::memcpy(&actual, pixels + static_cast<size_t>(y)*r.outputStride + x*4, 4);
            const auto expected = expectedPixel(x, y, 32, 32, true);
            ++r.checked;
            if (actual == expected) continue;
            if (!r.mismatches) { r.firstX=x; r.firstY=y; r.firstExpected=expected; r.firstActual=actual; }
            ++r.mismatches;
        }
    r.pixelsMatch = r.mismatches == 0;
    return r;
}
std::string report(unsigned which, bool live) {
    auto& h = held[which];
    const auto stamp = nowNs();
    if (!stamp || stamp >= h.deadline) { h = {}; return failure("snapshot deadline expired"); }
    const auto view = h.snapshot.view();
    if (!view || !sameDescription(view->description, h.description) || !originalPattern(*view, which == 1))
        return failure("retained original pattern or metadata invalid");
    bool textureOk = false, released = false, rawMatch = false;
    Consumer consumer;
    if (live) {
        const auto surface = liveSurface(which, h.peer, h.surfaceId);
        if (!surface) return failure("real selected same-peer surface missing");
        const auto current = surface->m_current.shmSnapshot.view();
        if (!current || !sameDescription(current->description, h.description) || !originalPattern(*current, which == 1))
            return failure("live snapshot metadata or pattern changed");
        textureOk = surface->m_current.texture && surface->m_current.texture->ok();
        released = !surface->m_current.buffer;
        if (!textureOk || !released) return failure("texture validity or client-buffer release incomplete");
        if (which == 1) {
            auto& data = CCursorSurfaceRole::cursorPixelData(surface);
            rawMatch = data.size() == view->pixel_bytes && std::memcmp(data.data(), view->pixels, data.size()) == 0;
            if (!rawMatch) return failure("cursor cache differs from actual retained padded snapshot");
            consumer = consume(surface, *view);
            if (!consumer.invoked || !consumer.cairoOk) return failure("actual production Cairo consumer setup failed");
        }
    }
    const auto& d = view->description;
    std::ostringstream out;
    out << std::boolalpha << "{\"ok\":true,\"kind\":\"" << (which ? "cursor" : "window") << "\",\"at_ns\":\"" << stamp
        << "\",\"peer_pid\":" << h.peer << ",\"uid_verified\":1000,\"surface_id\":" << h.surfaceId
        << ",\"resource_id\":\"" << d.resource_id << "\",\"generation\":\"" << d.generation << "\",\"commit_sequence\":\"" << d.commit_sequence
        << "\",\"width\":" << d.width << ",\"height\":" << d.height << ",\"stride\":" << d.stride << ",\"drm_format\":" << d.drm_format
        << ",\"pixel_bytes\":\"" << view->pixel_bytes << "\",\"original_pattern_every_pixel\":true,\"padding_original\":true,\"live_checked\":" << live
        << ",\"texture_ok\":" << textureOk << ",\"client_buffer_released\":" << released << ",\"selected_pointer_role\":" << (live && which == 1)
        << ",\"cursor_cache_equals_padded_snapshot\":" << rawMatch << ",\"production_consumer_invoked\":" << consumer.invoked
        << ",\"cairo_setup_ok\":" << consumer.cairoOk << ",\"consumer_input_stride\":" << consumer.inputStride << ",\"consumer_output_stride\":" << consumer.outputStride
        << ",\"consumer_pixels_checked\":" << consumer.checked << ",\"consumer_mismatches\":" << consumer.mismatches
        << ",\"consumer_original_pattern_every_pixel\":" << consumer.pixelsMatch << ",\"first_mismatch_x\":" << consumer.firstX
        << ",\"first_mismatch_y\":" << consumer.firstY << ",\"first_mismatch_expected\":" << consumer.firstExpected << ",\"first_mismatch_actual\":" << consumer.firstActual
        << ",\"held_handles\":" << heldCount() << "}";
    return out.str();
}
std::string handle(eHyprCtlOutputFormat, std::string request) {
    std::istringstream input(request);
    std::string name, op, kind, peerText, deadlineText, idText, extra;
    input >> name >> op;
    if (name != "shm_consumer_witness") return failure("wrong command");
    if (op == "close") {
        if (input >> extra) return failure("trailing arguments");
        const auto count = heldCount(); held = {};
        return "{\"ok\":true,\"released_handles\":" + std::to_string(count) + ",\"held_handles\":0}";
    }
    input >> kind;
    const unsigned which = kind == "window" ? 0U : kind == "cursor" ? 1U : 2U;
    if (which > 1) return failure("unknown kind");
    if (op == "retain") {
        input >> peerText >> deadlineText >> idText;
        uint64_t peer = 0, deadline = 0, id = 0;
        const auto stamp = nowNs();
        if ((input >> extra) || !parseNumber(peerText, peer) || !parseNumber(deadlineText, deadline) || !parseNumber(idText, id) ||
            !peer || peer > static_cast<uint64_t>(std::numeric_limits<pid_t>::max()) || !id || id > UINT32_MAX ||
            !stamp || deadline <= stamp || deadline-stamp > 30000000000ULL || held[which].snapshot) return failure("invalid bounded retain");
        const auto surface = liveSurface(which, static_cast<pid_t>(peer), static_cast<uint32_t>(id));
        if (!surface) return failure("selected real same-peer surface missing");
        const auto view = surface->m_current.shmSnapshot.view();
        if (!view || !originalPattern(*view, which == 1)) return failure("actual original source pattern invalid");
        Held candidate;
        candidate.snapshot = surface->m_current.shmSnapshot;
        candidate.description = view->description;
        candidate.peer = static_cast<pid_t>(peer); candidate.surfaceId = static_cast<uint32_t>(id); candidate.deadline = deadline;
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
    PLUGIN_DESCRIPTION_INFO description{"shm-consumer-witness", "Actual live SHM plus linked production Cairo input consumer; CPU-only test", "Pokelinux", "0"};
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash()) throw std::runtime_error("matching SDK required");
    pluginHandle = handleValue;
    command = HyprlandAPI::registerHyprCtlCommand(pluginHandle, SHyprCtlCommand{.name="shm_consumer_witness", .exact=false, .fn=handle});
    if (!command) throw std::runtime_error("could not register bounded witness");
    return description;
}
APICALL EXPORT void PLUGIN_EXIT() {
    held = {};
    if (command) HyprlandAPI::unregisterHyprCtlCommand(pluginHandle, command);
    command.reset();
}
