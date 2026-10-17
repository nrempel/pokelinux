// Independent actual-kernel consumer. SOURCE ONLY; no production implementation.
// Fixture requests are not production Rust format-policy eligibility.
#include <aquamarine/allocator/Exact.hpp>
#include <aquamarine/allocator/GBM.hpp>
#include <aquamarine/backend/Backend.hpp>
#include <aquamarine/backend/DRM.hpp>
#include <aquamarine/backend/DisplayAttempt.hpp>
#include <aquamarine/backend/Session.hpp>
#include <aquamarine/output/Output.hpp>
#include <pokelinux_render_executor.h>
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm.h>
#include <drm_fourcc.h>
#include <gbm.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <fcntl.h>
#include <poll.h>
#include <set>
#include <stdexcept>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>
#include <vector>

using namespace Aquamarine;
using Hyprutils::Memory::CSharedPointer;
template<class T> using SP = CSharedPointer<T>;
namespace {
constexpr uint64_t SECOND = 1'000'000'000ULL;
std::atomic<bool> done{false};
std::atomic<int> outcome{2}; // 0 operation cases, 1 contract failure, 2 setup
std::atomic<pid_t> workerTid{0};
std::atomic<unsigned> completedGroups{0};
pid_t controlTid = 0;
uint64_t workUntil = 0;
struct Failure { const char* name; bool setup; };
uint64_t now() {
    timespec t{};
    if (clock_gettime(CLOCK_BOOTTIME, &t) || t.tv_sec < 0) throw Failure{"boottime", true};
    return uint64_t(t.tv_sec)*SECOND + uint64_t(t.tv_nsec);
}
void check(bool yes, const char* name, bool setup=false) {
    if (!yes) throw Failure{name,setup};
    if (now() >= workUntil) throw Failure{"original_work_deadline",true};
}
pid_t tid() { return static_cast<pid_t>(syscall(SYS_gettid)); }
void record(const char* group) {
    std::printf("EXACT_BUFFER_CASE {\"name\":\"%s\",\"passed\":true,\"worker_tid\":%d,\"at_ns\":%llu}\n",
        group,tid(),static_cast<unsigned long long>(now()));
    std::fflush(stdout);
    completedGroups.fetch_add(1);
}
uint32_t deny(void*, const SDisplayAttemptInfo*) noexcept { return 0; }
bool noDisplay(const CDisplayAttemptScope& s) {
    const auto& r=s.result();
    return s.bound() && r.info.kind==0 && !r.attempted && !r.additional_refused;
}
int duplicate(int fd) {
    if (fd<0) return -1;
    const auto out=fcntl(fd,F_DUPFD_CLOEXEC,0);
    check(out>=0,"owned_bootstrap_duplicate",true); return out;
}
SSessionRenderEvent metadata(const PrSessionRenderEventV1& in) {
    static_assert(sizeof(SSessionRenderEvent)==sizeof(PrSessionRenderEventV1));
    SSessionRenderEvent out; std::memcpy(&out,&in,sizeof(out)); return out;
}
bool sameRequest(const SExactBufferRequest& a,const SExactBufferRequest& b) {
    return a.request_id==b.request_id && a.mode==b.mode && a.modifier==b.modifier;
}
bool sameAttrs(const SDMABUFAttrs& a,const SDMABUFAttrs& b) {
    return a.success==b.success && a.size==b.size && a.format==b.format && a.modifier==b.modifier &&
        a.planes==b.planes && a.offsets==b.offsets && a.strides==b.strides && a.fds==b.fds;
}
void zeroImport(const SExactImport& r) {
    check(!r.reused && r.prime_count==0 && r.addfb.count==0 && !r.framebuffer_id,"refusal_has_no_import");
    for(const auto& p:r.prime) check(p.count==0,"refusal_has_no_prime");
}
struct Owner {
    const pid_t thread=tid();
    SP<CSession> mirror;
    SP<CBackend> backend;
    std::array<SP<IOutput>,PR_EXECUTOR_MAX_OUTPUTS> outputs;
    size_t outputCount=0;
    bool outputOverflow=false;
    Hyprutils::Signal::CHyprSignalListener listener;
    SP<CSwapchain> chain;
    SP<IBuffer> buffer;
    SP<CDRMFB> fb,reused;
};
// Independently resolve real native bus identity; primary/render rdevs may differ.
void deviceIdentity(int allocator,int kms) {
    drmDevicePtr a=nullptr,b=nullptr;
    errno=0;const int ar=drmGetDevice2(allocator,0,&a);const int ae=errno;
    errno=0;const int br=drmGetDevice2(kms,0,&b);const int be=errno;
    const int equal=(ar==0&&br==0&&a&&b)?drmDevicesEqual(a,b):0;
    struct stat as{},bs{};const int af=fstat(allocator,&as),bf=fstat(kms,&bs);
    std::printf("EXACT_BUFFER_DEVICE {\"allocator_fd\":%d,\"kms_fd\":%d,\"allocator_get\":[%d,%d],\"kms_get\":[%d,%d],\"devices_equal\":%d,\"allocator_rdev\":%llu,\"kms_rdev\":%llu}\n",
        allocator,kms,ar,ae,br,be,equal,static_cast<unsigned long long>(as.st_rdev),static_cast<unsigned long long>(bs.st_rdev));
    if(a) drmFreeDevice(&a);
    if(b) drmFreeDevice(&b);
    check(ar==0&&br==0&&equal==1&&af==0&&bf==0&&S_ISCHR(as.st_mode)&&S_ISCHR(bs.st_mode),"same_native_device",true);
}
void receiptsDevice(const SExactDeviceBinding& r,int allocator,int kms) {
    check(r.allocator_fd==allocator&&r.kms_fd==kms&&r.allocator_get_device.count==1&&r.kms_get_device.count==1&&
        r.allocator_get_device.result==0&&r.allocator_get_device.error==0&&r.kms_get_device.result==0&&r.kms_get_device.error==0&&r.devices_equal==1,"copied_actual_device_binding");
    struct stat a{},b{};
    check(fstat(allocator,&a)==0&&fstat(kms,&b)==0&&r.allocator_rdev==uint64_t(a.st_rdev)&&r.kms_rdev==uint64_t(b.st_rdev),"copied_device_rdevs");
}
void runCases(Owner& o,const PrBootstrapViewV1& input) {
    check(tid()==o.thread&&tid()!=controlTid,"actual_worker_identity",true);
    CDisplayAttemptScope guard(eDisplayAttemptDomain::RENDER,deny,&o);
    check(guard.bound(),"deny_scope",true);
    o.mirror=CSession::createRenderMirror(input.session_generation);
    check(bool(o.mirror),"real_render_mirror",true);
    uint64_t lease=0,generation=0,rdev=0,enumeratedSequence=0;
    for(uint32_t i=0;i<input.event_count;++i) {
        const auto& e=input.events[i];
        const auto copied=metadata(e.metadata);
        const auto result=o.mirror->applyRenderEvent(copied,duplicate(e.primary_fd),duplicate(e.render_fd));
        check(result==eSessionApply::APPLIED_METADATA,"real_prefix_apply",true);
        if(copied.type==AQ_SESSION_DEVICE_ADD) {
            check(!lease,"one_fixture_device",true);
            lease=copied.device_lease_id;generation=copied.device_generation;rdev=copied.rdev;
        }
        if(copied.type==AQ_SESSION_ENUMERATION_DONE) enumeratedSequence=copied.session_sequence;
    }
    check(lease&&generation&&rdev&&enumeratedSequence&&o.mirror->initialEnumerationComplete()&&o.mirror->active,"real_prefix_identity",true);
    SBackendOptions backendOptions;
    o.backend=CBackend::createRenderSessionHolder(backendOptions,o.mirror);
    check(bool(o.backend),"retained_real_backend",true);
    check(o.backend->initializeRenderSession(),"native_backend_initialize",true);
    o.listener=o.backend->events.newOutput.listen([&o](const SP<IOutput>& output){
        if(o.outputCount==o.outputs.size()) {o.outputOverflow=true;return;}
        o.outputs[o.outputCount++]=output;
    });
    check(o.backend->start()&&o.backend->primaryAllocator&&!o.outputOverflow&&o.outputCount==1,"native_backend_start_one_output",true);
    auto output=o.outputs[0];
    auto implementation=output->getBackend();
    auto drm=Hyprutils::Memory::dynamicPointerCast<CDRMBackend>(implementation);
    auto nativeOutput=Hyprutils::Memory::dynamicPointerCast<CDRMOutput>(output);
    check(drm&&nativeOutput&&implementation->type()==AQ_BACKEND_DRM,"real_drm_output",true);
    const int kms=implementation->drmFD(),allocator=o.backend->primaryAllocator->drmFD();
    deviceIdentity(allocator,kms);
    struct stat device{};
    check(fstat(kms,&device)==0&&uint64_t(device.st_rdev)==rdev&&nativeOutput->getConnectorID()>0,"bound_real_kms_fd",true);
    uint64_t cap=0;errno=0;const int capResult=drmGetCap(kms,DRM_CAP_ADDFB2_MODIFIERS,&cap);const int capError=errno;
    std::printf("EXACT_BUFFER_CONTEXT {\"session\":%llu,\"enumeration_sequence\":%llu,\"lease\":%llu,\"device_generation\":%llu,\"rdev\":%llu,\"connector\":%d,\"cap_return\":%d,\"cap_errno\":%d,\"cap_value\":%llu}\n",
        (unsigned long long)input.session_generation,(unsigned long long)enumeratedSequence,(unsigned long long)lease,
        (unsigned long long)generation,(unsigned long long)rdev,nativeOutput->getConnectorID(),capResult,capError,(unsigned long long)cap);
    check(capResult==0&&cap==0&&noDisplay(guard),"real_cap0_fixture",true);
    SSwapchainOptions options;
    options.length=1;options.size={64,64};options.format=DRM_FORMAT_XRGB8888;
    options.scanout=true;options.cursor=false;options.multigpu=false;options.scanoutOutput=output;
    SExactBufferRequest request{input.session_generation,eExactBufferMode::IMPLICIT,0};
    o.chain=CSwapchain::create(o.backend->primaryAllocator,implementation);
    check(bool(o.chain),"real_swapchain",true);
    std::vector<SExactAllocation> attempts;
    const bool configured=o.chain->reconfigureExact(options,request,attempts);
    for(const auto& a:attempts) {
        std::printf("EXACT_BUFFER_ALLOCATION_ATTEMPT {\"result\":%u,\"buffer_id\":%llu,\"api\":%u,\"flags\":%u,\"modifier_count\":%u,\"create_count\":%u,\"returned_buffer\":%s,\"create_errno\":%d,\"getter_count\":%u,\"raw_modifier\":%llu,\"export_success\":%s}\n",
            uint32_t(a.result),(unsigned long long)a.buffer_id,uint32_t(a.api),a.flags,a.modifier_count,a.create_count,
            a.returned_buffer?"true":"false",a.create_error,a.modifier_getter_count,(unsigned long long)a.raw_modifier,a.exported.success?"true":"false");
    }
    // A real GBM/driver failure is retained setup/unsupported, not substituted.
    check(configured,"actual_implicit_allocation",true);
    check(attempts.size()==1,"one_buffer_one_attempt");
    const auto allocation=attempts.front();
    check(allocation.result==eExactBufferResult::SUCCESS&&sameRequest(allocation.request,request)&&allocation.buffer_id&&
        allocation.api==eExactAllocationAPI::GBM_BO_CREATE&&allocation.create_count==1&&allocation.returned_buffer&&
        allocation.create_error==0&&allocation.flags==(GBM_BO_USE_RENDERING|GBM_BO_USE_SCANOUT)&&allocation.modifier_count==0&&
        allocation.modifier_getter_count==1,"exact_implicit_dispatch");
    receiptsDevice(allocation.device,allocator,kms);
    check(allocation.options.length==1&&allocation.options.size==options.size&&allocation.options.format==options.format&&
        allocation.options.scanout&&!allocation.options.cursor&&!allocation.options.multigpu&&allocation.options.scanoutOutput==output,"all_exact_options");
    o.buffer=o.chain->next(nullptr);
    auto gbm=Hyprutils::Memory::dynamicPointerCast<CGBMBuffer>(o.buffer);
    check(bool(gbm)&&gbm->good(),"real_gbm_buffer",true);
    const auto retained=gbm->exactAllocation();
    const auto before=gbm->dmabuf();
    check(retained.buffer_id==allocation.buffer_id&&retained.raw_modifier==allocation.raw_modifier&&retained.modifier_getter_count==1&&
        retained.api==allocation.api&&sameRequest(retained.request,request)&&sameAttrs(retained.exported,before)&&
        sameAttrs(allocation.exported,before)&&before.success&&before.planes>0&&before.planes<=4&&before.size==options.size&&before.format==options.format,
        "same_retained_buffer_raw_export");
    std::printf("EXACT_BUFFER_ALLOCATION {\"buffer_id\":%llu,\"raw_modifier\":%llu,\"exported_modifier\":%llu,\"planes\":%d,\"create_count\":%u,\"getter_count\":%u}\n",
        (unsigned long long)allocation.buffer_id,(unsigned long long)allocation.raw_modifier,(unsigned long long)before.modifier,
        before.planes,allocation.create_count,allocation.modifier_getter_count);
    for(int i=0;i<before.planes;++i) {
        check(before.fds[i]>=0&&fcntl(before.fds[i],F_GETFD)>=0&&before.strides[i]>0,"real_export_plane",true);
        std::printf("EXACT_BUFFER_EXPORT_PLANE {\"plane\":%d,\"fd\":%d,\"pitch\":%u,\"offset\":%u}\n",i,before.fds[i],before.strides[i],before.offsets[i]);
    }
    record("actual_implicit_allocation_raw_export");

    auto explicitRequest=request;explicitRequest.request_id++;explicitRequest.mode=eExactBufferMode::EXPLICIT;explicitRequest.modifier=DRM_FORMAT_MOD_LINEAR;
    attempts.clear();
    check(!o.chain->reconfigureExact(options,explicitRequest,attempts)&&attempts.size()==1,"cap0_explicit_refused");
    const auto refusal=attempts.front();
    std::printf("EXACT_BUFFER_REFUSAL {\"result\":%u,\"cap_return\":%d,\"cap_value\":%llu,\"api\":%u,\"create_count\":%u,\"getter_count\":%u}\n",
        uint32_t(refusal.result),refusal.modifiers_cap.result,(unsigned long long)refusal.modifiers_cap_value,
        uint32_t(refusal.api),refusal.create_count,refusal.modifier_getter_count);
    check(refusal.result==eExactBufferResult::MODIFIERS_UNAVAILABLE&&sameRequest(refusal.request,explicitRequest)&&
        refusal.modifiers_cap.count==1&&refusal.modifiers_cap.result==0&&refusal.modifiers_cap_value==0&&
        refusal.api==eExactAllocationAPI::NONE&&refusal.create_count==0&&!refusal.returned_buffer&&refusal.modifier_getter_count==0&&
        !refusal.buffer_id&&!refusal.exported.success,"cap0_no_alternative_operation");
    check(o.chain->next(nullptr)==o.buffer&&sameAttrs(before,gbm->dmabuf()),"cap0_refusal_preserves_positive_swapchain_buffer");
    record("actual_cap0_explicit_refusal_no_retry");

    for(unsigned which=0;which<3;++which) {
        auto wrong=request;
        if(which==0) ++wrong.request_id;
        if(which==1) wrong.mode=eExactBufferMode::EXPLICIT;
        if(which==2) wrong.modifier=1;
        SExactImport rejected;
        auto extra=CDRMFB::createExact(o.buffer,drm,wrong,rejected);
        std::printf("EXACT_BUFFER_MISMATCH {\"which\":%u,\"result\":%u,\"prime_count\":%u,\"addfb_count\":%u}\n",which,uint32_t(rejected.result),rejected.prime_count,rejected.addfb.count);
        check(!extra&&(rejected.result==eExactBufferResult::REQUEST_MISMATCH||
            (which==2&&rejected.result==eExactBufferResult::INVALID_REQUEST)),"mismatched_request_refused");
        zeroImport(rejected);check(sameAttrs(before,gbm->dmabuf()),"mismatch_export_unchanged");
    }
    record("positive_buffer_request_mismatch_refusals");

    SExactImport imported;
    o.fb=CDRMFB::createExact(o.buffer,drm,request,imported);
    std::printf("EXACT_BUFFER_IMPORT {\"result\":%u,\"buffer_id\":%llu,\"framebuffer_id\":%u,\"flags\":%u,\"with_modifiers\":%s,\"prime_count\":%u,\"addfb_count\":%u,\"addfb_return\":%d,\"addfb_errno\":%d}\n",
        uint32_t(imported.result),(unsigned long long)imported.buffer_id,imported.framebuffer_id,imported.addfb_flags,
        imported.with_modifiers?"true":"false",imported.prime_count,imported.addfb.count,imported.addfb.result,imported.addfb.error);
    for(unsigned i=0;i<std::min(imported.prime_count,4u);++i) std::printf("EXACT_BUFFER_PRIME {\"plane\":%u,\"count\":%u,\"return\":%d,\"errno\":%d,\"handle\":%u}\n",
        i,imported.prime[i].count,imported.prime[i].result,imported.prime[i].error,imported.handles[i]);
    check(bool(o.fb),"actual_addfb2_import",true);
    check(imported.result==eExactBufferResult::SUCCESS&&sameRequest(imported.request,request)&&imported.buffer_id==allocation.buffer_id&&
        !imported.reused&&imported.prime_count==uint32_t(before.planes)&&imported.addfb.count==1&&imported.addfb.result==0&&
        imported.addfb.error==0&&!imported.with_modifiers&&imported.addfb_flags==0&&imported.framebuffer_id&&o.fb->id==imported.framebuffer_id&&
        sameAttrs(imported.exported,before),"exact_implicit_addfb2_flags0");
    receiptsDevice(imported.device,allocator,kms);
    for(int i=0;i<before.planes;++i) check(imported.prime[i].count==1&&imported.prime[i].result==0&&imported.handles[i],"actual_prime_returns");
    errno=0;auto* readback=drmModeGetFB2(kms,o.fb->id);const int getError=errno;
    if(!readback) std::printf("EXACT_BUFFER_GETFB2_FAILURE {\"errno\":%d}\n",getError);
    check(readback!=nullptr,"actual_getfb2",true);
    // Close every unique fresh GETFB2 GEM handle even on metadata assertion failure.
    bool metadataEqual=readback->width==64&&readback->height==64&&readback->pixel_format==before.format;
    std::set<uint32_t> freshHandles;
    for(int i=0;i<4;++i) {
        const bool plane=i<before.planes;
        metadataEqual=metadataEqual&&(plane?(readback->pitches[i]==before.strides[i]&&readback->offsets[i]==before.offsets[i]):readback->pitches[i]==0);
        if(readback->handles[i]) freshHandles.insert(readback->handles[i]);
        std::printf("EXACT_BUFFER_GETFB2_PLANE {\"plane\":%d,\"handle\":%u,\"pitch\":%u,\"offset\":%u}\n",
            i,readback->handles[i],readback->pitches[i],readback->offsets[i]);
    }
    std::printf("EXACT_BUFFER_GETFB2 {\"framebuffer_id\":%u,\"width\":%u,\"height\":%u,\"format\":%u,\"flags\":%u,\"modifier\":%llu,\"unique_fresh_handles\":%zu,\"errno\":%d}\n",
        o.fb->id,readback->width,readback->height,readback->pixel_format,readback->flags,(unsigned long long)readback->modifier,freshHandles.size(),getError);
    drmModeFreeFB2(readback);
    bool closed=true;
    for(const auto handle:freshHandles) {drm_gem_close c{};c.handle=handle;errno=0;const auto rc=drmIoctl(kms,DRM_IOCTL_GEM_CLOSE,&c);const auto error=errno;
        std::printf("EXACT_BUFFER_GETFB2_HANDLE {\"handle\":%u,\"return\":%d,\"errno\":%d}\n",handle,rc,error);closed=closed&&rc==0;}
    check(metadataEqual&&closed,"kernel_fb_metadata_and_fresh_handle_close");
    SExactImport reuse;
    o.reused=CDRMFB::createExact(o.buffer,drm,request,reuse);
    check(o.reused==o.fb&&reuse.result==eExactBufferResult::SUCCESS&&reuse.reused&&reuse.buffer_id==allocation.buffer_id&&
        reuse.framebuffer_id==o.fb->id&&reuse.prime_count==0&&reuse.addfb.count==0,"same_exact_attachment_reuse_no_import");
    check(sameAttrs(before,gbm->dmabuf()),"import_export_metadata_unchanged");
    record("actual_implicit_import_getfb2_same_buffer");

    const auto savedId=o.fb->id;
    const auto close=o.fb->closeExact();
    std::printf("EXACT_BUFFER_CLOSE {\"request_id\":%llu,\"buffer_id\":%llu,\"framebuffer_id\":%u,\"count\":%u,\"return\":%d,\"errno\":%d}\n",
        (unsigned long long)close.request_id,(unsigned long long)close.buffer_id,close.framebuffer_id,close.closefb.count,close.closefb.result,close.closefb.error);
    // On uncertainty Owner keeps fb/buffer/chain retained; no RMFB or hidden retry.
    check(close.request_id==request.request_id&&close.buffer_id==allocation.buffer_id&&close.framebuffer_id==savedId&&
        close.closefb.count==1&&close.closefb.result==0&&close.closefb.error==0,"actual_owned_closefb",true);
    const auto again=o.fb->closeExact();
    check(again.closefb.count==1&&again.closefb.result==close.closefb.result&&again.closefb.error==close.closefb.error&&
        again.framebuffer_id==savedId,"close_result_copy_no_retry");
    errno=0;auto* absent=drmModeGetFB2(kms,savedId);const int absentError=errno;
    if(absent) {drmModeFreeFB2(absent);throw Failure{"closed_fb_still_registered",false};}
    std::printf("EXACT_BUFFER_REGISTRATION_ABSENT {\"framebuffer_id\":%u,\"errno\":%d}\n",savedId,absentError);
    check(absentError==ENOENT,"closed_registration_absent",true);
    check(sameAttrs(before,gbm->dmabuf()),"close_does_not_relabel_export");
    o.fb->closeHandles();
    o.reused.reset();o.fb.reset();o.chain.reset();o.buffer.reset();gbm.reset();
    for(int i=0;i<before.planes;++i) {
        errno=0;const auto closed=fcntl(before.fds[i],F_GETFD);const auto error=errno;
        std::printf("EXACT_BUFFER_EXPORT_CLOSED {\"plane\":%d,\"fd\":%d,\"return\":%d,\"errno\":%d}\n",i,before.fds[i],closed,error);
        check(closed==-1&&error==EBADF,"export_fd_closed_on_buffer_release");
    }
    check(noDisplay(guard),"no_display_request_for_resource_operations");
    record("owned_never_attached_close_and_buffer_release");
}
void* create(const PrOwnerOptionsV1*) noexcept {
    try {auto* o=new Owner;workerTid.store(o->thread);return o;} catch(...) {return nullptr;}
}
void destroy(void* p) noexcept {
    // Whole AQ/backend retirement is not implemented/claimed by this fixture.
    auto* o=static_cast<Owner*>(p);if(o&&(o->backend||o->mirror)) std::terminate();delete o;
}
int32_t bootstrap(void* p,const PrBootstrapViewV1* in,PrDiscoveryV1*) noexcept {
    try {
        check(p&&in&&in->events&&in->event_count&&in->event_count<=PR_EXECUTOR_MAX_BOOTSTRAP_EVENTS,"real_borrowed_prefix",true);
        runCases(*static_cast<Owner*>(p),*in);outcome.store(0);
    } catch(const Failure& e) {
        outcome.store(e.setup?2:1);std::printf("EXACT_BUFFER_FAILURE {\"name\":\"%s\",\"setup\":%s}\n",e.name,e.setup?"true":"false");
    } catch(...) {outcome.store(2);std::puts("EXACT_BUFFER_FAILURE {\"name\":\"exception\",\"setup\":true}");}
    std::fflush(stdout);done.store(true);
    // Deliberate fixture result: no backend Ready/frame/discovery publication.
    return PR_EXECUTOR_NATIVE;
}
int32_t prepare(void*,const PrFrame*,const PrAdmissionV1*) noexcept {return PR_EXECUTOR_PROTOCOL;}
int32_t execute(void*,const PrFrame*,const PrAdmissionV1*,PrAttempt*,PrNativeResultV1*) noexcept {return PR_EXECUTOR_PROTOCOL;}
int32_t advance(void*,const PrRetirementContextV1*,PrNativeProgressV1*) noexcept {return PR_EXECUTOR_PROTOCOL;}
int32_t sources(void*,PrPollSourceV1*,uint32_t,uint32_t* n) noexcept {if(!n)return PR_EXECUTOR_INVALID;*n=0;return PR_EXECUTOR_OK;}
int32_t dispatch(void*,const PrNativeReadyV1*) noexcept {return PR_EXECUTOR_PROTOCOL;}
int32_t shutdown(void*,uint32_t* n) noexcept {if(!n)return PR_EXECUTOR_INVALID;*n=0;return PR_EXECUTOR_OK;}
int32_t change(void*,const PrSessionRenderEventV1*,PrDiscoveryV1*) noexcept {return PR_EXECUTOR_PROTOCOL;}
struct Control {PrExecutor* executor=nullptr;uint64_t session=0;bool revoked=false;};
uint32_t revoke(void* p,uint64_t session,uint32_t) noexcept {
    auto& c=*static_cast<Control*>(p);c.revoked=true;
    if(session!=c.session||!c.executor) return AQ_REVOKE_UNAVAILABLE;
    return pr_executor_stop(c.executor)==PR_EXECUTOR_OK?AQ_REVOKED:AQ_REVOKE_UNAVAILABLE;
}
}
int main(int argc,char**) {
    Control c;PrBootstrap* pack=nullptr;
    try {
        controlTid=tid();const auto start=now();workUntil=start+35*SECOND;c.session=start;
        check(argc==1&&start,"no_arguments",true);
        SInputSessionOptions opts;opts.sessionGeneration=c.session;opts.revokeRender=revoke;opts.controlContext=&c;
        auto broker=CSession::attemptInputBroker(opts);
        check(broker&&!broker->transportFailed()&&!c.revoked,"actual_input_broker",true);
        std::array<SSessionPendingEvent,PR_EXECUTOR_MAX_BOOTSTRAP_EVENTS> batch{};size_t n=0;
        check(broker->peekInitialRenderEvents(batch.data(),batch.size(),&n)==eSessionInitialRenderEventsResult::READY,"actual_initial_prefix",true);
        std::array<PrBootstrapEventV1,PR_EXECUTOR_MAX_BOOTSTRAP_EVENTS> copy{};
        for(size_t i=0;i<n;++i) {std::memcpy(&copy[i].metadata,&batch[i].metadata,sizeof(copy[i].metadata));copy[i].primary_fd=batch[i].primary_fd;copy[i].render_fd=batch[i].render_fd;}
        check(pr_bootstrap_capture_v1(c.session,copy.data(),uint32_t(n),&pack)==PR_EXECUTOR_OK&&pack,"actual_rust_capture",true);
        for(size_t i=0;i<n;++i) check(broker->markRenderEventAdmitted(batch[i].metadata.session_sequence),"mark_only_after_rust_custody",true);
        check(!broker->peekPendingRenderEvent()&&!c.revoked,"initial_prefix_no_suffix",true);
        PrNativeOwnerFunctionsV2 f{};f.abi_version=2;f.struct_size=sizeof(f);f.create=create;f.destroy=destroy;f.bootstrap=bootstrap;
        f.prepare=prepare;f.execute=execute;f.advance=advance;f.poll_sources=sources;f.dispatch=dispatch;f.shutdown_progress=shutdown;f.apply_device_change=change;
        PrOwnerOptionsV1 o{};o.abi_version=2;o.struct_size=sizeof(o);o.owner_id=start;
        check(pr_executor_create_v2(&f,&o,c.session,&pack,start+20*SECOND,&c.executor)==PR_EXECUTOR_OK&&c.executor&&!pack,"actual_rust_worker",true);
        int complete=-1,timer=-1;
        check(pr_executor_watch_fds(c.executor,&complete,&timer)==PR_EXECUTOR_OK&&complete>=0&&timer>=0&&complete!=timer,"actual_both_watches",true);
        while(!done.load()) {
            check(now()<workUntil,"original_component_cutoff",true);
            std::vector<pollfd> watched{{complete,POLLIN,0},{timer,POLLIN,0}};
            const auto brokerPolls=broker->pollFDs();
            for(const auto& p:brokerPolls) watched.push_back({p->fd,POLLIN,0});
            const auto rc=poll(watched.data(),watched.size(),2);if(rc<0&&errno==EINTR)continue;
            check(rc>=0,"owned_watch_poll",true);
            for(size_t i=0;i<brokerPolls.size();++i) if(watched[i+2].revents) brokerPolls[i]->onSignal();
            if(watched[1].revents) {PrExecutorSnapshotV1 s{};(void)pr_executor_snapshot_v1(c.executor,&s);}
            check(!c.revoked&&!broker->transportFailed()&&!broker->peekPendingRenderEvent(),"no_unhandled_authority_change",true);
        }
        check(workerTid.load()>0&&workerTid.load()!=controlTid,"observed_distinct_worker",true);
        check(now()<start+20*SECOND,"actual_operations_within_original_startup_cutoff",true);
        const auto stopped=pr_executor_stop(c.executor);
        check(stopped==PR_EXECUTOR_OK,"actual_terminal_stop",true);
        std::printf("EXACT_BUFFER_RESULT {\"operation_cases_passed\":%s,\"groups\":%d,\"control_tid\":%d,\"worker_tid\":%d,\"stop_result\":%d,\"whole_native_retirement\":false,\"joined\":false,\"process_disposal_required\":true,\"at_ns\":%llu}\n",
            outcome.load()==0?"true":"false",int(completedGroups.load()),controlTid,workerTid.load(),stopped,(unsigned long long)now());
        std::fflush(stdout);
        // No blocking Rust Drop/destructor, detached IO or false joined claim.
        // External owned parent must directly wait this process, EOF and close VM.
        std::_Exit(outcome.load());
    } catch(const Failure& e) {
        std::printf("EXACT_BUFFER_FAILURE {\"name\":\"%s\",\"setup\":%s}\n",e.name,e.setup?"true":"false");
    } catch(...) {std::puts("EXACT_BUFFER_FAILURE {\"name\":\"control_exception\",\"setup\":true}");}
    if(c.executor) (void)pr_executor_stop(c.executor);
    if(pack) pr_bootstrap_release(pack);
    std::fflush(stdout);std::_Exit(2);
}
