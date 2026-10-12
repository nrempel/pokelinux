// Original synthetic controls. Camera parcel and callback code is generated unmodified.
#include <aidl/org/pokelinux/cameratest/BnCameraOwnershipProbe.h>
#include <android/binder_ibinder_jni.h>
#include <condition_variable>
#include <chrono>
#include <dirent.h>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <signal.h>
#include <unistd.h>
using namespace aidl::android::hardware::camera::device;
using aidl::org::pokelinux::cameratest::BnCameraOwnershipProbe;
struct Frame { int stream; int64_t buffer; ndk::ScopedFileDescriptor data, fence_read, fence_write; };
class OwnershipPeer final : public BnCameraOwnershipProbe {
    std::mutex mutex;
    std::condition_variable condition;
    bool holding = false;
    std::shared_ptr<ICameraDeviceCallback> callback;
    std::map<std::pair<int,int64_t>,ndk::ScopedFileDescriptor> cache;
    std::map<int,Frame> frames;
    static ndk::ScopedAStatus bad(int code) { return ndk::ScopedAStatus::fromServiceSpecificError(code); }
public:
    ndk::ScopedAStatus setCallback(const std::shared_ptr<ICameraDeviceCallback>& value) override {
        if (!value || !value->isRemote()) return bad(20);
        int32_t version; std::string hash;
        if (!value->getInterfaceVersion(&version).isOk() || !value->getInterfaceHash(&hash).isOk() || version!=4 || hash!="6e6878817de92e03fd52d5f10937714862ab4922") return bad(29);
        std::lock_guard<std::mutex> guard(mutex); callback=value; return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus processCaptureRequest(const std::vector<CaptureRequest>& requests,const std::vector<BufferCache>& remove,int32_t mode,int32_t* count) override {
        std::unique_lock<std::mutex> guard(mutex);
        if (!callback || requests.size()>2 || frames.size()+requests.size()>8) return bad(20);
        for (const auto& item:remove) cache.erase({item.streamId,item.bufferId});
        for (const auto& request:requests) {
            if (request.outputBuffers.size()!=1 || request.inputBuffer.streamId!=-1 || request.fmqSettingsSize!=0 || request.settings.metadata!=std::vector<uint8_t>{1,2,3} || frames.count(request.frameNumber)) return bad(21);
            const auto& buffer=request.outputBuffers[0]; auto key=std::make_pair(buffer.streamId,buffer.bufferId);
            if (buffer.streamId!=7 || buffer.bufferId!=99 || buffer.status!=BufferStatus::OK || !buffer.acquireFence.fds.empty() || !buffer.releaseFence.fds.empty()) return bad(21);
            if (!buffer.buffer.fds.empty()) {
                if (buffer.buffer.fds.size()!=1 || buffer.buffer.ints!=std::vector<int32_t>{37,4} || cache.count(key)) return bad(22);
                cache[key].set(fcntl(buffer.buffer.fds[0].get(),F_DUPFD_CLOEXEC,0));
            }
            if (!cache.count(key) || cache[key].get()<0) return bad(23);
            Frame frame; frame.stream=buffer.streamId; frame.buffer=buffer.bufferId;
            frame.data.set(fcntl(cache[key].get(),F_DUPFD_CLOEXEC,0));
            if (frame.data.get()<0) return bad(24);
            frames.emplace(request.frameNumber,std::move(frame));
        }
        *count=static_cast<int32_t>(requests.size());
        if (mode==6) *count=0;
        if (mode==4) *count=-1;
        if (mode==3) *count+=1; // Successful but invalid processed-count contract.
        if (mode==5) { holding=true; condition.notify_all(); if (!condition.wait_for(guard,std::chrono::seconds(3),[this]{return !holding;})) {holding=false; return bad(28);} }
        if (mode==1) return bad(17);
        if (mode==2) { if (!requests.empty()) write(frames.at(requests[0].frameNumber).data.get(),"D",1); kill(getpid(),SIGKILL); _exit(99); }
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus returnResult(int32_t frame_number,int32_t behavior) override {
        std::shared_ptr<ICameraDeviceCallback> target;
        CaptureResult result; NotifyMsg notification;
        {
            std::lock_guard<std::mutex> guard(mutex);
            if (!frames.count(frame_number) || !callback) return bad(25);
            auto& frame=frames.at(frame_number); target=callback;
            result.frameNumber=frame_number; result.inputBuffer.streamId=-1;
            if (behavior==1) {
                result.partialResult=1; result.result.metadata={42};
                PhysicalCameraMetadata physical; physical.physicalCameraId="synthetic-physical"; physical.metadata.metadata={43}; result.physicalCameraMetadata.emplace_back(std::move(physical));
                ShutterMsg shutter; shutter.frameNumber=frame_number; shutter.timestamp=100; shutter.readoutTimestamp=101;
                StreamGroupState group; group.groupId=4; group.activeStreamIds={7}; shutter.streamGroupState.emplace_back(std::move(group));
                notification.set<NotifyMsg::Tag::shutter>(std::move(shutter));
            } else {
                StreamBuffer buffer; buffer.streamId=frame.stream; buffer.bufferId=behavior==4?100:frame.buffer;
                buffer.status=behavior==2?BufferStatus::ERROR:BufferStatus::OK;
                if (frame.fence_read.get()<0) { int pipefd[2]; if (pipe2(pipefd,O_CLOEXEC)!=0) return bad(24); frame.fence_read.set(pipefd[0]); frame.fence_write.set(pipefd[1]); }
                buffer.releaseFence.fds.emplace_back(fcntl(frame.fence_read.get(),F_DUPFD_CLOEXEC,0));
                result.outputBuffers.emplace_back(std::move(buffer));
                if (behavior==2) { ErrorMsg error; error.frameNumber=frame_number; error.errorStreamId=7; error.errorCode=ErrorCode::ERROR_REQUEST; notification.set<NotifyMsg::Tag::error>(std::move(error)); }
            }
        }
        if (behavior==1 || behavior==2) { std::vector<NotifyMsg> messages; messages.emplace_back(std::move(notification)); auto status=target->notify(messages); if(!status.isOk())return status; }
        std::vector<CaptureResult> results; results.emplace_back(std::move(result));
        return target->processCaptureResult(results);
    }
    ndk::ScopedAStatus signalFence(int32_t frame_number) override {
        std::lock_guard<std::mutex> guard(mutex);
        if (!frames.count(frame_number)) return bad(25);
        auto& frame=frames.at(frame_number);
        if(frame.fence_write.get()<0 || write(frame.fence_write.get(),"F",1)!=1)return bad(26);
        frame.fence_write.set(-1); return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus held(bool* value) override { std::lock_guard<std::mutex> guard(mutex); *value=holding; return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus releaseHold() override { std::lock_guard<std::mutex> guard(mutex); holding=false; condition.notify_all(); return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus releaseAll() override { std::lock_guard<std::mutex> guard(mutex); frames.clear(); cache.clear(); return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus fdCount(int32_t* count) override { DIR* dir=opendir("/proc/self/fd"); if(!dir)return bad(27); *count=0; while(auto* e=readdir(dir))if(e->d_name[0]!='.')++*count; closedir(dir); return ndk::ScopedAStatus::ok(); }
    ndk::ScopedAStatus die() override { kill(getpid(),SIGKILL); _exit(99); }
};
extern "C" JNIEXPORT jobject JNICALL Java_org_pokelinux_camerapeer_OwnershipPeer_createEndpoint(JNIEnv* env,jclass) {
    static auto peer=ndk::SharedRefBase::make<OwnershipPeer>(); return AIBinder_toJavaBinder(env,peer->asBinder().get());
}
