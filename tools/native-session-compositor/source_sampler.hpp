#pragma once
// Shared production packet publisher; no permissions or receiver lease policy.
#include <sys/socket.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <charconv>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>
namespace pokelinux::source {
inline constexpr size_t MAX_FRAME = 1024;
inline void closeOutput(int& output) {
    if (output >= 0) { close(output); output = -1; }
}
inline bool boottime(uint64_t& ns) {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0 || ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L)
        return false;
    const auto sec = static_cast<uint64_t>(ts.tv_sec);
    if (sec > (std::numeric_limits<uint64_t>::max() - static_cast<uint64_t>(ts.tv_nsec)) / 1000000000ULL)
        return false;
    ns = sec * 1000000000ULL + static_cast<uint64_t>(ts.tv_nsec);
    return true;
}
inline int provisionedFd(std::string& incarnation) {
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
struct Facts {
    bool locked, complete;
    const char* focus;
    const char* locker;
    const char* generation;
    const char* unlock;
    int peer_fd;
};
class Sampler {
    bool timer_dispatched_ = false;
  public:
    // The actual event-loop timer callback alone activates this incarnation.
    void timerDispatched() { timer_dispatched_ = true; }
    bool canPublish() const { return timer_dispatched_; }
    void publish(int& output, uint64_t& sequence, const std::string& incarnation,
                 uint64_t sampled, const Facts& facts) {
        if (output < 0 || !canPublish()) return;
        char frame[MAX_FRAME];
        const int n = snprintf(frame, sizeof(frame),
            "{\"source\":\"%s\",\"sequence\":%llu,\"sampled_at_ns\":%llu,\"locked\":%s,\"lock_completed\":%s,\"focus\":%s,\"locker\":%s,\"lock_generation\":%s,\"protocol_unlock\":%s}",
            incarnation.c_str(), static_cast<unsigned long long>(++sequence), static_cast<unsigned long long>(sampled),
            facts.locked ? "true" : "false", facts.complete ? "true" : "false", facts.focus,
            facts.locker, facts.generation, facts.unlock);
        if (n <= 0 || static_cast<size_t>(n) >= sizeof(frame)) { closeOutput(output); return; }
        // One nonblocking atomic packet attempt. No queue, retries, detached worker
        // or renewed incarnation on failure; receiver expiration fences old facts.
        iovec bytes{frame, static_cast<size_t>(n)};
        msghdr packet{};
        packet.msg_iov = &bytes;
        packet.msg_iovlen = 1;
        alignas(cmsghdr) char control[CMSG_SPACE(sizeof(int))]{};
        if (facts.peer_fd >= 0) {
            packet.msg_control = control;
            packet.msg_controllen = sizeof(control);
            auto* rights = CMSG_FIRSTHDR(&packet);
            rights->cmsg_level = SOL_SOCKET;
            rights->cmsg_type = SCM_RIGHTS;
            rights->cmsg_len = CMSG_LEN(sizeof(int));
            std::memcpy(CMSG_DATA(rights), &facts.peer_fd, sizeof(facts.peer_fd));
        }
        // The selected NativePeer owns the exact socket-derived handle until this atomic send.
        // Its temporary copy closes after success and every failure; the receiver
        // owns the SCM_RIGHTS reference. No pidfs identity or PID reopening is used.
        if (sendmsg(output, &packet, MSG_DONTWAIT | MSG_NOSIGNAL) != n) closeOutput(output);
    }
};
}
