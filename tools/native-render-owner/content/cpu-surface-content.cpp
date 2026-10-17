#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "protocols/types/SurfaceState.hpp"
#include <drm_fourcc.h>
#include <array>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <utility>
#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

namespace {
constexpr uint32_t WIDTH = 5;
constexpr uint32_t HEIGHT = 3;
constexpr uint32_t STRIDE = 32;
constexpr size_t BYTES = STRIDE * HEIGHT;
using Bytes = std::array<uint8_t, BYTES>;
struct Failure { const char* name; int code; };
void setup(bool condition, const char* name) {
    if (!condition)
        throw Failure{name, 77};
}
void require(bool condition, const char* name) {
    if (!condition)
        throw Failure{name, 1};
}
void geometry(SSurfaceState& state, Vector2D expected, const char* name) {
    const auto actual = state.sourceSize();
    if (actual != expected) {
        std::printf("CPU_CONTENT_GEOMETRY name=%s expected=%.17g,%.17g actual=%.17g,%.17g\n",
                    name, expected.x, expected.y, actual.x, actual.y);
        throw Failure{name, 1};
    }
}
void pass(const char* name) { std::printf("CPU_CONTENT_PASS %s\n", name); }
Bytes pattern(uint8_t seed) {
    Bytes result{};
    for (uint32_t y = 0; y < HEIGHT; ++y) {
        for (uint32_t x = 0; x < WIDTH; ++x) {
            const size_t i = static_cast<size_t>(y) * STRIDE + x * 4;
            result[i] = static_cast<uint8_t>(seed + x * 17 + y * 3);
            result[i + 1] = static_cast<uint8_t>(seed ^ (x * 29 + y * 11));
            result[i + 2] = static_cast<uint8_t>(255 - seed - x * 7 - y * 19);
            result[i + 3] = static_cast<uint8_t>(x * 47 + y * 31 + seed);
        }
        for (uint32_t x = WIDTH * 4; x < STRIDE; ++x)
            result[static_cast<size_t>(y) * STRIDE + x] = static_cast<uint8_t>(seed ^ 0xA5U ^ x ^ (y * 13));
    }
    return result;
}
bool sameDescription(const PrShmDescription& a, const PrShmDescription& b) {
    return a.resource_id == b.resource_id && a.generation == b.generation && a.commit_sequence == b.commit_sequence &&
        a.drm_format == b.drm_format && a.width == b.width && a.height == b.height && a.stride == b.stride &&
        a.bytes_per_pixel == b.bytes_per_pixel && a.reserved == b.reserved;
}
void snapshotMatches(const Render::CSHMSnapshot& snapshot, const PrShmDescription& expected, const Bytes& bytes, const char* name) {
    const auto view = snapshot.view();
    require(view.has_value(), name);
    require(sameDescription(view->description, expected) && view->pixel_bytes == BYTES && view->pixels, name);
    require(std::memcmp(view->pixels, bytes.data(), BYTES) == 0, name);
}
struct Producer {
    int fd = -1;
    uint8_t* pixels = nullptr;
    ~Producer() {
        if (pixels)
            (void)munmap(pixels, BYTES);
        if (fd >= 0)
            (void)close(fd);
    }
    void dispose() {
        setup(pixels && fd >= 0, "producer_disposal_state");
        setup(munmap(pixels, BYTES) == 0, "producer_munmap");
        pixels = nullptr;
        const int oldFD = std::exchange(fd, -1);
        setup(close(oldFD) == 0, "producer_close");
        errno = 0;
        setup(fcntl(oldFD, F_GETFD) == -1 && errno == EBADF, "producer_FD_actually_closed");
    }
};
struct Captured {
    Render::CSHMSnapshot snapshot;
    PrShmDescription description{};
    Bytes expected{};
};
Captured captureAndDispose(uint8_t seed) {
    Captured result;
    result.expected = pattern(seed);
    Producer producer;
    producer.fd = memfd_create("pokelinux-cpu-surface-content", MFD_CLOEXEC);
    setup(producer.fd >= 0, "memfd_create");
    const int descriptorFlags = fcntl(producer.fd, F_GETFD);
    setup(descriptorFlags >= 0 && (descriptorFlags & FD_CLOEXEC) != 0, "memfd_CLOEXEC");
    setup(ftruncate(producer.fd, static_cast<off_t>(BYTES)) == 0, "producer_ftruncate");
    void* mapping = mmap(nullptr, BYTES, PROT_READ | PROT_WRITE, MAP_SHARED, producer.fd, 0);
    setup(mapping != MAP_FAILED, "producer_mmap");
    producer.pixels = static_cast<uint8_t*>(mapping);
    std::memcpy(producer.pixels, result.expected.data(), BYTES);
    PrShmDescription input{};
    input.drm_format = DRM_FORMAT_ARGB8888;
    input.width = WIDTH;
    input.height = HEIGHT;
    input.stride = STRIDE;
    input.bytes_per_pixel = 4;
    setup(Render::CSHMSnapshot::capture(input, producer.pixels, BYTES, result.snapshot) == PR_OK, "real_Rust_capture");
    const auto view = result.snapshot.view();
    setup(view && view->pixels && view->pixels != producer.pixels && view->pixel_bytes == BYTES, "real_owned_snapshot_view");
    result.description = view->description;
    setup(result.description.resource_id && result.description.generation && result.description.commit_sequence &&
          result.description.drm_format == input.drm_format && result.description.width == WIDTH &&
          result.description.height == HEIGHT && result.description.stride == STRIDE &&
          result.description.bytes_per_pixel == 4 && result.description.reserved == 0, "real_snapshot_descriptor");
    snapshotMatches(result.snapshot, result.description, result.expected, "capture_exact_96_bytes");
    std::memset(producer.pixels, 0xEE, BYTES);
    snapshotMatches(result.snapshot, result.description, result.expected, "snapshot_independent_of_overwrite");
    producer.dispose();
    snapshotMatches(result.snapshot, result.description, result.expected, "snapshot_survives_unmap_and_close");
    return result;
}
void install(SSurfaceState& current, Captured& captured) {
    SSurfaceState pending;
    pending.updated.bits.buffer = true;
    pending.updated.bits.scale = true;
    pending.updated.bits.transform = true;
    pending.updated.bits.viewport = true;
    pending.shmSnapshot = std::move(captured.snapshot);
    pending.size = {static_cast<double>(WIDTH), static_cast<double>(HEIGHT)};
    pending.bufferSize = {static_cast<double>(WIDTH), static_cast<double>(HEIGHT)};
    pending.scale = 1;
    pending.transform = WL_OUTPUT_TRANSFORM_NORMAL;
    current.updateFrom(pending);
    pending.reset();
    require(!pending.shmSnapshot && !current.buffer, "CPU_only_install_and_pending_reset");
    snapshotMatches(current.shmSnapshot, captured.description, captured.expected, "installed_snapshot_retained");
}
} // namespace

