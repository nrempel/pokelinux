// Header/real-SDK unavailable-lifetime component only: no actor or GUI.
#include "pokelinux_settings_connectivity.h"
#include <cassert>
#include <cerrno>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <dirent.h>
#include <limits>
#include <sys/stat.h>
#include <time.h>
#include <type_traits>
#include <unistd.h>

using Client = PrSettingsConnectivityClient;
using View = PrSettingsConnectivityView;
static_assert(std::is_standard_layout_v<View> && std::is_trivially_copyable_v<View>);
static_assert(sizeof(View) == 40 && alignof(View) == 8);
static_assert(offsetof(View, observed_at_ns) == 0 && offsetof(View, until_ns) == 8);
static_assert(offsetof(View, transport_connected) == 16 && offsetof(View, available) == 20);
static_assert(offsetof(View, pending) == 24 && offsetof(View, can_control) == 28);
static_assert(offsetof(View, status) == 32 && offsetof(View, configured) == 36);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_deadline_v1), uint64_t (*)()>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_new_v1), int32_t (*)(const uint8_t *, size_t, Client **)>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_view_v1), int32_t (*)(Client *, View *)>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_set_v1), int32_t (*)(Client *, uint32_t, uint64_t)>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_stop_v1), int32_t (*)(Client *)>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_join_v1), int32_t (*)(Client *)>);
static_assert(std::is_same_v<decltype(&pr_settings_connectivity_free_v1), int32_t (*)(Client *)>);

static uint64_t now() {
    timespec t{};
    assert(clock_gettime(CLOCK_BOOTTIME, &t) == 0 && t.tv_sec >= 0);
    return uint64_t(t.tv_sec) * 1000000000ULL + uint64_t(t.tv_nsec);
}
static size_t task_count() {
    DIR *directory = opendir("/proc/self/task");
    assert(directory);
    size_t count = 0;
    errno = 0;
    while (const auto *entry = readdir(directory)) {
        if (entry->d_name[0] != '.') ++count;
    }
    assert(errno == 0 && closedir(directory) == 0);
    return count;
}
static void unavailable(const View &v, uint32_t status) {
    assert(v.transport_connected <= 1 && v.available <= 1 && v.pending <= 1 && v.can_control <= 1);
    assert(v.configured >= -1 && v.configured <= 1);
    assert(!v.transport_connected && !v.available && !v.pending && !v.can_control);
    assert(v.configured == -1 && v.observed_at_ns == 0 && v.until_ns == 0 && v.status == status);
}
int main(int argc, char **argv) {
    assert(argc == 2 && getuid() == 11050 && geteuid() == 11050 && getgid() == 11050);
    assert(task_count() == 1);
    const auto *path = reinterpret_cast<const uint8_t *>(argv[1]);
    const size_t length = std::strlen(argv[1]);
    assert(length > 0 && length <= 107 && argv[1][0] == '/');
    struct stat missing{};
    assert(lstat(argv[1], &missing) == -1 && errno == ENOENT);
    const uint64_t before = now();
    const uint64_t original_until = pr_settings_connectivity_deadline_v1();
    const uint64_t after = now();
    assert(before <= after && after <= std::numeric_limits<uint64_t>::max() - 1000000000ULL);
    assert(original_until >= before + 1000000000ULL && original_until <= after + 1000000000ULL);

    Client *client = nullptr;
    assert(pr_settings_connectivity_new_v1(nullptr, length, &client) == PR_SETTINGS_INVALID && client == nullptr);
    assert(pr_settings_connectivity_new_v1(path, 0, &client) == PR_SETTINGS_INVALID && client == nullptr);
    assert(pr_settings_connectivity_new_v1(path, length, nullptr) == PR_SETTINGS_INVALID);
    const uint8_t relative[] = {'r', 'e', 'l'};
    assert(pr_settings_connectivity_new_v1(relative, sizeof(relative), &client) == PR_SETTINGS_INVALID && client == nullptr);
    View output;
    std::memset(&output, 0xa5, sizeof(output));
    unsigned char saved[sizeof(View)];
    std::memcpy(saved, &output, sizeof(output));
    assert(pr_settings_connectivity_view_v1(nullptr, &output) == PR_SETTINGS_INVALID);
    assert(std::memcmp(saved, &output, sizeof(output)) == 0);
    assert(pr_settings_connectivity_stop_v1(nullptr) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_join_v1(nullptr) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_free_v1(nullptr) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_new_v1(path, length, &client) == PR_SETTINGS_OK && client);
    Client *occupied = client;
    assert(pr_settings_connectivity_new_v1(path, length, &occupied) == PR_SETTINGS_INVALID && occupied == client);
    assert(pr_settings_connectivity_view_v1(client, nullptr) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_view_v1(client, &output) == PR_SETTINGS_OK);
    unavailable(output, PR_SETTINGS_UNAVAILABLE);
    assert(pr_settings_connectivity_set_v1(nullptr, 0, original_until) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_set_v1(client, 2, original_until) == PR_SETTINGS_INVALID);
    assert(pr_settings_connectivity_set_v1(client, UINT32_MAX, original_until) == PR_SETTINGS_INVALID);
    assert(now() < original_until);
    assert(pr_settings_connectivity_set_v1(client, 0, original_until) == PR_SETTINGS_REFUSED);
    assert(pr_settings_connectivity_set_v1(client, 1, original_until) == PR_SETTINGS_REFUSED);
    assert(pr_settings_connectivity_view_v1(client, &output) == PR_SETTINGS_OK);
    unavailable(output, PR_SETTINGS_UNAVAILABLE);
    assert(pr_settings_connectivity_free_v1(client) == PR_SETTINGS_REFUSED);

    const uint64_t stop_started = now();
    const uint64_t stop_until = stop_started + 200000000ULL;
    assert(pr_settings_connectivity_stop_v1(client) == PR_SETTINGS_OK);
    for (;;) {
        assert(now() < stop_until);
        const int32_t result = pr_settings_connectivity_join_v1(client);
        assert(now() < stop_until);
        if (result == PR_SETTINGS_OK) break;
        assert(result == PR_SETTINGS_WAIT);
        const timespec pause{0, 1000000};
        assert(nanosleep(&pause, nullptr) == 0 || errno == EINTR);
    }
    assert(pr_settings_connectivity_view_v1(client, &output) == PR_SETTINGS_OK);
    unavailable(output, PR_SETTINGS_STOPPED);
    assert(task_count() == 1 && now() < stop_until);
    assert(pr_settings_connectivity_free_v1(client) == PR_SETTINGS_OK);
    client = nullptr;
    const uint64_t completed = now();
    assert(completed < stop_until);
    std::printf("PASS C++ header/unavailable/actual_join tasks=1 original_stop_ns=%llu completed_ns=%llu\n",
                static_cast<unsigned long long>(stop_until), static_cast<unsigned long long>(completed));
}
