#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "fixture-pattern.h"
#include <wayland-client.h>
#include "xdg-shell-client.h"
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <array>
#include <fcntl.h>
#include <poll.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

namespace {
wl_display* display = nullptr;
wl_registry* registry = nullptr;
wl_compositor* compositor = nullptr;
wl_shm* shm = nullptr;
xdg_wm_base* wm = nullptr;
wl_surface* surface = nullptr;
xdg_surface* xdg = nullptr;
xdg_toplevel* top = nullptr;
uint64_t until = 0;
unsigned sequence = 0, configures = 0, submitted = 0;
bool failed = false, done = false, ready = false, configured = false, argb = false;
std::array<bool, 7> synced{};
struct Image {
    wl_shm_pool* pool = nullptr;
    wl_buffer* buffer = nullptr;
    uint8_t* mapping = nullptr;
    int fd = -1;
    wl_callback* frame = nullptr;
    uint32_t frameId = 0;
    bool presented = false, released = false, disposed = false;
};
Image images[2];
uint64_t now() {
    timespec t{};
    if (clock_gettime(CLOCK_BOOTTIME, &t) || t.tv_sec < 0)
        _exit(77);
    return static_cast<uint64_t>(t.tv_sec) * 1000000000ULL + static_cast<uint64_t>(t.tv_nsec);
}
void event(const char* name, unsigned value = 0) {
    if (sequence >= 64) {
        failed = true;
        return;
    }
    std::printf("{\"event\":\"%s\",\"sequence\":%u,\"boottime_ns\":%llu,\"pid\":%d,\"uid\":%u,\"value\":%u,\"surface_id\":%u}\n",
                name, ++sequence, static_cast<unsigned long long>(now()), getpid(), getuid(), value,
                surface ? wl_proxy_get_id(reinterpret_cast<wl_proxy*>(surface)) : 0);
}
void fail(const char* name) {
    if (!failed)
        event(name);
    failed = true;
}
void released(void* data, wl_buffer*) {
    auto& image = *static_cast<Image*>(data);
    if (image.released) {
        fail("SETUP_DUPLICATE_BUFFER_RELEASE");
        return;
    }
    image.released = true;
    event("BUFFER_RELEASE", &image == &images[0] ? 1 : 2);
}
const wl_buffer_listener bufferListener{released};
void requestSync(unsigned phase);
void syncDone(void* data, wl_callback* callback, uint32_t) {
    const auto phase = static_cast<unsigned>(reinterpret_cast<uintptr_t>(data));
    wl_callback_destroy(callback);
    if (phase >= synced.size() || synced[phase]) {
        fail("SETUP_SYNC_PHASE");
        return;
    }
    synced[phase] = true;
    if (phase == 0) {
        event("REGISTRY_SYNC_COMPLETE", (compositor ? 1U : 0U) | (shm ? 2U : 0U) | (wm ? 4U : 0U));
        if (!compositor || !shm || !wm) {
            fail("SETUP_NORMAL_GLOBALS_MISSING");
            return;
        }
        requestSync(5); // binds precede this real second server dispatch sync
    } else if (phase == 5) {
        event("GLOBALS_BOUND_SYNC_COMPLETE", argb ? 1U : 0U);
        if (!argb) {
            fail("SETUP_NORMAL_ARGB8888_MISSING");
            return;
        }
        ready = true;
        event("REGISTRY_READY");
    } else {
        event("REQUESTS_SYNCED", phase);
    }
}
const wl_callback_listener syncListener{syncDone};
void requestSync(unsigned phase) {
    wl_callback* callback = wl_display_sync(display);
    if (!callback || wl_callback_add_listener(callback, &syncListener, reinterpret_cast<void*>(static_cast<uintptr_t>(phase))) < 0)
        fail("SETUP_SYNC_CALLBACK");
}
void ping(void*, xdg_wm_base* base, uint32_t serial) { xdg_wm_base_pong(base, serial); }
const xdg_wm_base_listener wmListener{ping};
void shmFormat(void*, wl_shm*, uint32_t format) { argb = argb || format == WL_SHM_FORMAT_ARGB8888; }
const wl_shm_listener shmListener{shmFormat};
void global(void*, wl_registry* reg, uint32_t name, const char* iface, uint32_t version) {
    if (std::strcmp(iface, "wl_compositor") == 0 && version >= 4 && !compositor)
        compositor = static_cast<wl_compositor*>(wl_registry_bind(reg, name, &wl_compositor_interface, 4));
    else if (std::strcmp(iface, "wl_shm") == 0 && !shm) {
        shm = static_cast<wl_shm*>(wl_registry_bind(reg, name, &wl_shm_interface, 1));
        if (shm)
            wl_shm_add_listener(shm, &shmListener, nullptr);
    } else if (std::strcmp(iface, xdg_wm_base_interface.name) == 0 && !wm) {
        wm = static_cast<xdg_wm_base*>(wl_registry_bind(reg, name, &xdg_wm_base_interface, 1));
        if (wm)
            xdg_wm_base_add_listener(wm, &wmListener, nullptr);
    }
}
void removed(void*, wl_registry*, uint32_t) { fail("SETUP_GLOBAL_REMOVED"); }
const wl_registry_listener registryListener{global, removed};
void xdgConfigured(void*, xdg_surface* resource, uint32_t serial) {
    xdg_surface_ack_configure(resource, serial);
    if (++configures > 8) {
        fail("SETUP_CONFIGURE_BOUND");
        return;
    }
    configured = true;
    event("XDG_CONFIGURE_ACK", serial);
}
const xdg_surface_listener xdgListener{xdgConfigured};
void topConfigured(void*, xdg_toplevel*, int32_t width, int32_t height, wl_array*) {
    if (sequence >= 64) {
        failed = true;
        return;
    }
    std::printf("{\"event\":\"TOPLEVEL_CONFIGURE\",\"sequence\":%u,\"boottime_ns\":%llu,\"pid\":%d,\"width\":%d,\"height\":%d}\n",
                ++sequence, static_cast<unsigned long long>(now()), getpid(), width, height);
}
void topClosed(void*, xdg_toplevel*) { fail("SETUP_TOPLEVEL_CLOSED"); }
void createWindow() {
    if (!ready || surface) {
        fail("SETUP_WINDOW_ORDER");
        return;
    }
    surface = wl_compositor_create_surface(compositor);
    xdg = surface ? xdg_wm_base_get_xdg_surface(wm, surface) : nullptr;
    if (!xdg || xdg_surface_add_listener(xdg, &xdgListener, nullptr) < 0) {
        fail("SETUP_XDG_SURFACE");
        return;
    }
    top = xdg_surface_get_toplevel(xdg);
    static xdg_toplevel_listener listener{};
    listener.configure = topConfigured;
    listener.close = topClosed;
    if (!top || xdg_toplevel_add_listener(top, &listener, nullptr) < 0) {
        fail("SETUP_TOPLEVEL");
        return;
    }
    xdg_toplevel_set_app_id(top, "org.pokelinux.normal-client-frames");
    xdg_toplevel_set_title(top, "Two real SHM frames");
    xdg_toplevel_set_min_size(top, CAPTURE_WIDTH, CAPTURE_HEIGHT);
    xdg_toplevel_set_max_size(top, CAPTURE_WIDTH, CAPTURE_HEIGHT);
    wl_surface_commit(surface); // genuine initial empty xdg commit
    event("INITIAL_EMPTY_COMMIT");
    requestSync(6); // actual server dispatch witness, not configure or presentation
}
void frameDone(void* data, wl_callback* callback, uint32_t milliseconds) {
    auto& image = *static_cast<Image*>(data);
    if (image.presented || image.frame != callback) {
        fail("SETUP_FRAME_CALLBACK_IDENTITY");
        return;
    }
    if (sequence >= 64) {
        failed = true;
        return;
    }
    image.presented = true;
    image.frame = nullptr;
    std::printf("{\"event\":\"FRAME_CALLBACK\",\"sequence\":%u,\"boottime_ns\":%llu,\"pid\":%d,\"value\":%u,\"callback_id\":%u,\"milliseconds\":%u}\n",
                ++sequence, static_cast<unsigned long long>(now()), getpid(), &image == &images[0] ? 1U : 2U, image.frameId, milliseconds);
    wl_callback_destroy(callback);
}
const wl_callback_listener frameListener{frameDone};
void submit(unsigned index) {
    if (!configured || !argb || index != submitted || index > 1 || (index == 1 && (!synced[2] || !images[0].disposed || !images[0].presented))) {
        fail("SETUP_ATTACHMENT_ORDER");
        return;
    }
    auto& image = images[index];
    image.fd = memfd_create("pokelinux-real-shm-capture", MFD_CLOEXEC);
    const int flags = image.fd >= 0 ? fcntl(image.fd, F_GETFD) : -1;
    if (flags < 0 || !(flags & FD_CLOEXEC) || ftruncate(image.fd, CAPTURE_POOL_BYTES)) {
        fail("SETUP_MEMFD");
        return;
    }
    void* mapping = mmap(nullptr, CAPTURE_POOL_BYTES, PROT_READ | PROT_WRITE, MAP_SHARED, image.fd, 0);
    if (mapping == MAP_FAILED) {
        fail("SETUP_PRODUCER_MMAP");
        return;
    }
    image.mapping = static_cast<uint8_t*>(mapping);
    std::memset(image.mapping, 0xB7, CAPTURE_POOL_BYTES);
    for (uint32_t i = 0; i < CAPTURE_BYTES; ++i)
        image.mapping[CAPTURE_OFFSET + i] = capture_expected_byte(i, index == 0 ? 0x23 : 0x91);
    image.pool = wl_shm_create_pool(shm, image.fd, CAPTURE_POOL_BYTES);
    image.buffer = image.pool ? wl_shm_pool_create_buffer(image.pool, CAPTURE_OFFSET, CAPTURE_WIDTH, CAPTURE_HEIGHT, CAPTURE_STRIDE, WL_SHM_FORMAT_ARGB8888) : nullptr;
    if (!image.buffer || wl_buffer_add_listener(image.buffer, &bufferListener, &image) < 0) {
        fail("SETUP_REAL_SHM_RESOURCE");
        return;
    }
    image.frame = wl_surface_frame(surface);
    if (!image.frame || wl_callback_add_listener(image.frame, &frameListener, &image) < 0) {
        fail("SETUP_REAL_FRAME_CALLBACK");
        return;
    }
    image.frameId = wl_proxy_get_id(reinterpret_cast<wl_proxy*>(image.frame));
    wl_surface_attach(surface, image.buffer, 0, 0);
    wl_surface_damage_buffer(surface, 0, 0, CAPTURE_WIDTH, CAPTURE_HEIGHT);
    const auto commitBegin = now();
    wl_surface_commit(surface);
    ++submitted;
    if (sequence >= 64) {
        failed = true;
        return;
    }
    std::printf("{\"event\":\"REAL_SHM_COMMIT\",\"sequence\":%u,\"boottime_ns\":%llu,\"pid\":%d,\"index\":%u,\"surface_id\":%u,\"pool_id\":%u,\"buffer_id\":%u,\"width\":5,\"height\":3,\"stride\":32,\"offset\":64,\"bytes\":96,\"callback_id\":%u,\"commit_begin_ns\":%llu}\n",
                ++sequence, static_cast<unsigned long long>(now()), getpid(), index + 1,
                wl_proxy_get_id(reinterpret_cast<wl_proxy*>(surface)), wl_proxy_get_id(reinterpret_cast<wl_proxy*>(image.pool)),
                wl_proxy_get_id(reinterpret_cast<wl_proxy*>(image.buffer)), image.frameId, static_cast<unsigned long long>(commitBegin));
    requestSync(index == 0 ? 1 : 3);
}
void disposeProducer(unsigned index) {
    auto& image = images[index];
    if (!image.released || !image.presented || !synced[index == 0 ? 1 : 3] || image.disposed || !image.mapping || image.fd < 0) {
        fail("SETUP_DISPOSAL_BEFORE_RELEASE");
        return;
    }
    std::memset(image.mapping, 0xEE, CAPTURE_POOL_BYTES);
    if (munmap(image.mapping, CAPTURE_POOL_BYTES)) {
        fail("SETUP_PRODUCER_MUNMAP");
        return;
    }
    image.mapping = nullptr;
    const int old = image.fd;
    image.fd = -1;
    if (close(old)) {
        fail("SETUP_PRODUCER_CLOSE");
        return;
    }
    errno = 0;
    if (fcntl(old, F_GETFD) != -1 || errno != EBADF) {
        fail("SETUP_PRODUCER_FD_NOT_CLOSED");
        return;
    }
    wl_buffer_destroy(image.buffer);
    image.buffer = nullptr;
    wl_shm_pool_destroy(image.pool);
    image.pool = nullptr;
    image.disposed = true;
    event("PRODUCER_OVERWRITTEN_UNMAPPED_CLOSED_AND_RESOURCES_DESTROY_REQUESTED", index + 1);
    // The live surface keeps the same immutable content. Do not introduce an
    // extra damage/commit/FB replacement between the two selected client frames.
    requestSync(index == 0 ? 2 : 4);
}
bool flush() {
    while (now() < until) {
        if (wl_display_flush(display) >= 0)
            return true;
        if (errno != EAGAIN && errno != EINTR)
            return false;
        pollfd out{wl_display_get_fd(display), POLLOUT, 0};
        if (poll(&out, 1, 5) < 0 && errno != EINTR)
            return false;
    }
    return false;
}
} // namespace
int main(int argc, char** argv) {
    (void)std::setvbuf(stdout, nullptr, _IONBF, 0);
    if (argc != 3)
        return 64;
    char* end = nullptr;
    errno = 0;
    until = std::strtoull(argv[1], &end, 10);
    const auto start = now();
    if (errno || !end || *end || until <= start || until - start > 35000000000ULL)
        return 64;
    char* pidEnd = nullptr;
    errno = 0;
    const long expectedMain = std::strtol(argv[2], &pidEnd, 10);
    if (errno || !pidEnd || *pidEnd || expectedMain <= 0 || expectedMain > INT32_MAX)
        return 64;
    display = wl_display_connect(nullptr);
    if (!display)
        return 77;
    const int fd = wl_display_get_fd(display);
    ucred peer{};
    socklen_t peerSize = sizeof(peer);
    if (fd < 0 || getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &peer, &peerSize) || peerSize != sizeof(peer) ||
        peer.pid != expectedMain || peer.uid != 1000 || peer.gid != 1000) {
        wl_display_disconnect(display);
        return 77;
    }
    event("WAYLAND_PEER_AUTHENTICATED", static_cast<unsigned>(peer.pid));
    const int stdinFlags = fcntl(STDIN_FILENO, F_GETFL);
    if (fd < 0 || stdinFlags < 0 || fcntl(STDIN_FILENO, F_SETFL, stdinFlags | O_NONBLOCK)) {
        wl_display_disconnect(display);
        return 77;
    }
    registry = wl_display_get_registry(display);
    if (!registry || wl_registry_add_listener(registry, &registryListener, nullptr) < 0) {
        wl_display_disconnect(display);
        return 77;
    }
    requestSync(0);
    while (!failed && !done && now() < until) {
        bool prepared = false;
        while (!failed && now() < until) {
            if (wl_display_prepare_read(display) == 0) {
                prepared = true;
                break;
            }
            if (wl_display_dispatch_pending(display) < 0)
                fail("SETUP_PENDING_DISPATCH");
        }
        if (!prepared) {
            if (!failed)
                fail("SETUP_ORIGINAL_DEADLINE_EXPIRED");
            break;
        }
        if (!flush()) {
            wl_display_cancel_read(display);
            fail("SETUP_FLUSH");
            break;
        }
        pollfd fds[2]{{fd, POLLIN, 0}, {STDIN_FILENO, POLLIN, 0}};
        const int count = poll(fds, 2, 5);
        if (count > 0 && (fds[0].revents & POLLIN)) {
            if (wl_display_read_events(display) < 0 || wl_display_dispatch_pending(display) < 0)
                fail("SETUP_WAYLAND_READ_DISPATCH");
        } else {
            wl_display_cancel_read(display);
        }
        if (count < 0 && errno != EINTR)
            fail("SETUP_POLL");
        if (fds[0].revents & (POLLERR | POLLHUP | POLLNVAL))
            fail("SETUP_WAYLAND_CLOSED");
        if (fds[1].revents & POLLIN) {
            char command = 0;
            if (read(STDIN_FILENO, &command, 1) == 1) {
                if (now() >= until)
                    fail("SETUP_COMMAND_DEADLINE");
                else if (command == 'G')
                    createWindow();
                else if (command == 'A')
                    submit(0);
                else if (command == 'M')
                    disposeProducer(0);
                else if (command == 'B')
                    submit(1);
                else if (command == 'D')
                    disposeProducer(1);
                else if (command == 'Q' && synced[4] && images[1].disposed && images[1].presented)
                    done = true;
                else
                    fail("SETUP_INVALID_COMMAND_OR_ORDER");
            }
        } else if (fds[1].revents & (POLLHUP | POLLERR | POLLNVAL)) {
            fail("SETUP_CONTROL_PIPE_CLOSED");
        }
    }
    if (!done && !failed)
        fail("SETUP_ORIGINAL_DEADLINE_EXPIRED");
    wl_display_disconnect(display); // actual client-death boundary; no blocking roundtrip
    surface = nullptr; // proxies are invalid after disconnect; never inspect one again
    for (auto& image : images) {
        if (image.mapping)
            (void)munmap(image.mapping, CAPTURE_POOL_BYTES);
        if (image.fd >= 0)
            (void)close(image.fd);
    }
    event(done && !failed ? "CLIENT_DISCONNECTED_PASS" : "CLIENT_DISCONNECTED_SETUP_FAILED");
    return done && !failed ? 0 : 77;
}
