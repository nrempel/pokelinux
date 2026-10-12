// Synthetic discovery/transfer peer. Frozen suspend proxy code is generated unchanged.
#include <aidl/org/pokelinux/suspendtest/BnSuspendBoundaryProbe.h>
#include <android/binder_ibinder_jni.h>
#include <jni.h>
#include <aidl/android/system/suspend/BnSystemSuspend.h>
#include <aidl/android/system/suspend/BnWakeLock.h>
#include <atomic>
#include <mutex>
using namespace aidl::android::system::suspend;
using aidl::org::pokelinux::suspendtest::BnSuspendBoundaryProbe;
using aidl::org::pokelinux::suspendtest::ISuspendControl;
// A synthetic native ledger observes real generated V1 object destruction.
// This counter is not a kernel wake lock or a SystemSuspend implementation.
struct Ledger { std::atomic<int64_t> active{0},acquired{0},released{0},uid{0},pid{0}; };
class NdkWake final : public BnWakeLock {
    std::shared_ptr<Ledger> ledger;std::atomic<bool> done{false};
public:
    explicit NdkWake(std::shared_ptr<Ledger> value):ledger(std::move(value)){ledger->active++;ledger->acquired++;}
    ~NdkWake() override { release(); }
    ndk::ScopedAStatus release() override { if(!done.exchange(true)){ledger->active--;ledger->released++;}return ndk::ScopedAStatus::ok(); }
};
class NdkSuspend final : public BnSystemSuspend {
    std::shared_ptr<Ledger> ledger;
public:
    explicit NdkSuspend(std::shared_ptr<Ledger> value):ledger(std::move(value)){}
    ndk::ScopedAStatus acquireWakeLock(WakeLockType,const std::string&,std::shared_ptr<IWakeLock>* out) override {
        ledger->uid=AIBinder_getCallingUid();ledger->pid=AIBinder_getCallingPid();*out=ndk::SharedRefBase::make<NdkWake>(ledger);return ndk::ScopedAStatus::ok();
    }
};
class Peer final : public BnSuspendBoundaryProbe {
    std::mutex mutex;
    std::shared_ptr<Ledger> ledger=std::make_shared<Ledger>();
    std::shared_ptr<ISystemSuspend> ndk=ndk::SharedRefBase::make<NdkSuspend>(ledger);
    std::shared_ptr<ISystemSuspend> current;
    std::shared_ptr<ISuspendControl> observer;
    std::shared_ptr<IWakeLock> held;
public:
    ndk::ScopedAStatus ndkService(std::shared_ptr<ISystemSuspend>* out) override { *out=ndk;return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus ndkSnapshot(std::vector<int64_t>* out) override { *out={ledger->active.load(),ledger->acquired.load(),ledger->released.load(),ledger->uid.load(),ledger->pid.load()};return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus install(const std::shared_ptr<ISystemSuspend>& service,const std::shared_ptr<ISuspendControl>& control) override {
        if(!service || !service->isRemote() || !control || !control->isRemote()) return ndk::ScopedAStatus::fromServiceSpecificError(20);
        std::lock_guard<std::mutex> g(mutex); current=service; observer=control; held.reset(); return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus service(std::shared_ptr<ISystemSuspend>* out) override { std::lock_guard<std::mutex> g(mutex); *out=current; return current?ndk::ScopedAStatus::ok():ndk::ScopedAStatus::fromServiceSpecificError(21); }
    ndk::ScopedAStatus control(std::shared_ptr<ISuspendControl>* out) override { std::lock_guard<std::mutex> g(mutex); *out=observer; return observer?ndk::ScopedAStatus::ok():ndk::ScopedAStatus::fromServiceSpecificError(21); }
    ndk::ScopedAStatus retain(const std::shared_ptr<IWakeLock>& wake) override { std::lock_guard<std::mutex> g(mutex); if(held || !wake || !wake->isRemote())return ndk::ScopedAStatus::fromServiceSpecificError(22); held=wake;return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus borrow(std::shared_ptr<IWakeLock>* out) override { std::lock_guard<std::mutex> g(mutex);*out=held;return held?ndk::ScopedAStatus::ok():ndk::ScopedAStatus::fromServiceSpecificError(23); }
    ndk::ScopedAStatus dropRetained() override { std::lock_guard<std::mutex> g(mutex); held.reset();return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus foreignAcquire(int32_t* out) override { std::shared_ptr<ISystemSuspend> target;{std::lock_guard<std::mutex> g(mutex);target=current;}if(!target)return ndk::ScopedAStatus::fromServiceSpecificError(21);std::shared_ptr<IWakeLock> wake; auto status=target->acquireWakeLock(WakeLockType::PARTIAL,"foreign-app",&wake);*out=status.isOk()?0:status.getExceptionCode();return ndk::ScopedAStatus::ok(); }
};
extern "C" JNIEXPORT jobject JNICALL Java_org_pokelinux_suspendpeer_Peer_createEndpoint(JNIEnv* env,jclass) {
    static auto peer=ndk::SharedRefBase::make<Peer>(); return AIBinder_toJavaBinder(env,peer->asBinder().get());
}
