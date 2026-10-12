// Synthetic HAL controls; original generated SensorsV3 + public libfmq/EventFlag implement IPC.
#include <aidl/android/hardware/sensors/BnSensors.h>
#include <aidl/org/pokelinux/sensortest/BnSensorBoundaryControl.h>
#include <android/binder_ibinder_jni.h>
#include <fmq/AidlMessageQueue.h>
#include <dirent.h>
#include <mutex>
#include <signal.h>
#include <unistd.h>
#include <utils/SystemClock.h>
using namespace aidl::android::hardware::sensors;
using aidl::android::hardware::common::fmq::MQDescriptor;
using aidl::android::hardware::common::fmq::SynchronizedReadWrite;
using android::hardware::EventFlag;
using android::AidlMessageQueue;
static ndk::ScopedAStatus error(int code){return ndk::ScopedAStatus::fromServiceSpecificError(code);}
struct Queues {
    std::mutex mutex;
    std::unique_ptr<AidlMessageQueue<Event,SynchronizedReadWrite>> events;
    std::unique_ptr<AidlMessageQueue<int32_t,SynchronizedReadWrite>> acks;
    EventFlag* event_flag=nullptr;
    EventFlag* ack_flag=nullptr;
    std::shared_ptr<ISensorsCallback> callback;
    int pending=0;
    bool active=false;
    void clear(){EventFlag::deleteEventFlag(&event_flag);EventFlag::deleteEventFlag(&ack_flag);events.reset();acks.reset();pending=0;active=false;}
    ~Queues(){clear();}
};
class Sensor final:public BnSensors {
    std::shared_ptr<Queues> q;
public:
    explicit Sensor(std::shared_ptr<Queues> value):q(std::move(value)){}
    ndk::ScopedAStatus activate(int32_t handle,bool enabled)override{std::lock_guard<std::mutex> lock(q->mutex);if(handle!=77)return error(22);q->active=enabled;return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus batch(int32_t handle,int64_t period,int64_t latency)override{return handle==77&&period>0&&latency>=0?ndk::ScopedAStatus::ok():error(22);}
    ndk::ScopedAStatus configDirectReport(int32_t,int32_t,ISensors::RateLevel,int32_t*)override{return error(22);}
    ndk::ScopedAStatus flush(int32_t)override{return error(22);}
    ndk::ScopedAStatus getSensorsList(std::vector<SensorInfo>* out)override{SensorInfo info;info.sensorHandle=77;info.name="Synthetic original-FMQ accelerometer";info.vendor="Pokelinux fixture";info.type=SensorType::ACCELEROMETER;info.flags=SensorInfo::SENSOR_FLAG_BITS_WAKE_UP;info.minDelayUs=1000;out->push_back(info);return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus initialize(const MQDescriptor<Event,SynchronizedReadWrite>& events,const MQDescriptor<int32_t,SynchronizedReadWrite>& acks,const std::shared_ptr<ISensorsCallback>& cb)override{
        if(AIBinder_getCallingUid()!=2000 || !cb || !cb->isRemote())return error(13);
        std::lock_guard<std::mutex> lock(q->mutex);q->clear();
        q->events=std::make_unique<AidlMessageQueue<Event,SynchronizedReadWrite>>(events,false);
        q->acks=std::make_unique<AidlMessageQueue<int32_t,SynchronizedReadWrite>>(acks,false);
        if(!q->events->isValid()||!q->acks->isValid()||q->events->getQuantumCount()>8||q->acks->getQuantumCount()>8){q->clear();return error(22);}
        if(EventFlag::createEventFlag(q->events->getEventFlagWord(),&q->event_flag)!=android::OK || EventFlag::createEventFlag(q->acks->getEventFlagWord(),&q->ack_flag)!=android::OK){q->clear();return error(22);}
        q->callback=cb;return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus injectSensorData(const Event&)override{return error(22);}
    ndk::ScopedAStatus registerDirectChannel(const ISensors::SharedMemInfo&,int32_t*)override{return error(22);}
    ndk::ScopedAStatus setOperationMode(ISensors::OperationMode mode)override{return mode==ISensors::OperationMode::NORMAL?ndk::ScopedAStatus::ok():error(22);}
    ndk::ScopedAStatus unregisterDirectChannel(int32_t)override{return error(22);}
};
class Control final:public aidl::org::pokelinux::sensortest::BnSensorBoundaryControl {
    std::shared_ptr<Queues> q=std::make_shared<Queues>();
    std::shared_ptr<Sensor> sensor=ndk::SharedRefBase::make<Sensor>(q);
public:
    ndk::ScopedAStatus uid(int32_t* out)override{*out=getuid();return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus requestSubscription(const std::shared_ptr<aidl::org::pokelinux::sensortest::ISensorClient>& client,int32_t uid,int32_t kind,int64_t* out)override{return client->subscribe(uid,kind,out);}
    ndk::ScopedAStatus sensors(std::shared_ptr<ISensors>* out)override{*out=sensor;return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus eventAbi(std::vector<int32_t>* out)override{
        Event event;Event::EventPayload::Vec3 vec;vec.x=.5f;vec.y=1.25f;vec.z=-2.5f;vec.status=SensorStatus::ACCURACY_HIGH;event.payload.set<Event::EventPayload::Tag::vec3>(vec);
        const auto* base=reinterpret_cast<const char*>(&event);const auto& value=event.payload.get<Event::EventPayload::Tag::vec3>();
        // Event/EventPayload are verified standard-layout: their first field is the declared tag.
        *out={sizeof(Event),alignof(Event),int(reinterpret_cast<const char*>(&event.timestamp)-base),int(reinterpret_cast<const char*>(&event.sensorHandle)-base),int(reinterpret_cast<const char*>(&event.sensorType)-base),int(reinterpret_cast<const char*>(&event.payload)-base),sizeof(Event::EventPayload::Tag),int(reinterpret_cast<const char*>(&value.x)-base),int(reinterpret_cast<const char*>(&value.y)-base),int(reinterpret_cast<const char*>(&value.z)-base),int(reinterpret_cast<const char*>(&value.status)-base)};
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus produce(int32_t count,int64_t timestamp)override{
        std::lock_guard<std::mutex> lock(q->mutex);if(!q->events||!q->active||count<1||count>8||timestamp<0)return error(22);
        std::vector<Event> events(count);for(int i=0;i<count;i++){events[i].timestamp=timestamp+i;events[i].sensorHandle=77;events[i].sensorType=SensorType::ACCELEROMETER;Event::EventPayload::Vec3 vec;vec.x=.5f+i;vec.y=1.25f;vec.z=-2.5f;vec.status=SensorStatus::ACCURACY_HIGH;events[i].payload.set<Event::EventPayload::Tag::vec3>(vec);}
        if(q->pending>INT32_MAX-count)return error(12);q->pending+=count; // Source wake responsibility precedes queue publication in this fixture.
        if(!q->events->write(events.data(),events.size())){q->pending-=count;return error(12);}
        return q->event_flag->wake(ISensors::EVENT_QUEUE_FLAG_BITS_READ_AND_PROCESS)==android::OK?ndk::ScopedAStatus::ok():error(23);
    }
    ndk::ScopedAStatus produceMalformed()override{
        std::lock_guard<std::mutex> lock(q->mutex);if(!q->events||!q->active)return error(22);
        Event event;event.timestamp=700;event.sensorHandle=77;event.sensorType=SensorType::GYROSCOPE;
        Event::EventPayload::Vec3 vec;vec.x=.5f;vec.y=1.25f;vec.z=-2.5f;vec.status=SensorStatus::ACCURACY_HIGH;event.payload.set<Event::EventPayload::Tag::vec3>(vec);
        if(q->pending==INT32_MAX)return error(12);++q->pending;if(!q->events->write(&event)){--q->pending;return error(12);}
        return q->event_flag->wake(ISensors::EVENT_QUEUE_FLAG_BITS_READ_AND_PROCESS)==android::OK?ndk::ScopedAStatus::ok():error(23);
    }
    ndk::ScopedAStatus awaitAckTimeout(int32_t* out)override{
        std::lock_guard<std::mutex> lock(q->mutex);if(!q->acks||q->pending==0)return error(22);
        uint32_t flag=0;int64_t start=android::elapsedRealtime();
        auto status=q->ack_flag->wait(ISensors::WAKE_LOCK_QUEUE_FLAG_BITS_DATA_WRITTEN,&flag,int64_t(ISensors::WAKE_LOCK_TIMEOUT_SECONDS)*1000000000,false);
        *out=static_cast<int32_t>(android::elapsedRealtime()-start);
        return status==android::TIMED_OUT?ndk::ScopedAStatus::ok():error(24);
    }
    ndk::ScopedAStatus drainAcks(int32_t* out)override{
        std::lock_guard<std::mutex> lock(q->mutex);if(!q->acks)return error(22);uint32_t flag=0;
        auto status=q->ack_flag->wait(ISensors::WAKE_LOCK_QUEUE_FLAG_BITS_DATA_WRITTEN,&flag,100000000,false);
        if(status!=android::OK)return error(24);
        int64_t total=0;int32_t value;while(q->acks->availableToRead()){if(!q->acks->read(&value)||value<0)return error(22);total+=value;if(total>INT32_MAX)return error(22);}
        // Public multihal reference clamps oversized deltas. Rust policy must never send one.
        q->pending-=std::min<int64_t>(q->pending,total);*out=static_cast<int32_t>(total);return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus pendingWake(int32_t* out)override{std::lock_guard<std::mutex> lock(q->mutex);*out=q->pending;return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus awaitReadFlag(int32_t* out)override{std::lock_guard<std::mutex> lock(q->mutex);if(!q->event_flag)return error(22);uint32_t value=0;auto status=q->event_flag->wait(ISensors::EVENT_QUEUE_FLAG_BITS_EVENTS_READ,&value,100000000,false);*out=value;return status==android::OK?ndk::ScopedAStatus::ok():error(24);}
    ndk::ScopedAStatus dropQueues()override{std::lock_guard<std::mutex> lock(q->mutex);q->clear();return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus fdCount(int32_t* out)override{DIR* dir=opendir("/proc/self/fd");if(!dir)return error(22);*out=0;while(auto* e=readdir(dir))if(e->d_name[0]!='.')++*out;closedir(dir);return ndk::ScopedAStatus::ok();}
    ndk::ScopedAStatus die()override{kill(getpid(),SIGKILL);_exit(99);}
};
extern "C" JNIEXPORT jobject JNICALL Java_org_pokelinux_sensorpeer_Peer_createEndpoint(JNIEnv* env,jclass){static auto peer=ndk::SharedRefBase::make<Control>();return AIBinder_toJavaBinder(env,peer->asBinder().get());}
