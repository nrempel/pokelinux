// Actual C++ -> Rust executor ABI fixture; authored callbacks, not GL/AQ/DRM proof.
// Build only after selecting one freshly built Rust archive and this exact header:
// c++ -std=c++20 -O2 -Wall -Wextra -Werror -pedantic -pthread -Iinclude
//   tests/executor_ffi.cpp /absolute/selected/libpokelinux_render.a
//   -ldl -lm -lrt -lutil -o /absolute/output/executor-ffi
// No second Rust archive. Preserve actual child wait/PIPE EOF in the outer runner.
#include "pokelinux_render_executor.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <fcntl.h>
#include <limits>
#include <mutex>
#include <new>
#include <poll.h>
#include <stdexcept>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <thread>
#include <time.h>
#include <type_traits>
#include <unistd.h>

static_assert(sizeof(PrSessionRenderEventV1)==408);
static_assert(offsetof(PrSessionRenderEventV1,seat_name)==88);
static_assert(sizeof(PrOperationPolicyV1)==40);
static_assert(std::is_trivially_copyable_v<PrNativeOwnerFunctionsV2>);
static_assert(std::is_trivially_copyable_v<PrOwnerOptionsV1>);
namespace {
constexpr uint64_t A=101,B=202,SECOND=1000000000ULL;
constexpr uint32_t AR24=0x34325241;
uint64_t work_until=0,close_until=0;
pid_t control_tid=0;
std::atomic<uint32_t> poisoned_calls{0};
uint64_t now(){
 timespec t{};
 if(clock_gettime(CLOCK_BOOTTIME,&t)||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000L)throw std::runtime_error("BOOTTIME");
 return static_cast<uint64_t>(t.tv_sec)*SECOND+static_cast<uint64_t>(t.tv_nsec);
}
void require(bool value,const char* why){if(!value)throw std::runtime_error(why);}
pid_t tid(){return static_cast<pid_t>(syscall(SYS_gettid));}
uint64_t until(){return std::min(work_until,now()+2*SECOND);}
template<class T> std::array<unsigned char,sizeof(T)> bytes(const T& value){
 std::array<unsigned char,sizeof(T)> out{};std::memcpy(out.data(),&value,sizeof(value));return out;
}
template<class T> T sentinel(){T out;std::memset(&out,0xA5,sizeof(out));return out;}
enum Event:uint32_t {CREATE_ENTER,CREATED,BOOTSTRAP_ENTER,BOOTSTRAP_RETURN,PREPARE,EXECUTE,ADVANCE,POLL_SOURCES,DISPATCH,SHUTDOWN_PENDING,SHUTDOWN_READY,DESTROY};
struct Record {Event event;pid_t thread;uint64_t owner;};
struct Probe {
 std::mutex mutex;std::condition_variable changed;
 std::array<Record,128> records{};size_t count=0;
 bool release_create=false,release_bootstrap=false,release_shutdown=false,error=false;
 int wake=-1;
 void reset(){
  std::lock_guard lock(mutex);require(wake==-1,"probe still has native FD");
  records={};count=0;release_create=release_bootstrap=release_shutdown=error=false;
 }
 void mark(Event event,uint64_t owner){
  std::lock_guard lock(mutex);require(count<records.size(),"bounded callback records");records[count++]={event,tid(),owner};changed.notify_all();
 }
 bool has_locked(Event event)const{for(size_t i=0;i<count;++i)if(records[i].event==event)return true;return false;}
 void wait(Event event,uint64_t cutoff){
  std::unique_lock lock(mutex);
  while(!has_locked(event)&&!error){const auto t=now();require(t<cutoff,"callback observation deadline");changed.wait_for(lock,std::chrono::nanoseconds(cutoff-t));}
  require(!error,"callback reported failure");
 }
 void gate(bool Probe::*field,uint64_t cutoff){
  std::unique_lock lock(mutex);
  while(!(this->*field)){const auto t=now();require(t<cutoff,"fixture barrier deadline");changed.wait_for(lock,std::chrono::nanoseconds(cutoff-t));}
 }
 void fail()noexcept{try{std::lock_guard lock(mutex);error=true;changed.notify_all();}catch(...){}}
 void open(bool Probe::*field){std::lock_guard lock(mutex);this->*field=true;changed.notify_all();}
 void signal(){
  std::lock_guard lock(mutex);require(wake>=0,"worker-owned source absent");uint64_t one=1;
  require(write(wake,&one,sizeof(one))==static_cast<ssize_t>(sizeof(one))||errno==EAGAIN,"native fixture wake");
 }
 void open_shutdown(){
  std::lock_guard lock(mutex);release_shutdown=true;changed.notify_all();
  if(wake>=0){uint64_t one=1;require(write(wake,&one,sizeof(one))==static_cast<ssize_t>(sizeof(one))||errno==EAGAIN,"shutdown source wake");}
 }
 bool shutdown_released(){std::lock_guard lock(mutex);return release_shutdown;}
 void all_open()noexcept{try{open(&Probe::release_create);open(&Probe::release_bootstrap);open_shutdown();}catch(...){fail();}}
 std::array<Record,128> copy(size_t& length){std::lock_guard lock(mutex);require(!error,"callback failure");length=count;return records;}
};
// These fixed slots are fixture observation/barriers ONLY. No owner-id lookup,
// captured configuration, native object or control userdata is obtained here.
// Each distinct static create function below binds one slot directly.
Probe probe_a,probe_b;
struct NativeOwner {
 Probe& probe;PrOwnerOptionsV1 options;pid_t worker;int wake=-1;
 uint64_t session=0,lease=0,device_generation=0;int primary=-1,render=-1;
 bool pending_marked=false;
 NativeOwner(Probe& p,const PrOwnerOptionsV1& o):probe(p),options(o),worker(tid()){}
 void current(){require(tid()==worker&&worker!=control_tid,"native callback wrong thread");}
};
template<class F> int32_t callback(void* pointer,F function)noexcept{
 auto* owner=static_cast<NativeOwner*>(pointer);
 try{require(owner!=nullptr,"missing native owner");owner->current();function(*owner);return PR_EXECUTOR_OK;}
 catch(...){if(owner)owner->probe.fail();return PR_EXECUTOR_NATIVE;}
}
void* create(const PrOwnerOptionsV1* options,Probe& probe,uint64_t expected)noexcept{
 try{
  probe.mark(CREATE_ENTER,expected);probe.gate(&Probe::release_create,until());
  require(options&&options->abi_version==PR_EXECUTOR_ABI_V2&&options->struct_size==sizeof(*options)&&options->owner_id==expected,"copied factory options");
  for(auto value:options->reserved)require(value==0,"copied option reserved");
  auto* owner=new(std::nothrow) NativeOwner(probe,*options);require(owner!=nullptr,"CPU holder allocation");
  require(owner->worker!=control_tid,"factory ran on control");probe.mark(CREATED,owner->options.owner_id);return owner;
 }catch(...){probe.fail();return nullptr;}
}
void* create_a(const PrOwnerOptionsV1* options)noexcept{return create(options,probe_a,A);}
void* create_b(const PrOwnerOptionsV1* options)noexcept{return create(options,probe_b,B);}
void destroy(void* pointer)noexcept{
 auto* owner=static_cast<NativeOwner*>(pointer);
 try{require(owner!=nullptr,"destroy null");owner->current();require(owner->wake==-1,"native resources not retired");owner->probe.mark(DESTROY,owner->options.owner_id);delete owner;}
 catch(...){if(owner)owner->probe.fail();}
}
int32_t bootstrap(void* pointer,const PrBootstrapViewV1* input,PrDiscoveryV1* result)noexcept{
 return callback(pointer,[&](NativeOwner& owner){
  require(input&&result&&input->events&&input->event_count==3&&input->reserved==0,"bootstrap shape");
  require(input->session_generation==owner.options.owner_id+1000,"distinct copied session");
  for(uint32_t i=0;i<3;++i){const auto& e=input->events[i];require(e.metadata.version==1&&e.metadata.event_type==i+1&&e.metadata.session_sequence==i+1&&e.metadata.session_generation==input->session_generation,"copied bootstrap grammar");}
  const auto& add=input->events[1];struct stat info{};
  require(add.metadata.device_lease_id==owner.options.owner_id+30&&add.metadata.device_generation==3&&std::strcmp(reinterpret_cast<const char*>(add.metadata.device_path),"/dev/null")==0,"copied bootstrap values");
  for(int fd:{add.primary_fd,add.render_fd}){const int flags=fcntl(fd,F_GETFD);require(fd>=0&&fstat(fd,&info)==0&&S_ISCHR(info.st_mode)&&static_cast<uint64_t>(info.st_rdev)==add.metadata.rdev&&flags>=0&&(flags&FD_CLOEXEC),"real duplicated character FD");}
  owner.session=input->session_generation;owner.lease=add.metadata.device_lease_id;owner.device_generation=add.metadata.device_generation;
  owner.primary=add.primary_fd;owner.render=add.render_fd;
  owner.wake=eventfd(0,EFD_CLOEXEC|EFD_NONBLOCK);require(owner.wake>=0,"worker-owned fixture source");
  {std::lock_guard lock(owner.probe.mutex);owner.probe.wake=owner.wake;}
  owner.probe.mark(BOOTSTRAP_ENTER,owner.options.owner_id);owner.probe.gate(&Probe::release_bootstrap,until());
  PrDiscoveryV1 value{};value.session_generation=owner.session;value.output_count=1;
  auto& out=value.outputs[0];out.output_id=owner.options.owner_id*10;out.output_generation=1;out.device_lease_id=owner.lease;out.device_generation=owner.device_generation;
  out.mode_id=1;out.mode_generation=1;out.connector_id=1;out.width=8;out.height=6;out.refresh_millihz=60000;out.drm_format=AR24;out.flags=1;
  *result=value;owner.probe.mark(BOOTSTRAP_RETURN,owner.options.owner_id);
 });
}
struct ViewCheck {uint64_t scene;bool checked=false;};
int32_t inspect_frame(const PrFrameBorrowV1* frame,void* token)noexcept{
 auto& check=*static_cast<ViewCheck*>(token);
 try{
  require(frame&&frame->scene_sequence==check.scene&&frame->draw_count==1&&frame->resource_count==1,"actual Frame metadata");
  PrSnapshotView pixels{};require(pr_snapshot_view(frame->resources[0].snapshot,&pixels)==PR_OK&&pixels.pixel_bytes==32,"actual retained Snapshot");
  for(size_t i=0;i<32;++i)require(pixels.pixels[i]==static_cast<uint8_t>(i^0x6D),"immutable bytes after original drop");
  check.checked=true;return 0;
 }catch(...){return 1;}
}
int32_t prepare(void* pointer,const PrFrame* frame,const PrAdmissionV1* admission)noexcept{
 return callback(pointer,[&](NativeOwner& owner){require(frame&&admission,"prepare inputs");ViewCheck value{admission->policy.scene_sequence};require(pr_frame_with_view_v1(frame,inspect_frame,&value)==0&&value.checked,"actual frame prepare view");owner.probe.mark(PREPARE,owner.options.owner_id);});
}
int32_t execute(void* pointer,const PrFrame* frame,const PrAdmissionV1* admission,PrAttempt* attempt,PrNativeResultV1* result)noexcept{
 return callback(pointer,[&](NativeOwner& owner){
  require(frame&&admission&&attempt&&result,"execute inputs");PrNativeScopeV1 scope{};scope.policy=admission->policy;scope.device_lease_id=owner.lease;scope.device_generation=owner.device_generation;scope.object_count=1;
  scope.objects[0]={77,0,1,admission->policy.output_id,admission->policy.output_generation};
  PrGrantObservationV1 grant{};require(pr_attempt_authorize_v1(attempt,&scope,&grant)==PR_EXECUTOR_OK&&grant.serial>0&&grant.at_boottime_ns>0,"actual authored one-shot authorization");
  auto refused=sentinel<PrGrantObservationV1>();const auto before=bytes(refused);
  require(pr_attempt_authorize_v1(attempt,&scope,&refused)==PR_EXECUTOR_STALE&&bytes(refused)==before,"one-shot second attempt refuses and preserves output");
  // Caller storage may change after synchronous authorization. Completion must
  // retain the original bounded scope value, never this pointer.
  scope.objects[0].id=909;scope.device_lease_id=0;
  *result={PR_NATIVE_RETURNED,42,0,0,1,1,0};owner.probe.mark(EXECUTE,owner.options.owner_id);
 });
}
int32_t advance(void* pointer,const PrRetirementContextV1* context,PrNativeProgressV1* result)noexcept{
 return callback(pointer,[&](NativeOwner& owner){
  require(context&&result&&context->has_grant==1&&context->has_scope==1&&context->scope.object_count==1&&context->scope.objects[0].id==77&&context->scope.device_lease_id==owner.lease,"copied authorized scope survives caller mutation");
  require(context->native.kind==PR_NATIVE_RETURNED&&context->native.succeeded==1&&context->native.extra_attempt_refused==1,"authored native result preserved");
  *result={};result->resources_retired=1;owner.probe.mark(ADVANCE,owner.options.owner_id);
 });
}
int32_t poll_sources(void* pointer,PrPollSourceV1* storage,uint32_t capacity,uint32_t* count)noexcept{
 return callback(pointer,[&](NativeOwner& owner){require(storage&&capacity>=1&&count,"poll source storage");*count=0;if(owner.wake>=0){storage[0]={};storage[0].fd=owner.wake;storage[0].token=owner.options.owner_id;storage[0].generation=1;storage[0].events=POLLIN;*count=1;}owner.probe.mark(POLL_SOURCES,owner.options.owner_id);});
}
int32_t dispatch(void* pointer,const PrNativeReadyV1* ready)noexcept{
 return callback(pointer,[&](NativeOwner& owner){require(ready&&ready->source.fd==owner.wake&&ready->source.token==owner.options.owner_id&&ready->source.generation==1&&(ready->revents&POLLIN),"actual fixture FD dispatch");uint64_t value=0;const auto n=read(owner.wake,&value,sizeof(value));require(n==static_cast<ssize_t>(sizeof(value))||(n<0&&errno==EAGAIN),"fixture source read");owner.probe.mark(DISPATCH,owner.options.owner_id);});
}
int32_t shutdown_progress(void* pointer,uint32_t* retired)noexcept{
 return callback(pointer,[&](NativeOwner& owner){
  require(retired,"shutdown output");*retired=0;
  if(!owner.probe.shutdown_released()){if(!owner.pending_marked){owner.probe.mark(SHUTDOWN_PENDING,owner.options.owner_id);owner.pending_marked=true;}return;}
  if(owner.wake>=0){std::lock_guard lock(owner.probe.mutex);owner.probe.wake=-1;const int fd=owner.wake;owner.wake=-1;require(close(fd)==0,"worker native source close");}
  *retired=1;owner.probe.mark(SHUTDOWN_READY,owner.options.owner_id);
 });
}
void* poison_create(const PrOwnerOptionsV1*)noexcept{++poisoned_calls;return nullptr;}
void poison_destroy(void*)noexcept{++poisoned_calls;}
int32_t poison_bootstrap(void*,const PrBootstrapViewV1*,PrDiscoveryV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_prepare(void*,const PrFrame*,const PrAdmissionV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_execute(void*,const PrFrame*,const PrAdmissionV1*,PrAttempt*,PrNativeResultV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_advance(void*,const PrRetirementContextV1*,PrNativeProgressV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_poll(void*,PrPollSourceV1*,uint32_t,uint32_t*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_dispatch(void*,const PrNativeReadyV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t poison_shutdown(void*,uint32_t*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
int32_t apply_device_change(void*,const PrSessionRenderEventV1*,PrDiscoveryV1*)noexcept{return PR_EXECUTOR_INVALID;}
int32_t poison_apply_device_change(void*,const PrSessionRenderEventV1*,PrDiscoveryV1*)noexcept{++poisoned_calls;return PR_EXECUTOR_NATIVE;}
PrNativeOwnerFunctionsV2 table(bool second=false,bool poison=false){
 PrNativeOwnerFunctionsV2 f{};f.abi_version=PR_EXECUTOR_ABI_V2;f.struct_size=sizeof(f);
 f.create=poison?poison_create:second?create_b:create_a;f.destroy=poison?poison_destroy:destroy;f.bootstrap=poison?poison_bootstrap:bootstrap;
 f.prepare=poison?poison_prepare:prepare;f.execute=poison?poison_execute:execute;f.advance=poison?poison_advance:advance;
 f.poll_sources=poison?poison_poll:poll_sources;f.dispatch=poison?poison_dispatch:dispatch;f.shutdown_progress=poison?poison_shutdown:shutdown_progress;f.apply_device_change=poison?poison_apply_device_change:apply_device_change;return f;
}
PrOwnerOptionsV1 options(uint64_t id){PrOwnerOptionsV1 value{};value.abi_version=PR_EXECUTOR_ABI_V2;value.struct_size=sizeof(value);value.owner_id=id;return value;}
PrBootstrap* pack(uint64_t id){
 const int fd=open("/dev/null",O_RDONLY|O_CLOEXEC);require(fd>=0,"/dev/null fixture");struct stat info{};
 if(fstat(fd,&info)||!S_ISCHR(info.st_mode)){close(fd);throw std::runtime_error("fixture character rdev");}
 std::array<PrBootstrapEventV1,3> events{};
 for(uint32_t i=0;i<events.size();++i){auto& e=events[i];e.primary_fd=e.render_fd=-1;e.metadata.version=1;e.metadata.event_type=i+1;e.metadata.session_sequence=i+1;e.metadata.session_generation=id+1000;e.metadata.active=1;}
 std::memcpy(events[0].metadata.seat_name,"seat-ffi",9);auto& add=events[1];add.metadata.device_lease_id=id+30;add.metadata.device_generation=3;add.metadata.rdev=static_cast<uint64_t>(info.st_rdev);std::memcpy(add.metadata.device_path,"/dev/null",10);add.primary_fd=add.render_fd=fd;
 PrBootstrap* out=nullptr;const auto status=pr_bootstrap_capture_v1(id+1000,events.data(),events.size(),&out);close(fd);
 std::memset(events.data(),0xEE,sizeof(events));require(status==PR_EXECUTOR_OK&&out,"real copied FD/metadata capture");return out;
}
PrFrame* frame(uint64_t id,uint64_t sequence){
 std::array<uint8_t,32> pixels{};for(size_t i=0;i<pixels.size();++i)pixels[i]=static_cast<uint8_t>(i^0x6D);
 auto* store=pr_snapshot_store_create(32);require(store,"Snapshot store");PrSnapshot* snapshot=nullptr;
 const PrShmDescription d{sequence,1,sequence,AR24,3,2,16,4,0};const auto s=pr_snapshot_capture(store,&d,pixels.data(),pixels.size(),&snapshot);
 if(s!=PR_OK){pr_snapshot_store_release(store);throw std::runtime_error("actual Snapshot capture");}
 const std::array<float,9> identity={1,0,0,0,1,0,0,0,1};PrFrameDrawV1 draw{};draw.kind=PR_DRAW_SHM;draw.resource_index=0;draw.scissor_first=2;draw.scissor_count=1;draw.box={0,0,3,2,0};draw.uv[2]=draw.uv[3]=1;draw.material.alpha=1;draw.material.rounding_power=2;draw.surface_id=id;draw.commit_sequence=sequence;std::memcpy(draw.projection,identity.data(),sizeof(draw.projection));
 const std::array<PrFrameRectV1,3> rects={{{0,0,8,6},{0,0,8,6},{0,0,8,6}}};const PrFrameResourceBorrowV1 resource{d,snapshot};PrFrameBorrowV1 v{};
 v.abi_version=1;v.struct_size=sizeof(v);v.flags=PR_FRAME_COMMIT;v.scene_sequence=sequence;v.security_generation=1;v.target.output_id=id*10;v.target.output_generation=1;v.target.width=v.target.transformed_width=8;v.target.height=v.target.transformed_height=6;v.target.scale=1;
 std::memcpy(v.target.target_projection,identity.data(),sizeof(v.target.target_projection));std::memcpy(v.target.scale_matrix,identity.data(),sizeof(v.target.scale_matrix));v.damage_count=1;v.final_damage_first=1;v.final_damage_count=1;v.draw_count=1;v.resource_count=1;v.rect_count=3;v.draws=&draw;v.resources=&resource;v.rects=rects.data();
 PrFrame* out=nullptr;const auto status=pr_frame_capture_v1(&v,&out);pr_snapshot_release(snapshot);pr_snapshot_store_release(store);pixels.fill(0xEE);require(status==PR_FRAME_OK&&out,"actual immutable Frame capture");return out;
}
PrOperationPolicyV1 policy(uint64_t id,uint64_t scene){return {1,id*10,1,scene,id+1000};}
struct Slot {
 PrExecutor* owner=nullptr;int epoll=-1,complete=-1,startup=-1;bool registered=false;
 void watches(){
  require(pr_executor_watch_fds(owner,&complete,&startup)==PR_EXECUTOR_OK&&complete>=0&&startup>=0&&complete!=startup,"borrowed distinct watches");
  const int complete_flags=fcntl(complete,F_GETFD),startup_flags=fcntl(startup,F_GETFD);require(complete_flags>=0&&startup_flags>=0&&(complete_flags&FD_CLOEXEC)&&(startup_flags&FD_CLOEXEC),"watch CLOEXEC");
  if(epoll<0){epoll=epoll_create1(EPOLL_CLOEXEC);require(epoll>=0,"owned epoll");}
  epoll_event event{};event.events=EPOLLIN;event.data.u32=1;require(epoll_ctl(epoll,EPOLL_CTL_ADD,complete,&event)==0,"register completion watch");event.data.u32=2;require(epoll_ctl(epoll,EPOLL_CTL_ADD,startup,&event)==0,"register BOOTTIME watch");registered=true;
 }
 void unwatch(){if(registered){require(epoll_ctl(epoll,EPOLL_CTL_DEL,complete,nullptr)==0&&epoll_ctl(epoll,EPOLL_CTL_DEL,startup,nullptr)==0,"unregister both borrowed watches");registered=false;}}
 uint32_t wait_watch(uint64_t cutoff){
  require(registered,"watches registered");for(;;){const auto t=now();require(t<cutoff,"watch deadline");epoll_event event{};const auto remaining=cutoff-t;const int ms=static_cast<int>(std::min<uint64_t>(50,remaining/1000000+(remaining%1000000!=0)));const int n=epoll_wait(epoll,&event,1,ms);if(n<0&&errno==EINTR)continue;require(n>=0,"epoll wait");if(n)return event.data.u32;}
 }
};
std::array<Slot,2> slots;
void start(Slot& slot,uint64_t id,PrNativeOwnerFunctionsV2& functions,PrOwnerOptionsV1& opts,uint64_t deadline){
 auto* input=pack(id);const auto status=pr_executor_create_v2(&functions,&opts,1,&input,deadline,&slot.owner);
 if(input)pr_bootstrap_release(input);
 require(status==PR_EXECUTOR_OK&&slot.owner&&!input,"executor accepts copied factory and consumes pack");
}
PrExecutorSnapshotV1 snapshot(Slot& slot){PrExecutorSnapshotV1 value{};require(pr_executor_snapshot_v1(slot.owner,&value)==PR_EXECUTOR_OK,"control snapshot");return value;}
void retained_release(Slot& slot){slot.unwatch();auto* original=slot.owner;require(pr_executor_try_release(&slot.owner)==PR_EXECUTOR_BUSY&&slot.owner==original,"Busy release retains exact owner");}
PrDiscoveryV1 ready(Slot& slot,uint64_t id){
 const auto cutoff=until();for(;;){PrDiscoveryV1 value{};const auto status=pr_executor_discovery_v1(slot.owner,&value);if(status==PR_EXECUTOR_OK){require(value.session_generation==id+1000&&value.output_count==1&&value.outputs[0].output_id==id*10,"distinct discovery");return value;}require(status==PR_EXECUTOR_EMPTY||status==PR_EXECUTOR_BUSY,"discovery refusal");slot.wait_watch(cutoff);}
}
void joined_release(Slot& slot,Probe& probe){
 const auto cutoff=std::min(close_until,now()+2*SECOND);for(;;){const auto status=pr_executor_try_join(slot.owner);if(status==PR_EXECUTOR_OK)break;require(status==PR_EXECUTOR_BUSY&&now()<cutoff,"actual bounded join");std::this_thread::yield();}
 const auto value=snapshot(slot);require(value.joined==1&&value.worker_alive==0,"actual joined snapshot");
 size_t count=0;const auto records=probe.copy(count);pid_t worker=0;bool destroyed=false;
 for(size_t i=0;i<count;++i){if(records[i].event==CREATED)worker=records[i].thread;if(records[i].event==DESTROY)destroyed=true;if(records[i].event!=CREATE_ENTER)require(records[i].thread!=control_tid,"callback on control");}
 require(worker>0&&destroyed,"actual destroy and worker identity");for(size_t i=0;i<count;++i)require(records[i].thread==worker,"all callbacks on same actual worker");
 // Task disappearance is separately observed after join, within the SAME
 // cutoff established before try_join; successful join alone is not this fact.
 char path[128];std::snprintf(path,sizeof(path),"/proc/self/task/%d",worker);uint64_t absent_at=0;
 while(now()<cutoff){const int status=access(path,F_OK);const int observed_errno=errno;const auto observed_at=now();
  if(status!=0&&observed_errno==ENOENT){require(observed_at<cutoff,"task absence observed before original join cutoff");absent_at=observed_at;break;}
  require(status==0,"worker task observation");std::this_thread::yield();
 }
 require(absent_at!=0,"joined worker task absent within original cutoff");
 std::printf("EXECUTOR_FFI_OBSERVATION worker_task_absent_after_join tid=%d boottime_ns=%llu\n",worker,static_cast<unsigned long long>(absent_at));
 slot.unwatch();require(pr_executor_try_release(&slot.owner)==PR_EXECUTOR_OK&&!slot.owner,"release only after actual join");
 if(slot.epoll>=0){require(close(slot.epoll)==0,"epoll close");slot.epoll=-1;}
}
void wrong_thread(Slot& slot){
 auto* payload=frame(A,7);auto* original=payload;std::exception_ptr failure;
 std::mutex mutex;std::condition_variable changed;bool done=false;
 std::thread caller([&]{try{
  auto s=sentinel<PrExecutorSnapshotV1>();const auto sb=bytes(s);auto d=sentinel<PrDiscoveryV1>();const auto db=bytes(d);auto c=sentinel<PrCompletionV1>();const auto cb=bytes(c);auto a=sentinel<PrAdmissionV1>();const auto ab=bytes(a);auto p=policy(A,7);int32_t first=-777,second=-888;auto* alias=slot.owner;
  require(pr_executor_snapshot_v1(slot.owner,&s)==PR_EXECUTOR_INVALID&&bytes(s)==sb,"wrong-thread snapshot output untouched");
  require(pr_executor_discovery_v1(slot.owner,&d)==PR_EXECUTOR_INVALID&&bytes(d)==db,"wrong-thread discovery untouched");
  require(pr_executor_watch_fds(slot.owner,&first,&second)==PR_EXECUTOR_INVALID&&first==-777&&second==-888,"wrong-thread FD outputs untouched");
  require(pr_executor_next_completion_v1(slot.owner,&c)==PR_EXECUTOR_INVALID&&bytes(c)==cb,"wrong-thread completion untouched");
  require(pr_executor_try_submit_v1(slot.owner,&payload,&p,work_until,7,&a)==PR_EXECUTOR_INVALID&&payload==original&&bytes(a)==ab,"wrong-thread Frame/admission retained");
  require(pr_executor_invalidate(slot.owner,2)==PR_EXECUTOR_INVALID&&pr_executor_stop(slot.owner)==PR_EXECUTOR_INVALID&&pr_executor_try_join(slot.owner)==PR_EXECUTOR_INVALID,"wrong-thread control refuses");
  require(pr_executor_try_release(&alias)==PR_EXECUTOR_INVALID&&alias==slot.owner,"wrong-thread owner slot retained");
 }catch(...){failure=std::current_exception();}{std::lock_guard lock(mutex);done=true;changed.notify_all();}});
 const auto cutoff=until();{std::unique_lock lock(mutex);while(!done){const auto t=now();if(t>=cutoff){std::fputs("EXECUTOR_FFI_FAIL wrong_thread_retained_process\n",stderr);std::fflush(stderr);std::_Exit(1);}changed.wait_for(lock,std::chrono::nanoseconds(cutoff-t));}}
 caller.join();if(failure){pr_frame_release(payload);std::rethrow_exception(failure);}ViewCheck check{7};require(pr_frame_with_view_v1(payload,inspect_frame,&check)==0&&check.checked,"refused exact Frame still valid");pr_frame_release(payload);
}
void before_ready(Slot& slot,uint64_t id){
 auto* payload=frame(id,1);auto* original=payload;auto result=sentinel<PrAdmissionV1>();const auto before=bytes(result);auto p=policy(id,1);
 const auto code=pr_executor_try_submit_v1(slot.owner,&payload,&p,work_until,99,&result);
 require(code==PR_EXECUTOR_BUSY&&payload==original&&bytes(result)==before,"before-ready Frame and output retained");pr_frame_release(payload);
 require(pr_executor_try_join(slot.owner)==PR_EXECUTOR_BUSY,"held bootstrap keeps join Busy");retained_release(slot);
 const auto entered=now();for(size_t i=0;i<1024;++i){const auto s=snapshot(slot);require(!s.ready&&s.worker_alive&&!s.accepted&&!s.occupied_credits,"held bootstrap control state");}require(now()-entered<2*SECOND,"bounded responsive held-bootstrap snapshots");
}
void submitted(Slot& slot,uint64_t id){
 auto* payload=frame(id,1);auto p=policy(id,1);PrAdmissionV1 admission{};
 require(pr_executor_try_submit_v1(slot.owner,&payload,&p,work_until,id+900,&admission)==PR_EXECUTOR_OK&&!payload&&admission.id>0,"accepted actual Frame transfer");
 const auto cutoff=until();std::array<bool,6> seen{};size_t count=0;
 while(!seen[PR_COMPLETION_RETIRED]){PrCompletionV1 c{};const auto status=pr_executor_next_completion_v1(slot.owner,&c);
  if(status==PR_EXECUTOR_EMPTY||status==PR_EXECUTOR_BUSY){slot.wait_watch(cutoff);continue;}
  require(status==PR_EXECUTOR_OK&&c.kind>=1&&c.kind<=5&&!seen[c.kind],"actual unique completion");seen[c.kind]=true;++count;
  require(c.admission.id==admission.id&&c.admission.control_cookie==id+900&&c.admission.policy.output_id==id*10&&c.error==0&&c.has_grant==1&&c.has_scope==1&&c.scope.objects[0].id==77&&c.scope.device_lease_id==id+30,"copied correlated completion");
  require(c.native.kind==PR_NATIVE_RETURNED&&c.native.operation==42&&c.native.succeeded==1&&c.native.extra_attempt_refused==1,"authored outcome preserved");
 }
 require(count==3&&seen[PR_COMPLETION_ATTEMPT]&&seen[PR_COMPLETION_OUTCOME]&&seen[PR_COMPLETION_RETIRED],"attempt/outcome/retirement separate");
}
void pending_shutdown(Slot& slot,Probe& probe){
 require(pr_executor_stop(slot.owner)==PR_EXECUTOR_OK,"control Stop");probe.wait(SHUTDOWN_PENDING,until());require(pr_executor_try_join(slot.owner)==PR_EXECUTOR_BUSY,"pending shutdown cannot join");retained_release(slot);
 size_t n=0;const auto records=probe.copy(n);for(size_t i=0;i<n;++i)require(records[i].event!=DESTROY,"no destroy before shutdown release");
 probe.open_shutdown();joined_release(slot,probe);
}
void stopped_refusal(Slot& slot,uint64_t id){
 auto* payload=frame(id,9);auto* original=payload;auto result=sentinel<PrAdmissionV1>();const auto prior=bytes(result);auto p=policy(id,9);
 require(pr_executor_try_submit_v1(slot.owner,&payload,&p,work_until,9,&result)==PR_EXECUTOR_STOPPING&&payload==original&&bytes(result)==prior,"Stop refuses without consuming exact Frame");pr_frame_release(payload);
 auto d=sentinel<PrDiscoveryV1>();const auto before=bytes(d);require(pr_executor_discovery_v1(slot.owner,&d)==PR_EXECUTOR_EMPTY&&bytes(d)==before,"late discovery unavailable and output preserved");require(!snapshot(slot).ready,"Stop never resurrects Ready");
}
void pass(const char* group){require(now()<work_until,"original fixture work cutoff");std::printf("EXECUTOR_FFI_PASS %s\n",group);std::fflush(stdout);}
void test(){
 probe_a.reset();probe_b.reset();auto functions_a=table(),functions_b=table(true);auto opts_a=options(A),opts_b=options(B);
 // Invalid table validation preserves an actual owned pack and empty owner slot.
 auto* input=pack(A);auto* original=input;PrExecutor* output=nullptr;auto invalid=functions_a;invalid.abi_version=1;
 require(pr_executor_create_v2(&invalid,&opts_a,1,&input,until(),&output)==PR_EXECUTOR_INVALID&&input==original&&!output,"invalid table preserves bootstrap/owner slots");pr_bootstrap_release(input);
 pass("abi_validation_original_bootstrap_retention");
 start(slots[0],A,functions_a,opts_a,until());start(slots[1],B,functions_b,opts_b,until());
 probe_a.wait(CREATE_ENTER,until());probe_b.wait(CREATE_ENTER,until());
 // Valid replacement callbacks make stale-table use deterministic, not a crash.
 functions_a=table(false,true);functions_b=table(true,true);opts_a=options(9001);opts_b=options(9002);
 probe_a.open(&Probe::release_create);probe_b.open(&Probe::release_create);
 probe_a.wait(BOOTSTRAP_ENTER,until());probe_b.wait(BOOTSTRAP_ENTER,until());
 before_ready(slots[0],A);before_ready(slots[1],B);wrong_thread(slots[0]);
 pass("two_factory_values_wrong_thread_and_before_ready_refusals");
 slots[0].watches();slots[1].watches();probe_a.open(&Probe::release_bootstrap);probe_b.open(&Probe::release_bootstrap);
 ready(slots[0],A);ready(slots[1],B);probe_a.signal();probe_b.signal();probe_a.wait(DISPATCH,until());probe_b.wait(DISPATCH,until());
 submitted(slots[0],A);submitted(slots[1],B);
 pending_shutdown(slots[0],probe_a);pending_shutdown(slots[1],probe_b);
 require(poisoned_calls.load()==0,"caller table replacement affected retained function table");
 pass("copied_dispatch_authored_scope_actual_destroy_and_join");
 // Separate explicit Stop while real C++ bootstrap is held.
 probe_a.reset();auto f=table();auto o=options(A);probe_a.open(&Probe::release_create);start(slots[0],A,f,o,until());probe_a.wait(BOOTSTRAP_ENTER,until());
 require(pr_executor_stop(slots[0].owner)==PR_EXECUTOR_OK,"Stop held bootstrap");require(pr_executor_try_join(slots[0].owner)==PR_EXECUTOR_BUSY,"held Stop retains worker");retained_release(slots[0]);
 probe_a.open(&Probe::release_bootstrap);probe_a.wait(BOOTSTRAP_RETURN,until());probe_a.wait(SHUTDOWN_PENDING,until());stopped_refusal(slots[0],A);probe_a.open_shutdown();joined_release(slots[0],probe_a);
 pass("Stop_late_bootstrap_success_never_ready");
 // Real CLOCK_BOOTTIME deadline FD, with no busy-wait/sleep as its oracle.
 probe_a.reset();f=table();o=options(A);probe_a.open(&Probe::release_create);const auto cutoff=now()+150000000ULL;start(slots[0],A,f,o,cutoff);slots[0].watches();probe_a.wait(BOOTSTRAP_ENTER,until());
 bool timer=false;const auto observation_until=until();while(!timer)timer=slots[0].wait_watch(observation_until)==2;
 require(now()>=cutoff,"actual original startup timer fired");const auto s=snapshot(slots[0]);require(s.stopping&&s.failed&&s.worker_alive&&!s.ready,"held bootstrap expires without dropping worker");require(pr_executor_try_join(slots[0].owner)==PR_EXECUTOR_BUSY,"expired held join Busy");retained_release(slots[0]);
 stopped_refusal(slots[0],A);probe_a.open(&Probe::release_bootstrap);probe_a.wait(BOOTSTRAP_RETURN,until());probe_a.wait(SHUTDOWN_PENDING,until());stopped_refusal(slots[0],A);probe_a.open_shutdown();joined_release(slots[0],probe_a);
 pass("real_BOOTTIME_watch_held_expiry_retains_context");
}
bool cleanup()noexcept{
 probe_a.all_open();probe_b.all_open();
 for(auto& slot:slots)if(slot.owner)pr_executor_stop(slot.owner);
 for(auto& slot:slots){
  if(slot.owner){bool joined=false;while(true){uint64_t t=0;try{t=now();}catch(...){return false;}if(t>=close_until)break;const auto code=pr_executor_try_join(slot.owner);if(code==PR_EXECUTOR_OK){joined=true;break;}if(code!=PR_EXECUTOR_BUSY)return false;std::this_thread::yield();}if(!joined)return false;try{slot.unwatch();}catch(...){return false;}if(pr_executor_try_release(&slot.owner)!=PR_EXECUTOR_OK||slot.owner)return false;}
  if(slot.epoll>=0){close(slot.epoll);slot.epoll=-1;}
 }
 return true;
}
}
int main(){
 try{control_tid=tid();const auto started=now();work_until=started+5*SECOND;close_until=started+8*SECOND;test();require(cleanup(),"success actual closure");std::puts("EXECUTOR_FFI_ALL_PASS");return 0;}
 catch(const std::exception& error){std::fprintf(stderr,"EXECUTOR_FFI_FAIL %s\n",error.what());std::fflush(stderr);if(!cleanup()){std::fputs("EXECUTOR_FFI_RETAINED_WORKER_PROCESS_EXIT_NO_JOIN_CLAIM\n",stderr);std::fflush(stderr);std::_Exit(1);}return 1;}
 catch(...){std::fputs("EXECUTOR_FFI_FAIL exception\n",stderr);std::fflush(stderr);if(!cleanup())std::_Exit(1);return 1;}
}
