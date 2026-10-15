// Independent public DataV3 NDK peer. This is private test infrastructure.
#include <aidl/android/hardware/radio/data/BnRadioData.h>
#include <aidl/android/hardware/radio/data/IRadioDataResponse.h>
#include <aidl/android/hardware/radio/data/IRadioDataIndication.h>
#include <aidl/org/pokelinux/datatest/BnDataFixture.h>
#include <aidl/org/pokelinux/datatest/CaptureSetup.h>
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <algorithm>
#include <cerrno>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <dlfcn.h>
#include <limits>
#include <mutex>
#include <optional>
#include <string>
#include <time.h>
#include <unistd.h>
#include <vector>

using namespace aidl::android::hardware::radio;
using namespace aidl::android::hardware::radio::data;
using aidl::org::pokelinux::datatest::BnDataFixture;
using aidl::org::pokelinux::datatest::CaptureSetup;
namespace {
constexpr int32_t kMaxCallbacks = 4;
constexpr int64_t kBarrierMaxNs = 3000000000LL;
int64_t bootNow() {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0) _exit(71);
    return static_cast<int64_t>(ts.tv_sec) * 1000000000LL + ts.tv_nsec;
}
void increment(int32_t& value, int32_t cap = 2) { if (value < cap) ++value; }
ndk::ScopedAStatus reject() {
    return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_STATE);
}
bool text(const std::string& s, size_t maximum) {
    return s.size() <= maximum && s.find('\0') == std::string::npos;
}
bool strings(const std::vector<std::string>& values) {
    if (values.size() > 4) return false;
    for (const auto& value : values) if (!text(value, 128)) return false;
    return true;
}
bool addresses(const std::vector<LinkAddress>& values) {
    if (values.size() > 4) return false;
    for (const auto& value : values) if (!text(value.address, 128)) return false;
    return true;
}
bool descriptor(const TrafficDescriptor& value) {
    return (!value.dnn || text(*value.dnn, 128))
        && (!value.osAppId || value.osAppId->osAppId.size() <= 256);
}
bool profile(const DataProfileInfo& value) {
    return text(value.apn, 128) && text(value.user, 128) && text(value.password, 128)
        && descriptor(value.trafficDescriptor);
}
bool resultBounded(const SetupDataCallResult& value) {
    // First fixture subset: no QoS sessions; retain all fields of accepted results.
    // The generated decoder runs before these limits: no hostile-decode bound claim.
    if (!text(value.ifname, 15) || !addresses(value.addresses) || !strings(value.dnses)
        || !strings(value.gateways) || !strings(value.pcscf)
        || value.defaultQos.getTag() != Qos::Tag::noinit
        || !value.qosSessions.empty() || value.trafficDescriptors.size() > 1) return false;
    for (const auto& valueDescriptor : value.trafficDescriptors)
        if (!descriptor(valueDescriptor)) return false;
    return true;
}
class Endpoint final : public BnRadioData {
public:
    const int64_t originalUntil;
    std::mutex lock;
    std::condition_variable changed;
    CaptureSetup saved;
    std::shared_ptr<IRadioDataResponse> response;
    std::shared_ptr<IRadioDataIndication> indication;
    bool callbackInFlight = false;
    explicit Endpoint(int64_t until) : originalUntil(until) {}
    ndk::ScopedAStatus refusedLocked() {
        increment(saved.rejectedCalls); changed.notify_all(); return reject();
    }
    ndk::ScopedAStatus forbidden() {
        std::lock_guard<std::mutex> guard(lock);
        increment(saved.forbiddenCalls); changed.notify_all();
        return ndk::ScopedAStatus::fromExceptionCode(EX_UNSUPPORTED_OPERATION);
    }
    ndk::ScopedAStatus setResponseFunctions(const std::shared_ptr<IRadioDataResponse>& res,
            const std::shared_ptr<IRadioDataIndication>& ind) override {
        std::lock_guard<std::mutex> guard(lock);
        increment(saved.attachmentAttempts);
        saved.attachmentCallingPid = AIBinder_getCallingPid();
        saved.attachmentCallingUid = static_cast<int32_t>(AIBinder_getCallingUid());
        if (bootNow() >= originalUntil || saved.attachmentAttempts != 1 || !res || !ind
            || !AIBinder_isRemote(res->asBinder().get())
            || !AIBinder_isRemote(ind->asBinder().get())) return refusedLocked();
        response = res; indication = ind; saved.attachmentsRetained = 1;
        std::printf("DATA-ATTACH retained=1 attempts=1 calling_pid=%d calling_uid=%d\n",
                    saved.attachmentCallingPid, saved.attachmentCallingUid);
        std::fflush(stdout); changed.notify_all(); return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus setupDataCall(int32_t serial, AccessNetwork accessNetwork,
            const DataProfileInfo& dataProfileInfo, bool roamingAllowed,
            DataRequestReason reason, const std::vector<LinkAddress>& linkAddresses,
            const std::vector<std::string>& dnses, int32_t pduSessionId,
            const std::optional<SliceInfo>& sliceInfo, bool matchAllRuleAllowed) override {
        std::lock_guard<std::mutex> guard(lock);
        increment(saved.setupAttempts);
        if (bootNow() >= originalUntil || saved.attachmentsRetained != 1
            || saved.attachmentAttempts != 1 || saved.setupAttempts != 1
            || !profile(dataProfileInfo) || !addresses(linkAddresses) || !strings(dnses))
            return refusedLocked();
        saved.setupCallingPid = AIBinder_getCallingPid();
        saved.setupCallingUid = static_cast<int32_t>(AIBinder_getCallingUid());
        saved.serial = serial; saved.accessNetwork = accessNetwork;
        saved.dataProfileInfo = dataProfileInfo; saved.roamingAllowed = roamingAllowed;
        saved.reason = reason; saved.addresses = linkAddresses; saved.dnses = dnses;
        saved.pduSessionId = pduSessionId; saved.sliceInfo = sliceInfo;
        saved.matchAllRuleAllowed = matchAllRuleAllowed; saved.setupPresent = true;
        std::printf("DATA-SETUP attempts=1 serial=%d calling_pid=%d calling_uid=%d\n",
                    serial, saved.setupCallingPid, saved.setupCallingUid);
        std::fflush(stdout); changed.notify_all(); return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus allocatePduSessionId(int32_t) override { return forbidden(); }
    ndk::ScopedAStatus cancelHandover(int32_t, int32_t) override { return forbidden(); }
    ndk::ScopedAStatus deactivateDataCall(int32_t, int32_t, DataRequestReason) override { return forbidden(); }
    ndk::ScopedAStatus getDataCallList(int32_t) override { return forbidden(); }
    ndk::ScopedAStatus getSlicingConfig(int32_t) override { return forbidden(); }
    ndk::ScopedAStatus releasePduSessionId(int32_t, int32_t) override { return forbidden(); }
    ndk::ScopedAStatus responseAcknowledgement() override { return forbidden(); }
    ndk::ScopedAStatus setDataAllowed(int32_t, bool) override { return forbidden(); }
    ndk::ScopedAStatus setDataProfile(int32_t, const std::vector<DataProfileInfo>&) override { return forbidden(); }
    ndk::ScopedAStatus setDataThrottling(int32_t, DataThrottlingAction, int64_t) override { return forbidden(); }
    ndk::ScopedAStatus setInitialAttachApn(int32_t, const std::optional<DataProfileInfo>&) override { return forbidden(); }
    ndk::ScopedAStatus startHandover(int32_t, int32_t) override { return forbidden(); }
    ndk::ScopedAStatus startKeepalive(int32_t, const KeepaliveRequest&) override { return forbidden(); }
    ndk::ScopedAStatus stopKeepalive(int32_t, int32_t) override { return forbidden(); }
};
class Fixture final : public BnDataFixture {
    const std::string incarnation;
    const std::shared_ptr<Endpoint> data;
public:
    Fixture(std::string generation, int64_t until)
        : incarnation(std::move(generation)), data(ndk::SharedRefBase::make<Endpoint>(until)) {}
    ndk::ScopedAStatus endpoint(std::shared_ptr<IRadioData>* out) override {
        *out = data; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus generation(std::string* out) override {
        *out = incarnation; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus capture(CaptureSetup* out) override {
        std::lock_guard<std::mutex> guard(data->lock);
        *out = data->saved; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus barrier(int32_t attachments, int32_t setups, int32_t responses,
            int32_t indications, int64_t until) override {
        int64_t now = bootNow();
        if (attachments < 0 || attachments > 1 || setups < 0 || setups > 1
            || responses < 0 || responses > kMaxCallbacks
            || indications < 0 || indications > kMaxCallbacks
            || until <= now || until > data->originalUntil || until - now > kBarrierMaxNs)
            return ndk::ScopedAStatus::fromExceptionCode(EX_ILLEGAL_ARGUMENT);
        std::unique_lock<std::mutex> guard(data->lock);
        for (;;) {
            now = bootNow();
            if (now >= until) return ndk::ScopedAStatus::fromServiceSpecificError(20);
            const auto& s = data->saved;
            if (s.rejectedCalls || s.forbiddenCalls || s.attachmentAttempts > 1
                || s.setupAttempts > 1 || s.responseAttempts > responses
                || s.indicationAttempts > indications) return reject();
            if (s.attachmentsRetained == attachments && s.setupAttempts == setups
                && s.responseSubmissions == responses && s.indicationSubmissions == indications)
                return ndk::ScopedAStatus::ok();
            now = bootNow();
            if (now >= until) return ndk::ScopedAStatus::fromServiceSpecificError(20);
            data->changed.wait_for(guard, std::chrono::nanoseconds(std::min<int64_t>(until-now, 5000000)));
        }
    }
    ndk::ScopedAStatus setupResponse(const RadioResponseInfo& info,
            const SetupDataCallResult& result) override {
        std::shared_ptr<IRadioDataResponse> callback;
        {
            std::lock_guard<std::mutex> guard(data->lock);
            increment(data->saved.responseAttempts, kMaxCallbacks + 1);
            if (bootNow() >= data->originalUntil || info.type != RadioResponseType::SOLICITED
                || !data->saved.setupPresent || !data->response || !resultBounded(result)
                || data->saved.responseAttempts > kMaxCallbacks
                || data->callbackInFlight) return data->refusedLocked();
            // Deliberately no serial rewrite: fixture can submit a wrong-serial control.
            data->saved.lastResponseInfo = info; data->saved.lastResponse = result;
            data->callbackInFlight = true; callback = data->response;
        }
        auto status = callback->setupDataCallResponse(info, result);
        {
            std::lock_guard<std::mutex> guard(data->lock);
            data->callbackInFlight = false;
            if (status.isOk()) increment(data->saved.responseSubmissions, kMaxCallbacks + 1);
            else increment(data->saved.rejectedCalls);
            data->changed.notify_all();
        }
        std::printf("DATA-RESPONSE serial=%d type=%d submission_ok=%d\n", info.serial,
                    static_cast<int32_t>(info.type), status.isOk());
        std::fflush(stdout); return status;
    }
    ndk::ScopedAStatus listChanged(const std::vector<SetupDataCallResult>& calls) override {
        std::shared_ptr<IRadioDataIndication> callback;
        {
            std::lock_guard<std::mutex> guard(data->lock);
            increment(data->saved.indicationAttempts, kMaxCallbacks + 1);
            if (bootNow() >= data->originalUntil || calls.size() > 1
                || !data->saved.setupPresent || !data->indication
                || data->saved.indicationAttempts > kMaxCallbacks
                || data->callbackInFlight) return data->refusedLocked();
            for (const auto& call : calls) if (!resultBounded(call)) return data->refusedLocked();
            data->saved.lastDataCallList = calls;
            data->callbackInFlight = true; callback = data->indication;
        }
        auto status = callback->dataCallListChanged(RadioIndicationType::UNSOLICITED, calls);
        {
            std::lock_guard<std::mutex> guard(data->lock);
            data->callbackInFlight = false;
            if (status.isOk()) increment(data->saved.indicationSubmissions, kMaxCallbacks + 1);
            else increment(data->saved.rejectedCalls);
            data->changed.notify_all();
        }
        std::printf("DATA-LIST count=%zu type=0 submission_ok=%d\n", calls.size(), status.isOk());
        std::fflush(stdout); return status;
    }
    ndk::ScopedAStatus die() override { _exit(0); }
};
} // namespace
int main(int argc, char** argv) {
    if (argc != 3 || std::string(argv[1]).size() != 32) return 2;
    const std::string generation = argv[1];
    for (char c : generation)
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) return 2;
    for (const char* c = argv[2]; *c; ++c) if (*c < '0' || *c > '9') return 2;
    errno = 0; char* end = nullptr; const long long untilValue = std::strtoll(argv[2], &end, 10);
    const int64_t now = bootNow();
    if (errno || !end || *end || untilValue <= now || untilValue-now > 120000000000LL) return 2;
    Dl_info library{};
    if (!dladdr(reinterpret_cast<void*>(&AIBinder_Class_define), &library)) return 3;
    std::printf("GENUINE-NDK-LOADED %s\n", library.dli_fname); std::fflush(stdout);
    auto setThreads = reinterpret_cast<decltype(&ABinderProcess_setThreadPoolMaxThreadCount)>(
        dlsym(RTLD_DEFAULT, "ABinderProcess_setThreadPoolMaxThreadCount"));
    auto startPool = reinterpret_cast<decltype(&ABinderProcess_startThreadPool)>(
        dlsym(RTLD_DEFAULT, "ABinderProcess_startThreadPool"));
    auto joinPool = reinterpret_cast<decltype(&ABinderProcess_joinThreadPool)>(
        dlsym(RTLD_DEFAULT, "ABinderProcess_joinThreadPool"));
    auto addService = reinterpret_cast<decltype(&AServiceManager_addService)>(
        dlsym(RTLD_DEFAULT, "AServiceManager_addService"));
    if (!setThreads || !startPool || !joinPool || !addService || !setThreads(4)) return 3;
    startPool();
    auto fixture = ndk::SharedRefBase::make<Fixture>(generation, static_cast<int64_t>(untilValue));
    if (addService(fixture->asBinder().get(), "org.pokelinux.fixture.cellular-data") != EX_NONE) return 4;
    std::printf("READY-DATA generation=%s pid=%d until_boottime_ns=%lld\n", generation.c_str(),
                getpid(), untilValue); std::fflush(stdout);
    joinPool(); return 0; // Parent retains exact process custody; no detached cleanup owner.
}
