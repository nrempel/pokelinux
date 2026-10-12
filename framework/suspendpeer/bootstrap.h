#pragma once
// Independent NDK proxy test through framework bootstrap and real kernel Binder.
#include <aidl/org/pokelinux/suspendtest/ISuspendBoundaryProbe.h>
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
using aidl::org::pokelinux::suspendtest::ISuspendBoundaryProbe;
static void require(bool ok, const char* message) {
    if (!ok) { fprintf(stderr, "FAIL: %s\n", message); exit(1); }
}
static void check(binder_status_t status) { require(status == STATUS_OK, "Binder parcel operation"); }
static void ok(const ndk::ScopedAStatus& status) { static int calls=0; ++calls; if(!status.isOk()) fprintf(stderr,"RPC[%d] %s\n",calls,status.getDescription().c_str()); require(status.isOk(), "expected successful remote status"); }
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
    bool valid = package == "org.pokelinux.suspendpeer" && component == "org.pokelinux.suspendpeer.Peer"
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
    check(ndk::AParcel_writeData(in.get(), std::string("org.pokelinux.suspendpeer")));
    check(ndk::AParcel_writeData(in.get(), std::string("org.pokelinux.suspendpeer.Peer")));
    check(AParcel_writeString(in.get(), nullptr, -1));
    // Installed-framework FlagOracle verifies this exact Android17 no-restart flag.
    check(AParcel_writeInt32(in.get(), 8));
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
