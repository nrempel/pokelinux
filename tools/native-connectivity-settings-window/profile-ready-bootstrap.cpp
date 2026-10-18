// Private generic-ARM display ordering fixture; no Source FD, facts or policy.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/plugins/PluginSystem.hpp>
#include <hyprland/src/event/EventBus.hpp>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include <cerrno>
#include <cstdint>
#include <stdexcept>
#include <string>

namespace {
constexpr const char* PHYSICAL = "/usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter.so";
constexpr const char* APPLIED = "/run/user/1000/pixel10-profile/applied.json";
constexpr const char* ORDERED = "/run/user/1000/pixel10-profile/ordered-load.json";
CHyprSignalListener readyListener;
bool attempted = false, completed = false, loaded = false;
uint64_t boottime() {
    timespec t{};
    if (clock_gettime(CLOCK_BOOTTIME, &t) != 0) throw std::runtime_error("profile fixture clock unavailable");
    return uint64_t(t.tv_sec) * 1'000'000'000ULL + uint64_t(t.tv_nsec);
}
void loadAtReady() {
    if (attempted) return;
    // Set before loadPlugin: its original implementation may queue config reload.
    attempted = true;
    const uint64_t began = boottime();
    if (!g_pPluginSystem || g_pPluginSystem->getPluginByPath(PHYSICAL))
        throw std::runtime_error("physical profile must have exactly one new fixed-path load");
    g_pPluginSystem->loadPlugin(PHYSICAL, SPECIAL_PID_TYPE_CONFIG)->then([](SP<CPromiseResult<CPlugin*>> result) {
        completed = true;
        loaded = !result->hasError() && result->result() && result->result()->m_path == PHYSICAL;
    });
    // Trusted config permission must complete synchronously before entering loop.
    // A pending/failed load is a setup failure, never an ordering success.
    if (!completed || !loaded) throw std::runtime_error("fixed physical profile load did not complete at ready");
    const int applied = open(APPLIED, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    struct stat st{};
    const bool qualified = applied >= 0 && fstat(applied, &st) == 0 && S_ISREG(st.st_mode)
        && st.st_uid == 1000 && st.st_size > 0 && st.st_size <= 65536;
    if (applied >= 0) close(applied);
    if (!qualified) throw std::runtime_error("physical profile actual application receipt absent");
    const uint64_t finished = boottime();
    const std::string body = "{\"schema\":1,\"private_fixture_only\":true,\"pid\":" + std::to_string(getpid())
        + ",\"uid\":" + std::to_string(getuid()) + ",\"attempted_before_load\":true,\"ready_callback_boottime_ns\":"
        + std::to_string(began) + ",\"physical_load_completed_boottime_ns\":" + std::to_string(finished)
        + ",\"fixed_plugin\":\"" + PHYSICAL + "\",\"actual_applied_receipt\":\"" + APPLIED
        + "\",\"actual_applied_receipt_bytes\":" + std::to_string(st.st_size)
        + ",\"synchronous_original_config_loader\":true,\"Source_FD_or_facts_accessed\":false}\n";
    const int fd = open(ORDERED, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (fd < 0) throw std::runtime_error("new profile ordering receipt unavailable");
    size_t offset = 0;
    while (offset < body.size()) {
        const ssize_t n = write(fd, body.data() + offset, body.size() - offset);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) { close(fd); throw std::runtime_error("profile ordering receipt write failed"); }
        offset += size_t(n);
    }
    if (close(fd) != 0) throw std::runtime_error("profile ordering receipt close failed");
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE) {
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash())
        throw std::runtime_error("exact installed compositor API mismatch");
    readyListener = Event::bus()->m_events.ready.listen(loadAtReady);
    return {"pokelinux-private-profile-at-ready", "One fixed physical profile load before actual loop entry", "Pokelinux fixture", "1"};
}
APICALL EXPORT void PLUGIN_EXIT() { readyListener.reset(); }
