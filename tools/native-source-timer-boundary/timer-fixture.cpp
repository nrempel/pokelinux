// Authored native Source producer using the actual production packet publisher.
#include "source_sampler.hpp"
#include <wayland-server-core.h>
#include <sys/un.h>
#include <poll.h>
#include <cerrno>
#include <memory>
#include <string>
#include <stdexcept>
namespace {
int output = -1;
uint64_t sequence = 0, callbacks = 0;
std::string incarnation;
bool locked = false;
pokelinux::source::Sampler sampler;
wl_event_source* timer = nullptr;
void publish() {
    uint64_t sampled = 0;
    if (!pokelinux::source::boottime(sampled) || sequence == UINT64_MAX) {
        pokelinux::source::closeOutput(output); return;
    }
    sampler.publish(output, sequence, incarnation, sampled,
        {locked, false, "null", "{\"state\":\"unavailable\"}", "null", "null", -1});
}
int tick(void*) {
    sampler.timerDispatched();
    ++callbacks;
    publish();
    if (output >= 0 && wl_event_source_timer_update(timer, 25) != 0)
        pokelinux::source::closeOutput(output);
    return 0;
}
void controlSend(int control, const std::string& text) {
    if (send(control, text.data(), text.size(), MSG_NOSIGNAL) != static_cast<ssize_t>(text.size()))
        throw std::runtime_error("control send");
}
}
int main(int argc, char** argv) {
    if (argc != 3 || std::string(argv[1]) != "--control"
        || getuid() != 1000 || geteuid() != 1000 || getgid() != 1000 || getegid() != 1000 || getgroups(0, nullptr) != 0)
        return 64;
    output = pokelinux::source::provisionedFd(incarnation);
    const int control = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0);
    if (control < 0) return 65;
    sockaddr_un address{}; address.sun_family = AF_UNIX;
    if (std::strlen(argv[2]) >= sizeof(address.sun_path)) return 66;
    std::strcpy(address.sun_path, argv[2]);
    if (connect(control, reinterpret_cast<sockaddr*>(&address), sizeof(address)) != 0) return 67;
    ucred parent{}; socklen_t length = sizeof(parent);
    if (getsockopt(control, SOL_SOCKET, SO_PEERCRED, &parent, &length) != 0
        || length != sizeof(parent) || parent.uid != 0 || parent.gid != 0 || parent.pid != getppid()) return 68;
    auto* loop = wl_event_loop_create(); if (!loop) return 69;
    timer = wl_event_loop_add_timer(loop, tick, nullptr);
    if (!timer || wl_event_source_timer_update(timer, 25) != 0) return 70;
    // The same two calls model actual init and an early state-observing hook.
    publish();
    locked = true;
    publish();
    controlSend(control, "ready");
    for (;;) {
        char command[32]{}; const auto count = recv(control, command, sizeof(command), 0);
        if (count <= 0) break;
        if (std::string(command, static_cast<size_t>(count)) != "dispatch") return 71;
        const auto before = callbacks;
        while (callbacks == before) {
            if (wl_event_loop_dispatch(loop, 1000) != 0) return 72;
        }
        controlSend(control, "tick:" + std::to_string(callbacks));
    }
    wl_event_source_remove(timer); wl_event_loop_destroy(loop);
    pokelinux::source::closeOutput(output); close(control); return 0;
}
