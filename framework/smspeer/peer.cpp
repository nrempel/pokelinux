#include <aidl/android/hardware/radio/messaging/BnRadioMessaging.h>
#include <aidl/android/hardware/radio/messaging/IRadioMessagingResponse.h>
#include <aidl/android/hardware/radio/messaging/IRadioMessagingIndication.h>
#include <aidl/org/pokelinux/smstest/BnSmsBoundaryProbe.h>
#include <android/binder_ibinder_jni.h>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <signal.h>
#include <unistd.h>
using namespace aidl::android::hardware::radio;
using namespace aidl::android::hardware::radio::messaging;
using aidl::org::pokelinux::smstest::BnSmsBoundaryProbe;
// Independent synthetic endpoint; no framework radio lookup, modem or vendor target.
class Endpoint final : public BnRadioMessaging {
    ndk::ScopedAStatus responseAcknowledgement() override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
public:
    std::mutex lock;
    std::condition_variable changed;
    std::shared_ptr<IRadioMessagingResponse> response, previous;
    std::shared_ptr<IRadioMessagingIndication> indication;
    int32_t registrations = 0, sends = 0, serial = 0;
    std::string pdu, smsc;
    ndk::ScopedAStatus setResponseFunctions(const std::shared_ptr<IRadioMessagingResponse>& res,
            const std::shared_ptr<IRadioMessagingIndication>& ind) override {
        if (!res || !ind) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(lock);
        previous = response; response = res; indication = ind; registrations++; changed.notify_all();
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus sendSms(int32_t value, const GsmSmsMessage& message) override {
        if (message.pdu.empty() || message.pdu.size() > 314) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        for (char c : message.pdu) if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')))
            return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(lock);
        serial = value; pdu = message.pdu; smsc = message.smscPdu; sends++; changed.notify_all();
        // Completion is released only by the separate synthetic control interface.
        return ndk::ScopedAStatus::ok();
    }
::ndk::ScopedAStatus acknowledgeIncomingGsmSmsWithPdu(int32_t , bool , const std::string& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus acknowledgeLastIncomingCdmaSms(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsAck& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus acknowledgeLastIncomingGsmSms(int32_t , bool , ::aidl::android::hardware::radio::messaging::SmsAcknowledgeFailCause ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus deleteSmsOnRuim(int32_t , int32_t ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus deleteSmsOnSim(int32_t , int32_t ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus getCdmaBroadcastConfig(int32_t ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus getGsmBroadcastConfig(int32_t ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus getSmscAddress(int32_t ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus reportSmsMemoryStatus(int32_t , bool ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus sendCdmaSms(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsMessage& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus sendCdmaSmsExpectMore(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsMessage& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus sendImsSms(int32_t , const ::aidl::android::hardware::radio::messaging::ImsSmsMessage& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus sendSmsExpectMore(int32_t , const ::aidl::android::hardware::radio::messaging::GsmSmsMessage& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus setCdmaBroadcastActivation(int32_t , bool ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus setCdmaBroadcastConfig(int32_t , const std::vector<::aidl::android::hardware::radio::messaging::CdmaBroadcastSmsConfigInfo>& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus setGsmBroadcastActivation(int32_t , bool ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus setGsmBroadcastConfig(int32_t , const std::vector<::aidl::android::hardware::radio::messaging::GsmBroadcastSmsConfigInfo>& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus setSmscAddress(int32_t , const std::string& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus writeSmsToRuim(int32_t , const ::aidl::android::hardware::radio::messaging::CdmaSmsWriteArgs& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
::ndk::ScopedAStatus writeSmsToSim(int32_t , const ::aidl::android::hardware::radio::messaging::SmsWriteArgs& ) override { return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION); }
};
class Peer final : public BnSmsBoundaryProbe {
    std::shared_ptr<Endpoint> slots[2] = {ndk::SharedRefBase::make<Endpoint>(), ndk::SharedRefBase::make<Endpoint>()};
    std::shared_ptr<Endpoint> slot(int32_t value) { return value >= 0 && value < 2 ? slots[value] : nullptr; }
public:
    ndk::ScopedAStatus endpoint(int32_t value, std::shared_ptr<IRadioMessaging>* out) override {
        *out = slot(value); return *out ? ndk::ScopedAStatus::ok() : ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
    }
    ndk::ScopedAStatus barrier(int32_t value, int32_t registrations, int32_t sends) override {
        auto e = slot(value); if (!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::unique_lock<std::mutex> guard(e->lock);
        if (!e->changed.wait_for(guard, std::chrono::seconds(8), [&]{return e->registrations >= registrations && e->sends >= sends;}))
            return ndk::ScopedAStatus::fromServiceSpecificError(20);
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus acknowledgeRequest(int32_t value, int32_t serial) override {
        auto e = slot(value); if (!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::shared_ptr<IRadioMessagingResponse> callback;
        {std::lock_guard<std::mutex> guard(e->lock); callback = e->response;}
        return callback ? callback->acknowledgeRequest(serial) : ndk::ScopedAStatus::fromServiceSpecificError(21);
    }
    ndk::ScopedAStatus finalResponse(int32_t value, int32_t serial, int32_t mode, bool previous) override {
        auto e = slot(value); if (!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::shared_ptr<IRadioMessagingResponse> callback;
        {std::lock_guard<std::mutex> guard(e->lock); callback = previous ? e->previous : e->response;}
        if (!callback) return ndk::ScopedAStatus::fromServiceSpecificError(21);
        RadioResponseInfo info; info.serial = serial;
        info.type = mode == 3 ? RadioResponseType::SOLICITED_ACK : RadioResponseType::SOLICITED;
        info.error = mode == 2 ? RadioError::MODEM_ERR : RadioError::NONE;
        SendSmsResult result; result.messageRef = mode == 4 ? 256 : 7; result.ackPDU = mode == 5 ? "f" : ""; result.errorCode = -1;
        return mode == 1 ? callback->sendImsSmsResponse(info, result) : callback->sendSmsResponse(info, result);
    }
    ndk::ScopedAStatus sendCount(int32_t value, int32_t* out) override {
        auto e=slot(value); if(!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(e->lock); *out=e->sends; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus capturedSerial(int32_t value, int32_t* out) override {
        auto e=slot(value); if(!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(e->lock); *out=e->serial; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus capturedPdu(int32_t value, std::string* out) override {
        auto e=slot(value); if(!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(e->lock); *out=e->pdu; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus capturedSmsc(int32_t value, std::string* out) override {
        auto e=slot(value); if(!e) return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::lock_guard<std::mutex> guard(e->lock); *out=e->smsc; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus die() override { kill(getpid(), SIGKILL); return ndk::ScopedAStatus::ok(); }
};
extern "C" JNIEXPORT jobject JNICALL Java_org_pokelinux_smspeer_Peer_createEndpoint(JNIEnv* env, jclass) {
    static auto peer = ndk::SharedRefBase::make<Peer>();
    return AIBinder_toJavaBinder(env, peer->asBinder().get());
}
