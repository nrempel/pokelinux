// Independent receive-only synthetic endpoint using frozen generated NDK messaging V3.
// Each slot permits one registration attempt for this process generation. No reset/retry.
#include <aidl/android/hardware/radio/messaging/BnRadioMessaging.h>
#include <aidl/android/hardware/radio/messaging/IRadioMessagingResponse.h>
#include <aidl/android/hardware/radio/messaging/IRadioMessagingIndication.h>
#include <aidl/org/pokelinux/messagesincomingtest/BnIncomingMessagesProbe.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <dlfcn.h>
#include <mutex>
#include <string>
#include <unistd.h>
using namespace aidl::android::hardware::radio;
using namespace aidl::android::hardware::radio::messaging;
using aidl::org::pokelinux::messagesincomingtest::BnIncomingMessagesProbe;
class Endpoint final : public BnRadioMessaging {
    bool hold;
public:
    std::mutex lock;
    std::condition_variable changed;
    std::shared_ptr<IRadioMessagingResponse> response;
    std::shared_ptr<IRadioMessagingIndication> indication;
    // attempted registrations, completed registrations, submitted callbacks,
    // forbidden ACK attempts, forbidden sends, response-ACK attempts, other calls.
    int32_t values[7] = {};
    explicit Endpoint(bool value): hold(value) {}
    ndk::ScopedAStatus forbidden(int index) {
        std::lock_guard<std::mutex> guard(lock); values[index]++; changed.notify_all();
        return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION);
    }
    ndk::ScopedAStatus setResponseFunctions(const std::shared_ptr<IRadioMessagingResponse>& res,
            const std::shared_ptr<IRadioMessagingIndication>& ind) override {
        std::unique_lock<std::mutex> guard(lock);
        values[0]++; changed.notify_all();
        if (values[0] != 1 || !res || !ind || !AIBinder_isRemote(res->asBinder().get())
                || !AIBinder_isRemote(ind->asBinder().get()))
            return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_STATE);
        if (hold) changed.wait(guard, [] { return false; }); // Deliberately no release/reset.
        guard.unlock();
        int32_t resVersion=0, indVersion=0; std::string resHash, indHash;
        if (!res->getInterfaceVersion(&resVersion).isOk() || !ind->getInterfaceVersion(&indVersion).isOk()
            || !res->getInterfaceHash(&resHash).isOk() || !ind->getInterfaceHash(&indHash).isOk()
            || resVersion != 3 || indVersion != 3
            || resHash != "30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb"
            || indHash != resHash) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        guard.lock(); response=res; indication=ind; values[1]++; changed.notify_all();
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus responseAcknowledgement() override { return forbidden(5); }
    ndk::ScopedAStatus acknowledgeLastIncomingGsmSms(int32_t value, bool ok, SmsAcknowledgeFailCause fail) override { return forbidden(3); }
    ndk::ScopedAStatus sendSms(int32_t, const GsmSmsMessage&) override { return forbidden(4); }
    ndk::ScopedAStatus acknowledgeIncomingGsmSmsWithPdu(int32_t , bool , const std::string& ) override { return forbidden(3); }
    ndk::ScopedAStatus acknowledgeLastIncomingCdmaSms(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsAck& ) override { return forbidden(3); }
    ndk::ScopedAStatus deleteSmsOnRuim(int32_t , int32_t ) override { return forbidden(6); }
    ndk::ScopedAStatus deleteSmsOnSim(int32_t , int32_t ) override { return forbidden(6); }
    ndk::ScopedAStatus getCdmaBroadcastConfig(int32_t ) override { return forbidden(6); }
    ndk::ScopedAStatus getGsmBroadcastConfig(int32_t ) override { return forbidden(6); }
    ndk::ScopedAStatus getSmscAddress(int32_t ) override { return forbidden(6); }
    ndk::ScopedAStatus reportSmsMemoryStatus(int32_t , bool ) override { return forbidden(6); }
    ndk::ScopedAStatus sendCdmaSms(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsMessage& ) override { return forbidden(4); }
    ndk::ScopedAStatus sendCdmaSmsExpectMore(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsMessage& ) override { return forbidden(4); }
    ndk::ScopedAStatus sendImsSms(int32_t , const ::aidl::android::hardware::radio::messaging::ImsSmsMessage& ) override { return forbidden(4); }
    ndk::ScopedAStatus sendSmsExpectMore(int32_t , const ::aidl::android::hardware::radio::messaging::GsmSmsMessage& ) override { return forbidden(4); }
    ndk::ScopedAStatus setCdmaBroadcastActivation(int32_t , bool ) override { return forbidden(6); }
    ndk::ScopedAStatus setCdmaBroadcastConfig(int32_t , const std::vector<::aidl::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo>& ) override { return forbidden(6); }
    ndk::ScopedAStatus setGsmBroadcastActivation(int32_t , bool ) override { return forbidden(6); }
    ndk::ScopedAStatus setGsmBroadcastConfig(int32_t , const std::vector<::aidl::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo>& ) override { return forbidden(6); }
    ndk::ScopedAStatus setSmscAddress(int32_t , const std::string& ) override { return forbidden(6); }
    ndk::ScopedAStatus writeSmsToRuim(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsWriteArgs& ) override { return forbidden(6); }
    ndk::ScopedAStatus writeSmsToSim(int32_t , const ::aidl::android::hardware::radio::messaging::SmsWriteArgs& ) override { return forbidden(6); }
};
class Peer final : public BnIncomingMessagesProbe {
    std::string incarnation;
    std::shared_ptr<Endpoint> slots[2];
    std::shared_ptr<Endpoint> slot(int32_t value) { return value >= 0 && value < 2 ? slots[value] : nullptr; }
public:
    Peer(std::string value, bool hold): incarnation(std::move(value)),
        slots{ndk::SharedRefBase::make<Endpoint>(hold),ndk::SharedRefBase::make<Endpoint>(hold)} {}
    ndk::ScopedAStatus endpoint(int32_t value, std::shared_ptr<IRadioMessaging>* out) override {
        *out=slot(value); return *out ? ndk::ScopedAStatus::ok() : ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
    }
    ndk::ScopedAStatus generation(std::string* out) override { *out=incarnation; return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus barrier(int32_t value, int32_t registrations, int32_t callbacks, int32_t timeoutMillis) override {
        auto e=slot(value); if (!e || registrations < 0 || registrations > 1 || callbacks < 0
                || callbacks > 8192 || timeoutMillis < 1 || timeoutMillis > 1000)
            return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::unique_lock<std::mutex> guard(e->lock);
        if (!e->changed.wait_for(guard,std::chrono::milliseconds(timeoutMillis),
                [&] { return e->values[1] >= registrations && e->values[2] >= callbacks; }))
            return ndk::ScopedAStatus::fromServiceSpecificError(20);
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus incoming(int32_t value, int32_t kind, const std::vector<uint8_t>& pdu, int32_t type) override {
        auto e=slot(value); if (!e || (kind != 0 && kind != 1) || pdu.size() > 513 || type < 0 || type > 1)
            return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::shared_ptr<IRadioMessagingIndication> callback;
        { std::lock_guard<std::mutex> guard(e->lock); callback=e->indication; }
        if (!callback) return ndk::ScopedAStatus::fromServiceSpecificError(21);
        auto result=kind == 0 ? callback->newSms(static_cast<RadioIndicationType>(type),pdu)
                             : callback->newSmsStatusReport(static_cast<RadioIndicationType>(type),pdu);
        { std::lock_guard<std::mutex> guard(e->lock); if (result.isOk()) e->values[2]++; e->changed.notify_all(); }
        return result; // One-way return is IPC progress, never durable admission.
    }
    ndk::ScopedAStatus counters(int32_t value, std::vector<int32_t>* out) override {
        auto e=slot(value); if (!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(e->lock); out->assign(e->values,e->values+7); return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus die() override { _exit(0); }
};
int main(int argc,char** argv) {
    if (argc < 2 || argc > 3 || std::string(argv[1]).size() != 32) return 2;
    std::string generation=argv[1]; for (char c:generation) if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) return 2;
    bool hold=argc == 3 && std::string(argv[2]) == "hold-register";
    if (argc == 3 && !hold) return 2;
    Dl_info library={}; if (!dladdr(reinterpret_cast<void*>(&AIBinder_Class_define),&library)) return 3;
    printf("GENUINE-NDK-LOADED %s\n",library.dli_fname); fflush(stdout);
    auto setThreads=reinterpret_cast<decltype(&ABinderProcess_setThreadPoolMaxThreadCount)>(dlsym(RTLD_DEFAULT,"ABinderProcess_setThreadPoolMaxThreadCount"));
    auto startPool=reinterpret_cast<decltype(&ABinderProcess_startThreadPool)>(dlsym(RTLD_DEFAULT,"ABinderProcess_startThreadPool"));
    auto joinPool=reinterpret_cast<decltype(&ABinderProcess_joinThreadPool)>(dlsym(RTLD_DEFAULT,"ABinderProcess_joinThreadPool"));
    auto addService=reinterpret_cast<decltype(&AServiceManager_addService)>(dlsym(RTLD_DEFAULT,"AServiceManager_addService"));
    if (!setThreads || !startPool || !joinPool || !addService || !setThreads(4)) return 3;
    startPool();
    auto peer=ndk::SharedRefBase::make<Peer>(generation,hold);
    auto status=addService(peer->asBinder().get(),"org.pokelinux.fixture.incoming-messages");
    if (status != EX_NONE) { fprintf(stderr,"INCOMING ADD FAILED %d\n",status); return 4; }
    printf("READY-INCOMING generation=%s pid=%d\n",generation.c_str(),getpid()); fflush(stdout);
    joinPool(); return 0;
}
