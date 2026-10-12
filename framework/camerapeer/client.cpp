// Independent NDK proxy test through framework bootstrap and real kernel Binder.
#include <aidl/org/pokelinux/cameratest/ICameraBoundaryProbe.h>
#include <dlfcn.h>
#include <android/binder_parcel_utils.h>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <dirent.h>
#include <fcntl.h>
#include <mutex>
#include <optional>
#include <poll.h>
#include <unistd.h>
using aidl::android::hardware::common::NativeHandle;
using aidl::org::pokelinux::cameratest::ICameraBoundaryProbe;
static void require(bool ok, const char* message) {
    if (!ok) { fprintf(stderr, "FAIL: %s\n", message); exit(1); }
}
static void check(binder_status_t status) { require(status == STATUS_OK, "Binder parcel operation"); }
static void ok(const ndk::ScopedAStatus& status) { require(status.isOk(), "expected successful remote status"); }
struct Watch {
    std::mutex mutex;
    std::condition_variable condition;
    ndk::SpAIBinder peer;
    binder_status_t error = STATUS_OK;
};
static void* create(void* args) { return args; }
static void destroy(void*) {}
static binder_status_t watch_status(AIBinder* binder, transaction_code_t code, const AParcel* in, AParcel* out) {
    auto* state = static_cast<Watch*>(AIBinder_getUserData(binder));
    if (code != 1) return STATUS_UNKNOWN_TRANSACTION;
    int32_t present, status, length, magic, entries, type, trailing;
    std::string package, component, key;
    ndk::SpAIBinder peer;
    #define READ(value) do { auto result = ndk::AParcel_readData(in, &value); if (result != STATUS_OK) return result; } while (0)
    READ(present); READ(package); READ(component); READ(status); READ(present);
    READ(length); READ(magic);
    auto begin = AParcel_getDataPosition(in);
    READ(entries); READ(key); READ(type); READ(peer); READ(trailing);
    #undef READ
    bool valid = package == "org.pokelinux.camerapeer" && component == "org.pokelinux.camerapeer.Peer"
        && status == 37 && present == 1 && magic == 0x4c444e42 && entries == 1 && key == "peer"
        && type == 15 && trailing == 0 && AParcel_getDataPosition(in) - begin == length + 4
        && AParcel_getDataPosition(in) == AParcel_getDataSize(in) && peer.get() && AIBinder_isRemote(peer.get());
    {
        std::lock_guard<std::mutex> guard(state->mutex);
        state->error = valid ? STATUS_OK : STATUS_BAD_VALUE;
        state->peer = peer;
    }
    state->condition.notify_one();
    ndk::ScopedAStatus result = ndk::ScopedAStatus::ok();
    return AParcel_writeStatusHeader(out, result.get());
}
static binder_status_t unused_transaction(AIBinder*, transaction_code_t, const AParcel*, AParcel*) { return STATUS_UNKNOWN_TRANSACTION; }
static ndk::SpAIBinder bootstrap() {
    // Platform discovery/thread APIs are outside NDK app stubs. Resolve only
    // from the emulator public libbinder_ndk; Java production is not retained.
    auto set_threads = reinterpret_cast<bool(*)(uint32_t)>(dlsym(RTLD_DEFAULT, "ABinderProcess_setThreadPoolMaxThreadCount"));
    auto start_pool = reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT, "ABinderProcess_startThreadPool"));
    auto service = reinterpret_cast<AIBinder*(*)(const char*)>(dlsym(RTLD_DEFAULT, "AServiceManager_checkService"));
    require(set_threads && start_pool && service, "public platform bootstrap symbols");
    require(set_threads(2), "thread pool configuration"); start_pool();
    Watch state;
    auto* klass = AIBinder_Class_define("android.app.IInstrumentationWatcher", create, destroy, watch_status);
    ndk::SpAIBinder watcher(AIBinder_new(klass, &state));
    ndk::SpAIBinder activity(service("activity"));
    require(activity.get(), "activity service exists");
    auto* activity_class = AIBinder_Class_define("android.app.IActivityManager", create, destroy, unused_transaction);
    require(AIBinder_associateClass(activity.get(), activity_class), "activity descriptor");
    ndk::ScopedAParcel in, out;
    check(AIBinder_prepareTransaction(activity.get(), in.getR()));
    check(AParcel_writeInt32(in.get(), 1));
    check(ndk::AParcel_writeData(in.get(), std::string("org.pokelinux.camerapeer")));
    check(ndk::AParcel_writeData(in.get(), std::string("org.pokelinux.camerapeer.Peer")));
    check(AParcel_writeString(in.get(), nullptr, -1));
    check(AParcel_writeInt32(in.get(), 0));
    check(AParcel_writeInt32(in.get(), 0));
    check(AParcel_writeStrongBinder(in.get(), watcher.get()));
    check(AParcel_writeStrongBinder(in.get(), nullptr));
    check(AParcel_writeInt32(in.get(), 0));
    check(AParcel_writeString(in.get(), nullptr, -1));
    // Qualified against FrameworkBootstrapOracle in the SDK37 harness.
    check(AIBinder_transact(activity.get(), 51, in.getR(), out.getR(), 0));
    ndk::ScopedAStatus status;
    check(AParcel_readStatusHeader(out.get(), status.getR())); ok(status);
    int32_t started = 0; check(AParcel_readInt32(out.get(), &started));
    require(started == 1, "instrumentation started");
    std::unique_lock<std::mutex> lock(state.mutex);
    require(state.condition.wait_for(lock, std::chrono::seconds(5), [&state] { return state.peer.get(); }), "bootstrap timeout");
    require(state.error == STATUS_OK, "bootstrap parcel shape");
    return state.peer;
}
static NativeHandle handle_for(int fd) {
    NativeHandle handle;
    handle.fds.emplace_back(fd);
    handle.ints = {37, 4};
    return handle;
}
static int fd_count() {
    DIR* directory = opendir("/proc/self/fd"); require(directory, "fd inventory");
    int count = 0; while (auto* e = readdir(directory)) if (e->d_name[0] != '.') ++count;
    closedir(directory); return count;
}
static void read_byte(int fd, char expected) {
    pollfd item{fd, POLLIN | POLLHUP, 0};
    require(poll(&item, 1, 2000) == 1, "bounded pipe receipt");
    char byte; require(read(fd, &byte, 1) == 1 && byte == expected, "pipe marker");
}
static void eof(int fd) {
    pollfd item{fd, POLLIN | POLLHUP, 0};
    require(poll(&item, 1, 2000) == 1, "bounded pipe EOF");
    char byte; require(read(fd, &byte, 1) == 0, "all writers independently closed");
}
// The generated proxy leaves output untouched on error. Ignore it until status is successful.
static std::optional<int32_t> acceptance(const ndk::ScopedAStatus& status, int32_t count) {
    return status.isOk() ? std::optional<int32_t>(count) : std::nullopt;
}
struct Death {
    std::mutex mutex;
    std::condition_variable condition;
    bool observed = false;
};
static void died(void* cookie) {
    auto* death = static_cast<Death*>(cookie);
    { std::lock_guard<std::mutex> guard(death->mutex); death->observed = true; }
    death->condition.notify_one();
}
int main() {
    auto binder = bootstrap();
    auto peer = ICameraBoundaryProbe::fromBinder(binder);
    require(peer && peer->isRemote(), "generated remote NDK proxy");
    puts("camera-ndk-bootstrap\tPASS");
    int32_t peer_baseline; ok(peer->fdCount(&peer_baseline));
    int client_baseline = fd_count();
    for (int mode : {0, 1}) {
        int fds[2]; require(pipe2(fds, O_CLOEXEC) == 0, "pipe");
        ndk::ScopedFileDescriptor reader(fds[0]);
        auto input = handle_for(fds[1]);
        int32_t count = -77;
        auto status = peer->submit(input, mode, &count);
        if (mode == 0) require(status.isOk() && count == 1, "successful count1 receipt");
        else require(status.getExceptionCode() == EX_SERVICE_SPECIFIC && status.getServiceSpecificError() == 17 && count == -77,
                     "generated error omits count written locally by service");
        input.fds.clear(); // Original sender fd closes before receiver uses its duplicate.
        NativeHandle returned; ok(peer->returnedHandle(&returned));
        require(returned.fds.size() == 1 && returned.ints == std::vector<int32_t>{37, 4}, "NativeHandle parcel identity");
        ok(peer->release()); read_byte(reader.get(), 'R');
        // A returned Binder fd survives both source and retained receiver close.
        require(write(returned.fds[0].get(), "C", 1) == 1, "returned fd independent lifetime");
        read_byte(reader.get(), 'C'); returned.fds.clear(); eof(reader.get());
        if (mode == 0) puts("camera-ndk-fd-independent\tPASS");
        else {
            puts("camera-ndk-error-output-untouched\tPASS");
            require(!acceptance(status, count).has_value(), "service error has no valid acceptance result");
            puts("camera-ndk-error-uncertain\tPASS");
        }
    }
    for (int i = 0; i < 64; ++i) {
        int fds[2]; require(pipe2(fds, O_CLOEXEC) == 0, "repeated pipe");
        ndk::ScopedFileDescriptor reader(fds[0]); auto input = handle_for(fds[1]);
        int32_t count = -77; auto status = peer->submit(input, i % 2, &count);
        require((i % 2 ? !status.isOk() && !acceptance(status, count) : status.isOk() && acceptance(status, count) == 1), "repeat receipt classification");
        input.fds.clear(); ok(peer->release()); read_byte(reader.get(), 'R'); eof(reader.get());
    }
    int32_t peer_final; ok(peer->fdCount(&peer_final));
    require(peer_final == peer_baseline && fd_count() == client_baseline, "bounded repeated FD count");
    puts("camera-ndk-fd-budget\t64\tPASS");
    Death death; auto* recipient = AIBinder_DeathRecipient_new(died);
    check(AIBinder_linkToDeath(binder.get(), recipient, &death));
    int fds[2]; require(pipe2(fds, O_CLOEXEC) == 0, "death pipe");
    ndk::ScopedFileDescriptor reader(fds[0]); auto input = handle_for(fds[1]);
    int32_t count = -77; auto status = peer->submit(input, 2, &count);
    require(status.getStatus() == STATUS_DEAD_OBJECT && count == -77 && !acceptance(status, count), "death before reply has no receipt");
    input.fds.clear(); read_byte(reader.get(), 'D'); eof(reader.get());
    { std::unique_lock<std::mutex> lock(death.mutex);
      require(death.condition.wait_for(lock, std::chrono::seconds(5), [&death] { return death.observed; }), "bounded death notification"); }
    require(!AIBinder_isAlive(binder.get()) && peer->fdCount(&count).getStatus() == STATUS_DEAD_OBJECT, "dead peer refuses further work");
    AIBinder_DeathRecipient_delete(recipient);
    puts("camera-ndk-service-death\tPASS");
    return 0;
}