int main() {
    (void)std::setvbuf(stdout, nullptr, _IONBF, 0);
    try {
        SSurfaceState current;
        require(!current.shmSnapshot && !current.buffer, "initial_empty_state");
        geometry(current, {}, "empty_zero");
        pass("empty_zero");

        auto first = captureAndDispose(0x23);
        install(current, first);
        std::printf("CPU_CONTENT_PREREQUISITES_PASS actual_Rust_snapshot=true client_buffer_absent=true producer_overwritten_unmapped_closed=true width=5 height=3 stride=32 retained_bytes=96\n");
        geometry(current, {static_cast<double>(WIDTH), static_cast<double>(HEIGHT)}, "retained_shm_geometry");
        pass("retained_shm_geometry_and_producer_disposal");

        SSurfaceState noAttach;
        noAttach.updated.bits.damage = true;
        noAttach.damage = CBox{1, 0, 2, 1};
        current.updateFrom(noAttach);
        noAttach.reset();
        require(!current.updated.bits.buffer && !noAttach.shmSnapshot, "no_attach_does_not_replace_content");
        snapshotMatches(current.shmSnapshot, first.description, first.expected, "no_attach_identity_and_96_bytes");
        geometry(current, {static_cast<double>(WIDTH), static_cast<double>(HEIGHT)}, "no_attach_geometry");
        pass("no_attach_update_and_pending_reset");

        SSurfaceState geometryUpdate;
        geometryUpdate.updated.bits.scale = true;
        geometryUpdate.updated.bits.transform = true;
        geometryUpdate.updated.bits.viewport = true;
        geometryUpdate.scale = 2;
        geometryUpdate.transform = WL_OUTPUT_TRANSFORM_90;
        geometryUpdate.viewport.hasDestination = true;
        geometryUpdate.viewport.destination = {11, 7};
        current.updateFrom(geometryUpdate);
        geometryUpdate.reset();
        geometry(current, {1.5, 2.5}, "rotation_scale_not_destination");
        current.viewport.hasSource = true;
        current.viewport.source = CBox{0.5, 0.25, 3.0, 2.0};
        geometry(current, {3, 2}, "viewport_source_precedes_rotation_scale");
        current.viewport.hasSource = false;
        current.transform = WL_OUTPUT_TRANSFORM_NORMAL;
        geometry(current, {2.5, 1.5}, "normal_scale");
        snapshotMatches(current.shmSnapshot, first.description, first.expected, "geometry_does_not_change_content");
        pass("viewport_rotation_scale_geometry");

        Render::CSHMSnapshot firstWitness = current.shmSnapshot;
        SSurfaceState detach;
        detach.updated.bits.buffer = true;
        current.updateFrom(detach);
        detach.reset();
        require(!current.shmSnapshot && !current.buffer && current.bufferSize == Vector2D{}, "attach_null_clears_content");
        geometry(current, {}, "attach_null_geometry");
        snapshotMatches(firstWitness, first.description, first.expected, "independent_old_handle_survives_detach");
        auto second = captureAndDispose(0x91);
        require(second.expected != first.expected && second.description.resource_id > first.description.resource_id &&
                second.description.generation > first.description.generation && second.description.commit_sequence > first.description.commit_sequence,
                "fresh_import_has_distinct_monotonic_identity");
        install(current, second);
        geometry(current, {static_cast<double>(WIDTH), static_cast<double>(HEIGHT)}, "fresh_attachment_geometry");
        snapshotMatches(current.shmSnapshot, second.description, second.expected, "fresh_attachment_exact_96_bytes");
        snapshotMatches(firstWitness, first.description, first.expected, "old_handle_unchanged_after_replacement");
        current.reset();
        require(!current.shmSnapshot && !current.updated.all, "current_reset_releases_content");
        geometry(current, {}, "reset_geometry_despite_stale_bufferSize");
        snapshotMatches(firstWitness, first.description, first.expected, "old_clone_survives_current_reset");
        firstWitness.reset();
        require(!firstWitness, "last_explicit_clone_released");
        pass("attach_null_fresh_import_and_current_reset");
        std::printf("CPU_CONTENT_ALL_PASS groups=5\n");
        return 0;
    } catch (const Failure& error) {
        std::printf("CPU_CONTENT_%s %s\n", error.code == 77 ? "SETUP_UNAVAILABLE" : "FAIL", error.name);
        return error.code;
    } catch (const std::exception& error) {
        std::printf("CPU_CONTENT_SETUP_UNAVAILABLE exception=%s\n", error.what());
        return 77;
    } catch (...) {
        std::printf("CPU_CONTENT_SETUP_UNAVAILABLE nonstandard_exception\n");
        return 77;
    }
}
