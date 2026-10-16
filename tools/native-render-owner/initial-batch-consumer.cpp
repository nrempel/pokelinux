// FROZEN SOURCE-ONLY CONSUMER. No implementation/fallback for the proposed API.
// First compile against the exact current AQ15 SDK must fail on that missing API.
#include <aquamarine/backend/Session.hpp>
#include "pokelinux_render_executor.h"
#include <libseat.h>
#include <libudev.h>
#include <xf86drm.h>
#include <array>
#include <atomic>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <exception>
#include <limits>
#include <stdexcept>
#include <string>
#include <thread>
#include <type_traits>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>

using namespace Aquamarine;
namespace {
using Result=eSessionInitialRenderEventsResult; // deliberately absent in old SDK
using Signature=Result(CSession::*)(SSessionPendingEvent*,size_t,size_t*) const;
static_assert(std::is_same_v<decltype(&CSession::peekInitialRenderEvents),Signature>);
static_assert(std::is_same_v<std::underlying_type_t<Result>,uint32_t>);
static_assert(static_cast<uint32_t>(Result::READY)==0 && static_cast<uint32_t>(Result::NOT_READY)==1 &&
 static_cast<uint32_t>(Result::WRONG_OWNER)==2 && static_cast<uint32_t>(Result::CAPACITY)==3 &&
 static_cast<uint32_t>(Result::INVALID_PREFIX)==4 && static_cast<uint32_t>(Result::TERMINAL)==5);
constexpr size_t MAX=PR_EXECUTOR_MAX_BOOTSTRAP_EVENTS;
static_assert(MAX==34 && AQ_SESSION_MAX_DEVICE_LEASES==32);
static_assert(std::is_trivially_copyable_v<SSessionPendingEvent>);
using Storage=std::array<SSessionPendingEvent,MAX+1>;
uint64_t workUntil=0,closeUntil=0;
bool setupQualified=false;
struct Setup {const char* reason;};
void need(bool value,const char* reason){if(!value){throw std::runtime_error(reason);}}
uint64_t boot(){timespec ts{};need(clock_gettime(CLOCK_BOOTTIME,&ts)==0&&ts.tv_sec>=0&&ts.tv_nsec>=0&&ts.tv_nsec<1000000000L,"CLOCK_BOOTTIME");return uint64_t(ts.tv_sec)*1000000000ULL+uint64_t(ts.tv_nsec);}
void work(){if(boot()>=workUntil){if(!setupQualified){throw Setup{"ORIGINAL_SETUP_WORK_CUTOFF"};}throw std::runtime_error("ORIGINAL_WORK_CUTOFF");}}
bool sameStat(const struct stat& a,const struct stat& b){return a.st_dev==b.st_dev&&a.st_ino==b.st_ino&&a.st_rdev==b.st_rdev&&a.st_mode==b.st_mode;}
bool closed(int fd){errno=0;return fcntl(fd,F_GETFD)==-1&&errno==EBADF;}
struct Descriptor {int fd=-1;struct stat st{};int flags=-1;};
Descriptor descriptor(int fd,bool cloexec){Descriptor d;d.fd=fd;d.flags=fcntl(fd,F_GETFD);need(d.flags>=0&&fstat(fd,&d.st)==0&&S_ISCHR(d.st.st_mode),"ACTUAL_CHARACTER_FD");need(!cloexec||(d.flags&FD_CLOEXEC),"ACTUAL_CLOEXEC");return d;}
void stillOpen(const Descriptor& d){struct stat st{};need(fcntl(d.fd,F_GETFD)==d.flags&&fstat(d.fd,&st)==0&&sameStat(st,d.st),"EXACT_RETAINED_FD_CHANGED_OR_CLOSED");}
bool virtio(int fd){auto* version=drmGetVersion(fd);if(!version){return false;}const bool ok=version->name&&version->name_len>0&&version->name_len<=64&&std::string(version->name,static_cast<size_t>(version->name_len))=="virtio_gpu";drmFreeVersion(version);return ok;}
struct Revoke {
 uint64_t generation=73,calls=0;
 std::thread::id owner=std::this_thread::get_id();
 bool terminal=false,admissionOpen=true,metadataValid=true;
};
uint32_t unavailable(void* raw,uint64_t generation,uint32_t reason) noexcept {
 auto* state=static_cast<Revoke*>(raw);
 if(state){++state->calls;state->terminal=true;state->admissionOpen=false;
  state->metadataValid=state->metadataValid&&state->owner==std::this_thread::get_id()&&generation==state->generation&&reason>=AQ_REVOKE_DISABLE&&reason<=AQ_REVOKE_DEVICE_CHANGE;}
 // No Rust owner exists. Never forge AQ_REVOKED or AQ_ALREADY_STOPPED.
 return AQ_REVOKE_UNAVAILABLE;
}
struct Pack {PrBootstrap* value=nullptr;~Pack(){if(value){pr_bootstrap_release(value);}}};
struct Service {uint64_t lease=0,generation=0;std::string path;Descriptor primary,render;bool hasRender=false;};
using Services=std::array<Service,AQ_SESSION_MAX_DEVICE_LEASES>;
bool sameMetadata(const SSessionRenderEvent& a,const SSessionRenderEvent& b){
 return a.version==b.version&&a.type==b.type&&a.session_sequence==b.session_sequence&&a.session_generation==b.session_generation&&a.device_lease_id==b.device_lease_id&&a.device_generation==b.device_generation&&a.rdev==b.rdev&&a.active==b.active&&a.vt==b.vt&&a.change_type==b.change_type&&a.connector_id==b.connector_id&&a.prop_id==b.prop_id&&a.boot_vga==b.boot_vga&&std::memcmp(a.reserved,b.reserved,sizeof(a.reserved))==0&&std::memcmp(a.seat_name,b.seat_name,sizeof(a.seat_name))==0&&std::memcmp(a.device_path,b.device_path,sizeof(a.device_path))==0;
}
bool sameEvent(const SSessionPendingEvent& a,const SSessionPendingEvent& b){return sameMetadata(a.metadata,b.metadata)&&a.primary_fd==b.primary_fd&&a.render_fd==b.render_fd;}
PrBootstrapEventV1 copied(const SSessionPendingEvent& source){
 PrBootstrapEventV1 out{};const auto& a=source.metadata;auto& b=out.metadata;
 b.version=a.version;b.event_type=a.type;b.session_sequence=a.session_sequence;b.session_generation=a.session_generation;
 b.device_lease_id=a.device_lease_id;b.device_generation=a.device_generation;b.rdev=a.rdev;
 b.active=a.active;b.vt=a.vt;b.change_type=a.change_type;b.connector_id=a.connector_id;b.prop_id=a.prop_id;b.boot_vga=a.boot_vga;
 for(size_t i=0;i<4;++i){b.reserved[i]=a.reserved[i];}
 for(size_t i=0;i<64;++i){b.seat_name[i]=static_cast<uint8_t>(static_cast<unsigned char>(a.seat_name[i]));}
 for(size_t i=0;i<256;++i){b.device_path[i]=static_cast<uint8_t>(static_cast<unsigned char>(a.device_path[i]));}
 out.primary_fd=source.primary_fd;out.render_fd=source.render_fd;return out;
}
void exactCopy(const SSessionPendingEvent& source,const PrBootstrapEventV1& out){
 const auto& a=source.metadata;const auto& b=out.metadata;
 need(b.version==a.version&&b.event_type==a.type&&b.session_sequence==a.session_sequence&&b.session_generation==a.session_generation&&b.device_lease_id==a.device_lease_id&&b.device_generation==a.device_generation&&b.rdev==a.rdev&&b.active==a.active&&b.vt==a.vt&&b.change_type==a.change_type&&b.connector_id==a.connector_id&&b.prop_id==a.prop_id&&b.boot_vga==a.boot_vga,"FIELDWISE_RUST_COPY");
 for(size_t i=0;i<4;++i){need(b.reserved[i]==a.reserved[i],"COPY_RESERVED");}
 for(size_t i=0;i<64;++i){need(b.seat_name[i]==static_cast<uint8_t>(static_cast<unsigned char>(a.seat_name[i])),"COPY_SEAT_BYTES");}
 for(size_t i=0;i<256;++i){need(b.device_path[i]==static_cast<uint8_t>(static_cast<unsigned char>(a.device_path[i])),"COPY_PATH_BYTES");}
 need(out.primary_fd==source.primary_fd&&out.render_fd==source.render_fd,"COPY_BORROWED_FD_INTEGERS");
}
template<class T> auto bytes(const T& value){std::array<unsigned char,sizeof(T)> result{};std::memcpy(result.data(),&value,sizeof(value));return result;}
Storage poison(){Storage s;auto* p=reinterpret_cast<unsigned char*>(s.data());for(size_t i=0;i<sizeof(s);++i){p[i]=0xA5;}
 return s;}
void refusal(CSession& broker,Result expected,size_t capacity){
 Storage out=poison();const auto before=bytes(out);size_t count=std::numeric_limits<size_t>::max();const auto previous=bytes(count);
 need(broker.peekInitialRenderEvents(out.data(),capacity,&count)==expected,"EXACT_BATCH_REFUSAL");
 need(bytes(count)==previous&&bytes(out)==before,"REFUSAL_PRESERVES_ALL_OUTPUT_AND_COUNT_BYTES");
}
struct Summary {size_t count=0,devices=0,sourceFDs=0;uint64_t captureAt=0,firstMarkAt=0,lastMarkAt=0;};
Summary run(){
 const char* backend=std::getenv("LIBSEAT_BACKEND");const char* seatd=std::getenv("SEATD_SOCK");
 if(!backend||std::strcmp(backend,"seatd")||!seatd||seatd[0]!='/'||std::getenv("LD_PRELOAD")||std::getenv("LD_AUDIT")){throw Setup{"EXPLICIT_REAL_SEATD_PROFILE_REQUIRED"};}
 if(getuid()!=1000||geteuid()!=1000||getgid()!=1000||getegid()!=1000||getgroups(0,nullptr)!=0){{throw Setup{"UID1000_GID1000_EMPTY_GROUPS_REQUIRED"};}}
 struct stat seatSocket{};if(stat(seatd,&seatSocket)||!S_ISSOCK(seatSocket.st_mode)){throw Setup{"ACTUAL_SEATD_SOCKET_REQUIRED"};}
 Revoke revoke;SInputSessionOptions options;options.sessionGeneration=revoke.generation;options.revokeRender=unavailable;options.controlContext=&revoke;
 auto broker=CSession::attemptInputBroker(options);work();
 if(!broker||!broker->isInputBroker()||!broker->onOwnerThread()||!broker->active||broker->transportFailed()||revoke.calls||revoke.terminal||!revoke.admissionOpen){throw Setup{"ENABLED_ACTUAL_BROKER_WITHOUT_REVOKE_REQUIRED"};}
 if(!broker->libseatHandle||!broker->udevHandle||!broker->udevMonitor||!broker->libinputHandle){throw Setup{"ACTUAL_UDEV_GRAPHICS_ENUMERATION_REQUIRED"};}
 Services services{};size_t serviceCount=0;bool realRender=false;
 for(const auto& device:broker->sessionDevices){
  work();
  // Libinput bookkeeping also lives here; lease0 records stay control-owned.
  if(!device||device->leaseID==0){continue;}
  if(serviceCount>=AQ_SESSION_MAX_DEVICE_LEASES||!device->generation||device->retiring||device->fd<0){throw Setup{"ACTUAL_LIVE_GRAPHICS_SERVICE_LEASE_REQUIRED"};}
  auto& s=services[serviceCount++];s.lease=device->leaseID;s.generation=device->generation;s.path=device->path;s.primary=descriptor(device->fd,false);
  if(!virtio(s.primary.fd)||major(s.primary.st.st_rdev)!=226||minor(s.primary.st.st_rdev)>=128){throw Setup{"VIRTUAL_VIRTIO_PRIMARY_REQUIRED"};}
  if(device->renderNodeFd>=0){s.render=descriptor(device->renderNodeFd,true);s.hasRender=true;realRender=true;if(!virtio(s.render.fd)||major(s.render.st.st_rdev)!=226||minor(s.render.st.st_rdev)<128){throw Setup{"VIRTUAL_VIRTIO_RENDER_REQUIRED"};}}
 }
 if(serviceCount==0||!realRender){throw Setup{"ACTUAL_GRAPHICS_AND_RENDER_CAPABILITY_REQUIRED"};}
 auto front=broker->peekPendingRenderEvent();if(!front||front->metadata.type!=AQ_SESSION_INIT||front->metadata.session_sequence!=1){throw Setup{"INITIAL_FRONT_REQUIRED"};}
 setupQualified=true;work();
 std::printf("{\"event\":\"SETUP\",\"qualified\":true,\"work_until_ns\":%llu,\"close_until_ns\":%llu,\"actual_service_devices\":%zu}\n",static_cast<unsigned long long>(workUntil),static_cast<unsigned long long>(closeUntil),serviceCount);
 Storage events=poison();const auto beforePeek=bytes(events);size_t count=std::numeric_limits<size_t>::max();
 need(broker->peekInitialRenderEvents(events.data(),MAX,&count)==Result::READY,"COMPLETE_INITIAL_BATCH_READY");
 need(count>=3&&count<=MAX&&count==serviceCount+2,"GENUINE_INIT_ADD_AT_LEAST_ONE_ENUM_PREFIX");
 const auto afterPeek=bytes(events);for(size_t k=count*sizeof(SSessionPendingEvent);k<sizeof(events);++k){need(afterPeek[k]==beforePeek[k],"SUCCESS_PRESERVES_UNUSED_STORAGE");}
 std::array<Descriptor,MAX*2> pending{};size_t descriptors=0;std::array<bool,AQ_SESSION_MAX_DEVICE_LEASES> seen{};
 for(size_t k=0;k<count;++k){
  work();const auto& event=events[k];const auto& m=event.metadata;
  need(m.version==AQ_SESSION_EVENT_VERSION&&m.session_generation==revoke.generation&&m.session_sequence==k+1,"ACTUAL_INITIAL_ORDER_GENERATION");
  need(m.active<=1&&m.boot_vga<=1&&std::memchr(m.seat_name,0,sizeof(m.seat_name))&&std::memchr(m.device_path,0,sizeof(m.device_path)),"BOUNDED_METADATA");
  for(auto reserved:m.reserved){need(reserved==0,"SOURCE_RESERVED_ZERO");}
  if(k==0){need(m.type==AQ_SESSION_INIT&&m.active==1&&m.vt==broker->vt&&std::string(m.seat_name)==broker->seatName&&libseat_seat_name(broker->libseatHandle)&&std::string(m.seat_name)==libseat_seat_name(broker->libseatHandle),"ACTUAL_LIBSEAT_INIT");need(event.primary_fd==-1&&event.render_fd==-1,"INIT_HAS_NO_FDS");continue;}
  if(k+1==count){need(m.type==AQ_SESSION_ENUMERATION_DONE&&event.primary_fd==-1&&event.render_fd==-1,"ENUM_HAS_NO_FDS");continue;}
  need(m.type==AQ_SESSION_DEVICE_ADD&&event.primary_fd>=0&&m.device_lease_id&&m.device_generation&&m.device_path[0],"GENUINE_DEVICE_ADD");
  size_t slot=0;while(slot<serviceCount&&services[slot].lease!=m.device_lease_id){++slot;}
  need(slot<serviceCount&&!seen[slot],"UNIQUE_ACTUAL_SERVICE_LEASE");seen[slot]=true;const auto& service=services[slot];
  need(service.generation==m.device_generation&&service.path==m.device_path&&uint64_t(service.primary.st.st_rdev)==m.rdev,"EXACT_SERVICE_METADATA");
  struct stat path{};need(stat(m.device_path,&path)==0&&sameStat(path,service.primary.st),"ACTUAL_PRIMARY_DEVICE_PATH");
  auto primary=descriptor(event.primary_fd,true);need(event.primary_fd!=service.primary.fd&&sameStat(primary.st,service.primary.st),"PENDING_PRIMARY_IS_DISTINCT_BORROWED_DUPLICATE");pending[descriptors++]=primary;
  need((event.render_fd>=0)==service.hasRender,"ALL_ACTUAL_RENDER_CAPABILITIES_COPIED");
  if(service.hasRender){auto render=descriptor(event.render_fd,true);need(event.render_fd!=service.render.fd&&sameStat(render.st,service.render.st),"PENDING_RENDER_IS_DISTINCT_BORROWED_DUPLICATE");pending[descriptors++]=render;}
  auto* device=udev_device_new_from_devnum(broker->udevHandle,'c',static_cast<dev_t>(m.rdev));need(device,"ACTUAL_UDEV_PRIMARY");
  const char* node=udev_device_get_devnode(device);const bool matches=node&&service.path==node;auto* pci=udev_device_get_parent_with_subsystem_devtype(device,"pci",nullptr);const char* bootFlag=pci?udev_device_get_sysattr_value(pci,"boot_vga"):nullptr;const bool bootVGA=bootFlag&&std::strcmp(bootFlag,"1")==0;udev_device_unref(device);
  need(matches&&m.boot_vga==uint32_t(bootVGA),"ACTUAL_UDEV_PATH_BOOT_VGA");
 }
 for(size_t a=0;a<descriptors;++a){for(size_t b=a+1;b<descriptors;++b){need(pending[a].fd!=pending[b].fd,"DISTINCT_QUEUED_DUPLICATES");}}
 auto intact=[&]{for(size_t k=0;k<descriptors;++k){stillOpen(pending[k]);}for(size_t k=0;k<serviceCount;++k){stillOpen(services[k].primary);if(services[k].hasRender){stillOpen(services[k].render);}}auto current=broker->peekPendingRenderEvent();need(current&&sameEvent(*current,events[0]),"PEEK_NEVER_ADVANCED_FRONT");};
 intact();Storage repeated=poison();size_t repeatedCount=0;need(broker->peekInitialRenderEvents(repeated.data(),MAX,&repeatedCount)==Result::READY&&repeatedCount==count,"REPEATED_COMPLETE_PEEK");
 for(size_t k=0;k<count;++k){need(sameEvent(events[k],repeated[k]),"REPEATED_FIELDWISE_IDENTICAL_BATCH");}
 intact();
 refusal(*broker,Result::CAPACITY,count-1);intact();
 Storage wrong=poison();const auto wrongBefore=bytes(wrong);size_t wrongCount=std::numeric_limits<size_t>::max();const auto wrongCountBefore=bytes(wrongCount);std::atomic<bool> returned=false;Result wrongResult=Result::READY;std::exception_ptr wrongFailure;
 const uint64_t wrongUntil=workUntil;std::thread other([&]{try{wrongResult=broker->peekInitialRenderEvents(wrong.data(),MAX,&wrongCount);}catch(...){wrongFailure=std::current_exception();}returned.store(true,std::memory_order_release);});
 while(!returned.load(std::memory_order_acquire)&&boot()<wrongUntil){std::this_thread::yield();}
 if(!returned.load(std::memory_order_acquire)){std::puts("{\"status\":\"FAIL\",\"reason\":\"WRONG_OWNER_CALL_BLOCKED_PROCESS_DISPOSAL_NO_JOIN\"}");std::fflush(stdout);std::_Exit(1);}
 other.join();work();if(wrongFailure){std::rethrow_exception(wrongFailure);}need(wrongResult==Result::WRONG_OWNER&&bytes(wrongCount)==wrongCountBefore&&bytes(wrong)==wrongBefore,"WRONG_OWNER_PRESERVES_ALL_BYTES");intact();
 std::array<PrBootstrapEventV1,MAX> rust{};for(size_t k=0;k<count;++k){rust[k]=copied(events[k]);exactCopy(events[k],rust[k]);}
 need(revoke.admissionOpen&&!revoke.terminal&&revoke.calls==0&&!broker->transportFailed(),"PREWORKER_ADMISSION_LATCH_BEFORE_RUST_CAPTURE");
 Pack pack;const int code=pr_bootstrap_capture_v1(revoke.generation,rust.data(),static_cast<uint32_t>(count),&pack.value);work();need(code==PR_EXECUTOR_OK&&pack.value,"ACTUAL_ORIGINAL_RUST_BOOTSTRAP_CAPTURE_ACCEPTED");intact();
 Summary summary;summary.count=count;summary.devices=serviceCount;summary.sourceFDs=descriptors;summary.captureAt=boot();
 std::printf("{\"event\":\"RUST_CAPTURE\",\"code\":0,\"events\":%zu,\"at_ns\":%llu,\"no_queue_mark_yet\":true}\n",count,static_cast<unsigned long long>(summary.captureAt));
 for(size_t k=0;k<count;++k){
  work();need(revoke.admissionOpen&&!revoke.terminal&&revoke.calls==0&&!broker->transportFailed(),"NO_ADMISSION_AFTER_PREWORKER_REVOKE");
  auto current=broker->peekPendingRenderEvent();need(current&&sameEvent(*current,events[k]),"EXACT_FRONT_BEFORE_ORDERED_MARK");
  const auto entered=boot();if(k==0){summary.firstMarkAt=entered;}need(entered>=summary.captureAt,"MARK_ONLY_AFTER_RUST_CAPTURE_SUCCESS");
  need(broker->markRenderEventAdmitted(events[k].metadata.session_sequence),"ORDERED_MARK_AFTER_DUPLICATION");
  need((events[k].primary_fd<0||closed(events[k].primary_fd))&&(events[k].render_fd<0||closed(events[k].render_fd)),"SOURCE_PENDING_DUPLICATES_CLOSED_AFTER_MARK");
  for(size_t next=k+1;next<count;++next){if(events[next].primary_fd>=0){need(fcntl(events[next].primary_fd,F_GETFD)>=0,"LATER_PENDING_PRIMARY_RETAINED");}if(events[next].render_fd>=0){need(fcntl(events[next].render_fd,F_GETFD)>=0,"LATER_PENDING_RENDER_RETAINED");}}
  for(size_t d=0;d<serviceCount;++d){stillOpen(services[d].primary);if(services[d].hasRender){stillOpen(services[d].render);}}
  auto advanced=broker->peekPendingRenderEvent();if(k+1<count){need(advanced&&sameEvent(*advanced,events[k+1]),"EXACT_QUEUE_ADVANCE");}else{need(!advanced,"INITIAL_QUEUE_DRAINED");}
  refusal(*broker,k+1<count?Result::INVALID_PREFIX:Result::NOT_READY,MAX);summary.lastMarkAt=boot();
 }
 need(summary.captureAt<=summary.firstMarkAt&&summary.firstMarkAt<=summary.lastMarkAt,"CAPTURE_PRECEDES_ALL_MARKS");
 // Explicit public terminal request AFTER the positive boundary; never forge an
 // actual disable callback, service retirement, mirror, discovery or Rust owner.
 need(revoke.calls==0&&!revoke.terminal&&revoke.admissionOpen&&!broker->transportFailed(),"NO_REVOKE_DURING_COMPLETE_POSITIVE_BOUNDARY");
 broker->failRenderTransport();need(revoke.calls==1&&revoke.terminal&&!revoke.admissionOpen&&revoke.metadataValid&&broker->transportFailed()&&broker->lastRenderRevocationResult()==AQ_REVOKE_UNAVAILABLE&&!broker->active,"TRUTHFUL_PREWORKER_UNAVAILABLE_TERMINAL_LATCH");
 refusal(*broker,Result::TERMINAL,MAX);need(!broker->markRenderEventAdmitted(1),"TERMINAL_ADMISSION_REFUSED");
 pr_bootstrap_release(pack.value);pack.value=nullptr;work();
 for(size_t d=0;d<serviceCount;++d){stillOpen(services[d].primary);if(services[d].hasRender){stillOpen(services[d].render);}}
 std::puts("{\"event\":\"PREWORKER_REVOKE\",\"result\":3,\"terminal_latched\":true,\"service_retirement_qualified\":false}");
 return summary; // broker destructor is forced service cleanup, not retirement
}
}
int main(int argc,char** argv){
 if(argc!=2||std::strcmp(argv[1],"--actual-seatd-udev-virtual-drm-approved")){std::puts("{\"status\":\"SETUP\",\"reason\":\"EXPLICIT_RUNTIME_PROFILE_REQUIRED\"}");return 77;}
 int result=1;Summary summary{};
 try{const auto started=boot();workUntil=started+5000000000ULL;closeUntil=started+8000000000ULL;summary=run();need(boot()<closeUntil,"ORIGINAL_CLOSE_CUTOFF");result=0;}
 catch(const Setup& e){std::printf("{\"status\":\"SETUP\",\"reason\":\"%s\"}\n",e.reason);result=77;}
 catch(const std::exception& e){std::fprintf(stderr,"AQ_INITIAL_BATCH: %.240s\n",e.what());std::printf("{\"status\":\"%s\",\"reason\":\"CONSUMER_ASSERTION_OR_EXCEPTION\"}\n",setupQualified?"FAIL":"SETUP");result=setupQualified?1:77;}
 if(closeUntil&&boot()>=closeUntil){std::puts("{\"status\":\"CLOSURE_FAILED\",\"reason\":\"ORIGINAL_CLOSE_CUTOFF\"}");return 1;}
 if(result==0){std::printf("{\"status\":\"PASS\",\"events\":%zu,\"devices\":%zu,\"source_pending_fds_closed\":%zu,\"Rust_capture_before_marks\":true,\"worker_or_output_discovery_qualified\":false,\"service_lease_retirement_qualified\":false}\n",summary.count,summary.devices,summary.sourceFDs);}
 return result;
}
