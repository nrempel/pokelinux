// Test-only future real NGL behavioral actor. Proposed APIs have NO implementation yet.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/render/gl/OwnerFramePrimitives.hpp>
#include <hyprland/src/render/gl/OwnerFrameResources.hpp>
#include <hyprland/src/render/Shader.hpp>
#include <hyprland/src/render/ShaderLoader.hpp>
#include "pokelinux_render_frame.h"
#include <EGL/eglext.h>
#include <gbm.h>
#include <linux/sync_file.h>
#include <poll.h>
#include <sys/ioctl.h>
#include <chrono>
#include <exception>
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <limits>
#include <memory>
#include <map>
#include <stdexcept>
#include <string>
#include <thread>
#include <type_traits>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

using namespace Render;
using namespace Render::GL;
namespace {
constexpr uint32_t WIDTH=8, HEIGHT=6, AR24=0x34325241, XR24=0x34325258;
uint64_t boot() {
    timespec t{};
    if (clock_gettime(CLOCK_BOOTTIME,&t)||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000L)
        throw std::runtime_error("BOOTTIME unavailable");
    const auto s=static_cast<uint64_t>(t.tv_sec);
    if(s>(std::numeric_limits<uint64_t>::max()-static_cast<uint64_t>(t.tv_nsec))/1000000000ULL)
        throw std::runtime_error("BOOTTIME overflow");
    return s*1000000000ULL+static_cast<uint64_t>(t.tv_nsec);
}
uint64_t decimal(const char* name) {
    const char* text=std::getenv(name);
    if(!text||!*text)throw std::runtime_error("missing actor configuration");
    uint64_t n=0;
    for(const char* p=text;*p;++p){
        if(*p<'0'||*p>'9'||n>(std::numeric_limits<uint64_t>::max()-static_cast<unsigned>(*p-'0'))/10)
            throw std::runtime_error("invalid actor configuration");
        n=n*10+static_cast<unsigned>(*p-'0');
    }
    return n;
}
std::string path(const char* name,size_t limit) {
    const char* p=std::getenv(name);
    if(!p||p[0]!='/'||strnlen(p,limit+1)>limit)throw std::runtime_error("invalid actor path");
    return p;
}
struct Inputs {
    int fd=-1;
    uint64_t run=0;
    std::string node,shaders;
    PrFrame* frame=nullptr;
    uint64_t producer_dropped=0,producer_tid=0,resource=1,until=0,fenceSequence=0,commandSequence=0;
    uint32_t format=AR24;
    std::thread worker;
    std::atomic<bool> done=false;
    ~Inputs(){
        if(frame)pr_frame_release(frame);
        if(fd>=0)close(fd);
    }
};
std::unique_ptr<Inputs> actor;
void emit(Inputs& i,const char* test,const char* status,uint64_t value=0,uint64_t entered=0,uint64_t returned=0) {
    char line[2048];
    const int n=std::snprintf(line,sizeof(line),
        "{\"schema\":1,\"authority\":\"OFFSCREEN_FIXTURE_IDS_ONLY\",\"run\":%llu,\"case\":\"%s\",\"status\":\"%s\",\"collection_boottime_ns\":%llu,\"operation_enter_ns\":%llu,\"operation_return_ns\":%llu,\"tid\":%llu,\"admission_id\":%llu,\"scene_sequence\":%llu,\"security_generation\":1,\"output_id\":1,\"output_generation\":1,\"resource_id\":%llu,\"resource_generation\":%llu,\"commit_sequence\":%llu,\"fence_sequence\":%llu,\"command_sequence\":%llu,\"value\":%llu}",
        static_cast<unsigned long long>(i.run),test,status,static_cast<unsigned long long>(boot()),
        static_cast<unsigned long long>(entered),static_cast<unsigned long long>(returned),
        static_cast<unsigned long long>(syscall(SYS_gettid)),static_cast<unsigned long long>(i.run),static_cast<unsigned long long>(i.resource),
        static_cast<unsigned long long>(i.resource),static_cast<unsigned long long>(i.resource),static_cast<unsigned long long>(i.resource),
        static_cast<unsigned long long>(i.fenceSequence),static_cast<unsigned long long>(i.commandSequence),static_cast<unsigned long long>(value));
    if(n<=0||static_cast<size_t>(n)>=sizeof(line)||send(i.fd,line,static_cast<size_t>(n),MSG_DONTWAIT|MSG_NOSIGNAL)!=n)
        throw std::runtime_error("actor receipt unavailable");
}
std::string jsonText(const char* raw){
    if(!raw||strnlen(raw,256)>255)throw std::runtime_error("bounded GL profile unavailable");
    std::string result;
    for(const unsigned char* p=reinterpret_cast<const unsigned char*>(raw);*p;++p){
        if(*p<32||*p>126)throw std::runtime_error("non-ASCII GL profile");
        if(*p=='"'||*p=='\\')result.push_back('\\');
        result.push_back(static_cast<char>(*p));
    }
    return result;
}
void profile(Inputs& i){
    const auto vendor=jsonText(reinterpret_cast<const char*>(glGetString(GL_VENDOR)));
    const auto renderer=jsonText(reinterpret_cast<const char*>(glGetString(GL_RENDERER)));
    const auto version=jsonText(reinterpret_cast<const char*>(glGetString(GL_VERSION)));
    const auto text=std::string{"{\"schema\":1,\"case\":\"GL_PROFILE\",\"run\":"}+std::to_string(i.run)+
        ",\"tid\":"+std::to_string(syscall(SYS_gettid))+",\"collection_boottime_ns\":"+std::to_string(boot())+
        ",\"vendor\":\""+vendor+"\",\"renderer\":\""+renderer+"\",\"version\":\""+version+"\"}";
    if(text.size()>2048||send(i.fd,text.data(),text.size(),MSG_DONTWAIT|MSG_NOSIGNAL)!=static_cast<ssize_t>(text.size()))
        throw std::runtime_error("GL profile receipt unavailable");
}
bool extension(const char* list,const char* wanted) {
    if(!list)return false;
    const size_t n=std::strlen(wanted);
    for(const char* p=list;(p=std::strstr(p,wanted));p+=n)
        if((p==list||p[-1]==' ')&&(p[n]==' '||p[n]=='\0'))return true;
    return false;
}
struct EGLFixture {
    int fd=-1;gbm_device* gbm=nullptr;
    EGLDisplay display=EGL_NO_DISPLAY;EGLContext context=EGL_NO_CONTEXT;
    GLuint fb=0,color=0;
    ~EGLFixture(){
        if(display!=EGL_NO_DISPLAY&&context!=EGL_NO_CONTEXT){
            if(eglMakeCurrent(display,EGL_NO_SURFACE,EGL_NO_SURFACE,context)){
                if(fb)glDeleteFramebuffers(1,&fb);
                if(color)glDeleteTextures(1,&color);
            }
            eglMakeCurrent(display,EGL_NO_SURFACE,EGL_NO_SURFACE,EGL_NO_CONTEXT);
            eglDestroyContext(display,context);
        }
        if(display!=EGL_NO_DISPLAY)eglTerminate(display);
        if(gbm)gbm_device_destroy(gbm);
        if(fd>=0)close(fd);
    }
    void finish(){
        if(display==EGL_NO_DISPLAY||context==EGL_NO_CONTEXT||eglGetCurrentDisplay()!=display||eglGetCurrentContext()!=context)
            throw std::runtime_error("wrong cleanup owner context");
        if(fb){const auto old=fb;glDeleteFramebuffers(1,&fb);fb=0;if(glIsFramebuffer(old))throw std::runtime_error("FBO not deleted");}
        if(color){const auto old=color;glDeleteTextures(1,&color);color=0;if(glIsTexture(old))throw std::runtime_error("FBO texture not deleted");}
        if(glGetError()!=GL_NO_ERROR)throw std::runtime_error("GL cleanup failed");
        if(!eglMakeCurrent(display,EGL_NO_SURFACE,EGL_NO_SURFACE,EGL_NO_CONTEXT))throw std::runtime_error("EGL unset failed");
        if(!eglDestroyContext(display,context))throw std::runtime_error("EGL destroy failed");
        context=EGL_NO_CONTEXT;
        if(!eglTerminate(display))throw std::runtime_error("EGL terminate failed");
        display=EGL_NO_DISPLAY;
        if(!eglReleaseThread())throw std::runtime_error("EGL release thread failed");
        if(gbm){gbm_device_destroy(gbm);gbm=nullptr;}
        if(fd>=0){const int owned=fd;fd=-1;if(close(owned))throw std::runtime_error("render FD close failed");}
    }
    uint64_t init(const std::string& node){
        fd=open(node.c_str(),O_RDWR|O_CLOEXEC|O_NOFOLLOW);
        struct stat st{};
        if(fd<0||fstat(fd,&st)||!S_ISCHR(st.st_mode))throw std::runtime_error("render node unavailable");
        gbm=gbm_create_device(fd);if(!gbm)throw std::runtime_error("GBM unavailable");
        auto getDisplay=reinterpret_cast<PFNEGLGETPLATFORMDISPLAYEXTPROC>(eglGetProcAddress("eglGetPlatformDisplayEXT"));
        if(!getDisplay)throw std::runtime_error("GBM EGL platform unavailable");
        display=getDisplay(EGL_PLATFORM_GBM_KHR,gbm,nullptr);
        EGLint major=0,minor=0;
        if(display==EGL_NO_DISPLAY||!eglInitialize(display,&major,&minor)||!eglBindAPI(EGL_OPENGL_ES_API))
            throw std::runtime_error("EGL initialization failed");
        const char* exts=eglQueryString(display,EGL_EXTENSIONS);
        if(!extension(exts,"EGL_KHR_no_config_context")||!extension(exts,"EGL_KHR_surfaceless_context"))
            throw std::runtime_error("explicit fixture EGL capability unavailable");
        const EGLint attrs[]={EGL_CONTEXT_CLIENT_VERSION,3,EGL_NONE};
        context=eglCreateContext(display,EGL_NO_CONFIG_KHR,EGL_NO_CONTEXT,attrs);
        if(context==EGL_NO_CONTEXT||!eglMakeCurrent(display,EGL_NO_SURFACE,EGL_NO_SURFACE,context))
            throw std::runtime_error("worker context unavailable");
        glGenTextures(1,&color);glBindTexture(GL_TEXTURE_2D,color);
        glTexStorage2D(GL_TEXTURE_2D,1,GL_RGBA8,WIDTH,HEIGHT);
        glGenFramebuffers(1,&fb);glBindFramebuffer(GL_FRAMEBUFFER,fb);
        glFramebufferTexture2D(GL_FRAMEBUFFER,GL_COLOR_ATTACHMENT0,GL_TEXTURE_2D,color,0);
        if(!color||!fb||glCheckFramebufferStatus(GL_FRAMEBUFFER)!=GL_FRAMEBUFFER_COMPLETE||glGetError()!=GL_NO_ERROR)
            throw std::runtime_error("real RGBA8 FBO unavailable");
        return static_cast<uint64_t>(st.st_rdev);
    }
};
void capture(Inputs& i,uint32_t format=AR24,uint64_t resourceId=1){
    i.resource=resourceId;i.format=format;i.producer_tid=static_cast<uint64_t>(syscall(SYS_gettid));
    constexpr size_t BYTES=32*3;
    auto* bytes=static_cast<uint8_t*>(mmap(nullptr,BYTES,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0));
    if(bytes==MAP_FAILED)throw std::runtime_error("producer mapping");
    std::memset(bytes,0xA5,BYTES);
    for(uint32_t y=0;y<3;++y)for(uint32_t x=0;x<5;++x){
        const uint8_t pixel[4]={static_cast<uint8_t>((x&1)?255:0),static_cast<uint8_t>((y&1)?255:0),static_cast<uint8_t>(((x+y)&1)?255:0),static_cast<uint8_t>(format==AR24?255:0x39)};
        std::memcpy(bytes+y*32+x*4,pixel,4);
    }
    PrSnapshotStore* store=pr_snapshot_store_create(4096);
    PrSnapshot* snapshot=nullptr;
    const PrShmDescription desc{resourceId,resourceId,resourceId,format,5,3,32,4,0};
    const int32_t imported=store?pr_snapshot_capture(store,&desc,bytes,BYTES,&snapshot):static_cast<int32_t>(PR_ALLOCATION);
    const std::array<float,9> identity={1,0,0,0,1,0,0,0,1};
    std::array<PrFrameDrawV1,3> draws{};
    for(auto& d:draws){d.resource_index=UINT32_MAX;d.scissor_count=1;d.material.alpha=1;d.material.rounding_power=2;std::copy(identity.begin(),identity.end(),d.projection);}
    draws[0].kind=PR_DRAW_CLEAR;draws[0].scissor_first=2;draws[0].converted_color[0]=1;draws[0].converted_color[3]=1;
    draws[1].kind=PR_DRAW_RECT;draws[1].scissor_first=3;draws[1].box={1,1,5,3,0};
    draws[1].converted_color[1]=1;draws[1].converted_color[3]=1;draws[1].color_srgb[1]=1;draws[1].color_srgb[3]=1;
    const std::array<float,9> rectMatrix={1.25F,0,-0.75F,0,1,-2.0F/3.0F,0,0,1};
    std::copy(rectMatrix.begin(),rectMatrix.end(),draws[1].projection);
    draws[2].kind=PR_DRAW_SHM;draws[2].resource_index=0;draws[2].scissor_first=4;draws[2].box={1,1,5,3,0};
    draws[2].uv[2]=1;draws[2].uv[3]=1;draws[2].surface_id=1;draws[2].commit_sequence=resourceId;draws[2].material.flags=PR_MAT_NEAREST;
    std::copy(rectMatrix.begin(),rectMatrix.end(),draws[2].projection);
    const std::array<PrFrameRectV1,5> rects={{{0,0,8,6},{0,0,8,6},{0,0,8,6},{2,2,5,4},{2,1,4,4}}};
    const PrFrameResourceBorrowV1 resource{desc,snapshot};
    PrFrameBorrowV1 frame{};frame.abi_version=1;frame.struct_size=sizeof(frame);frame.flags=PR_FRAME_COMMIT|PR_FRAME_FULL_REPAINT;
    frame.scene_sequence=resourceId;frame.security_generation=1;
    frame.target.output_id=1;frame.target.output_generation=1;frame.target.width=WIDTH;frame.target.height=HEIGHT;
    frame.target.transformed_width=WIDTH;frame.target.transformed_height=HEIGHT;frame.target.scale=1;
    std::copy(identity.begin(),identity.end(),frame.target.target_projection);std::copy(identity.begin(),identity.end(),frame.target.scale_matrix);
    frame.damage_count=1;frame.final_damage_first=1;frame.final_damage_count=1;
    frame.draw_count=draws.size();frame.resource_count=1;frame.rect_count=rects.size();frame.draws=draws.data();frame.resources=&resource;frame.rects=rects.data();
    const int32_t status=imported==PR_OK?pr_frame_capture_v1(&frame,&i.frame):static_cast<int32_t>(PR_FRAME_ALLOCATION);
    if(snapshot)pr_snapshot_release(snapshot);
    if(store)pr_snapshot_store_release(store);
    std::memset(bytes,0x5A,BYTES);const int unmapped=munmap(bytes,BYTES);
    if(status!=PR_FRAME_OK||!i.frame||unmapped)throw std::runtime_error("actual immutable frame capture failed");
    i.producer_dropped=boot();
}
SFrameDraw leaf(const PrFrameBorrowV1& frame,size_t index){
    const auto& s=frame.draws[index];SFrameDraw d;
    d.kind=static_cast<eFrameDrawKind>(s.kind);d.box={s.box.x,s.box.y,s.box.width,s.box.height,s.box.rotation};
    d.material.alpha=s.material.alpha;d.material.roundingPower=s.material.rounding_power;d.material.blend=(s.material.flags&PR_MAT_BLEND)!=0;
    d.material.nearest=(s.material.flags&PR_MAT_NEAREST)!=0;d.material.rgbx=(s.material.flags&PR_MAT_RGBX)!=0;
    d.resourceIndex=s.resource_index;d.surfaceId=s.surface_id;d.commitSequence=s.commit_sequence;
    std::copy_n(s.uv,4,d.uv.begin());
    std::copy_n(s.projection,9,d.projection.begin());std::copy_n(s.converted_color,4,d.convertedColor.begin());std::copy_n(s.color_srgb,4,d.colorSRGB.begin());
    if(s.scissor_count!=1||s.scissor_first>=frame.rect_count)throw std::runtime_error("unexpected control scissor");
    const auto& r=frame.rects[s.scissor_first];d.scissors.push_back({r.x1,r.y1,r.x2,r.y2});return d;
}
SFrameTarget target(const PrFrameTargetV1& s){
    SFrameTarget t;t.outputId=s.output_id;t.outputGeneration=s.output_generation;t.width=s.width;t.height=s.height;
    t.transformedWidth=s.transformed_width;t.transformedHeight=s.transformed_height;t.outputTransform=s.output_transform;t.scale=s.scale;
    std::copy_n(s.target_projection,9,t.targetProjection.begin());std::copy_n(s.scale_matrix,9,t.scaleMatrix.begin());return t;
}
uint64_t compare(EGLFixture& egl,bool rectangle){
    std::array<uint8_t,WIDTH*HEIGHT*4> pixels{};
    glBindFramebuffer(GL_READ_FRAMEBUFFER,egl.fb);glPixelStorei(GL_PACK_ALIGNMENT,1);
    glReadPixels(0,0,WIDTH,HEIGHT,GL_RGBA,GL_UNSIGNED_BYTE,pixels.data());
    if(glGetError()!=GL_NO_ERROR)throw std::runtime_error("readback failed");
    uint64_t mismatches=0;
    for(uint32_t y=0;y<HEIGHT;++y)for(uint32_t x=0;x<WIDTH;++x){
        const bool green=rectangle&&x>=2&&x<5&&y>=2&&y<4;
        const std::array<uint8_t,4> expected=green?std::array<uint8_t,4>{0,255,0,255}:std::array<uint8_t,4>{255,0,0,255};
        for(size_t c=0;c<4;++c)if(pixels[(y*WIDTH+x)*4+c]!=expected[c])++mismatches;
    }
    return mismatches;
}

void require(bool value,const char* reason){if(!value)throw std::runtime_error(reason);}
bool same(const PrShmDescription& a,const PrShmDescription& b){
    return a.resource_id==b.resource_id&&a.generation==b.generation&&a.commit_sequence==b.commit_sequence&&
        a.drm_format==b.drm_format&&a.width==b.width&&a.height==b.height&&a.stride==b.stride&&
        a.bytes_per_pixel==b.bytes_per_pixel&&a.reserved==b.reserved;
}
uint8_t pattern(uint32_t format,size_t index){
    const auto y=index/32,offset=index%32;
    if(offset>=20)return 0xA5;
    const auto x=offset/4,c=offset%4;
    const std::array<uint8_t,4> p={static_cast<uint8_t>((x&1)?255:0),static_cast<uint8_t>((y&1)?255:0),
        static_cast<uint8_t>(((x+y)&1)?255:0),static_cast<uint8_t>(format==AR24?255:0x39)};
    return p[c];
}
void packState(){
    glBindBuffer(GL_PIXEL_PACK_BUFFER,0);glPixelStorei(GL_PACK_ALIGNMENT,1);
    glPixelStorei(GL_PACK_ROW_LENGTH,0);glPixelStorei(GL_PACK_SKIP_PIXELS,0);glPixelStorei(GL_PACK_SKIP_ROWS,0);
}
uint64_t compareShm(EGLFixture& egl){
    std::array<uint8_t,WIDTH*HEIGHT*4> pixels{};glBindFramebuffer(GL_READ_FRAMEBUFFER,egl.fb);packState();
    glReadPixels(0,0,WIDTH,HEIGHT,GL_RGBA,GL_UNSIGNED_BYTE,pixels.data());
    require(glGetError()==GL_NO_ERROR,"SHM readback failed");uint64_t mismatches=0;
    for(uint32_t y=0;y<HEIGHT;++y)for(uint32_t x=0;x<WIDTH;++x){
        std::array<uint8_t,4> p={255,0,0,255};
        if(x>=2&&x<5&&y>=2&&y<4)p={0,255,0,255};
        if(x>=2&&x<4&&y>=1&&y<4){
            const auto sx=x-1,sy=y-1;
            p={static_cast<uint8_t>(((sx+sy)&1)?255:0),static_cast<uint8_t>((sy&1)?255:0),static_cast<uint8_t>((sx&1)?255:0),255};
        }
        for(size_t c=0;c<4;++c)if(pixels[(y*WIDTH+x)*4+c]!=p[c])++mismatches;
    }
    return mismatches;
}
struct SetupUnavailable : std::runtime_error {using std::runtime_error::runtime_error;};
struct WorkerState {
    Inputs& inputs;
    EGLFixture egl;
    CShader quad,rgba,rgbx;
    std::unique_ptr<COwnerFramePrimitives> primitives;
    std::unique_ptr<COwnerShmTexture> texture;
    std::unique_ptr<COwnerNativeFence> fence;
    GLuint unpack=0;
    EGLContext alternate=EGL_NO_CONTEXT;
    int duplicate=-1,capabilityFD=-1;
    EGLSyncKHR capabilitySync=EGL_NO_SYNC_KHR;
    bool helperAcquisitionStarted=false;
    const char* activeCase="SETUP";
    uint64_t observed=0;
    std::exception_ptr failure;
    explicit WorkerState(Inputs& i):inputs(i){}
};
SOwnerFenceTag nextTag(Inputs& i){
    require(i.fenceSequence<UINT64_MAX,"fence sequence overflow");++i.fenceSequence;
    return {i.run,i.resource,1,1,1,i.fenceSequence};
}
SOwnerFenceFD borrowed(WorkerState& s){
    SOwnerFenceFD result;require(s.fence->borrowedFD(result)==eOwnerResourceStatus::OK,"actual exported fence unavailable");
    require(result.fd>=0&&result.tag==SOwnerFenceTag{s.inputs.run,s.inputs.resource,1,1,1,s.inputs.fenceSequence},"fence correlation mismatch");
    const int flags=fcntl(result.fd,F_GETFD);require(flags>=0&&(flags&FD_CLOEXEC),"native fence not CLOEXEC");return result;
}
void syncInfo(int fd,int32_t& status,uint32_t& count){
    sync_file_info info{};
    // num_fences=0 requests aggregate numeric status/count, never names/points.
    int rc;do{rc=ioctl(fd,SYNC_IOC_FILE_INFO,&info);}while(rc<0&&errno==EINTR);
    require(rc==0,"exported FD is not a queryable sync_file");status=info.status;count=info.num_fences;
}
uint64_t awaitFence(WorkerState& s,const SOwnerFenceFD& expected){
    const auto entered=boot();int32_t status=0;uint32_t count=0;
    syncInfo(expected.fd,status,count);require(count>0&&status>=0,"native fence failed or empty");
    for(;;){
        const auto now=boot();require(now<s.inputs.until,"absolute actor deadline before fence completion");
        const uint64_t remaining=s.inputs.until-now;
        const int timeout=static_cast<int>(std::min<uint64_t>(50,remaining/1000000+(remaining%1000000!=0)));
        pollfd p{expected.fd,POLLIN,0};const int rc=poll(&p,1,timeout);
        if(rc<0&&errno==EINTR)continue;
        require(rc>=0&&!(p.revents&(POLLERR|POLLHUP|POLLNVAL)),"native fence poll failed");
        syncInfo(expected.fd,status,count);require(count>0&&status>=0,"native fence signaled error");
        if(rc>0&&(p.revents&POLLIN)&&status>0)break;
    }
    const auto returned=boot();require(returned<s.inputs.until,"fence observation exceeded original deadline");
    const auto after=borrowed(s);require(after.fd==expected.fd&&after.tag==expected.tag,"fence changed during observation");
    emit(s.inputs,"FENCE_FACT","SYNC_FILE_SUCCESS_OBSERVED",count,entered,returned);
    s.observed=returned;return returned;
}
void qualifyNativeFence(WorkerState& s){
    // Independent actual backend fixture qualification, not helper success.
    // Acquisitions are saved immediately in the retained worker state.
    const auto create=reinterpret_cast<PFNEGLCREATESYNCKHRPROC>(eglGetProcAddress("eglCreateSyncKHR"));
    const auto duplicate=reinterpret_cast<PFNEGLDUPNATIVEFENCEFDANDROIDPROC>(eglGetProcAddress("eglDupNativeFenceFDANDROID"));
    const auto destroy=reinterpret_cast<PFNEGLDESTROYSYNCKHRPROC>(eglGetProcAddress("eglDestroySyncKHR"));
    if(!create||!duplicate||!destroy)throw SetupUnavailable("native fence procedures absent");
    const auto entered=boot();s.capabilitySync=create(s.egl.display,EGL_SYNC_NATIVE_FENCE_ANDROID,nullptr);
    if(s.capabilitySync==EGL_NO_SYNC_KHR)throw SetupUnavailable("actual native fence creation unavailable");
    glFlush();s.capabilityFD=duplicate(s.egl.display,s.capabilitySync);
    if(s.capabilityFD<0||glGetError()!=GL_NO_ERROR)throw SetupUnavailable("actual native fence export unavailable");
    const int flags=fcntl(s.capabilityFD,F_GETFD);
    if(flags<0||fcntl(s.capabilityFD,F_SETFD,flags|FD_CLOEXEC)<0)throw SetupUnavailable("native fence CLOEXEC unavailable");
    int32_t status=0;uint32_t count=0;
    try{
        for(;;){
            syncInfo(s.capabilityFD,status,count);if(status<0||count==0)throw SetupUnavailable("native fixture fence failed");
            const auto now=boot();if(now>=s.inputs.until)throw SetupUnavailable("native fence fixture deadline");
            const auto remaining=s.inputs.until-now;
            pollfd p{s.capabilityFD,POLLIN,0};const int timeout=static_cast<int>(std::min<uint64_t>(50,remaining/1000000+(remaining%1000000!=0)));
            const int rc=poll(&p,1,timeout);if(rc<0&&errno==EINTR)continue;
            if(rc<0||(p.revents&(POLLERR|POLLHUP|POLLNVAL)))throw SetupUnavailable("native fixture poll unavailable");
            syncInfo(s.capabilityFD,status,count);
            if(status<0||count==0)throw SetupUnavailable("native fixture sync_file failed");
            if(rc>0&&(p.revents&POLLIN)&&status>0)break;
        }
    }catch(...){throw SetupUnavailable("actual native sync_file capability not qualified");}
    const auto returned=boot();if(returned>=s.inputs.until)throw SetupUnavailable("native fixture observation late");
    // Only actual positive sync_file+poll observation permits this local cleanup.
    if(!destroy(s.egl.display,s.capabilitySync))throw SetupUnavailable("native fixture sync cleanup failed");
    s.capabilitySync=EGL_NO_SYNC_KHR;
    const int owned=s.capabilityFD;s.capabilityFD=-1;
    if(close(owned)!=0)throw SetupUnavailable("native fixture FD close failed");
    emit(s.inputs,"NATIVE_FENCE_CAPABILITY","ACTUAL_SYNC_FILE_POLL_SUCCESS",count,entered,returned);
}
void issueAndObserve(WorkerState& s){
    const auto tag=nextTag(s.inputs);const auto entered=boot();
    require(s.fence->issue(tag)==eOwnerResourceStatus::OK,"native fence issue/export failed");
    const auto returned=boot();emit(s.inputs,"FENCE_ISSUE","EXPORTED_NOT_COMPLETION",0,entered,returned);
    awaitFence(s,borrowed(s));
}
void retireFence(WorkerState& s){
    require(s.observed!=0,"actor requires actual fence fact before retirement");
    require(s.fence->retire()==eOwnerResourceStatus::OK,"correct-owner fence retirement failed");
    SOwnerFenceFD output{123,{9,8,7,6,5,4}};const auto before=output;
    require(s.fence->borrowedFD(output)==eOwnerResourceStatus::NOT_READY&&output.fd==before.fd&&output.tag==before.tag,"retired fence borrow mutated output");
    s.observed=0;
}
void provePixels(WorkerState& s,const char* test,uint64_t entered){
    require(s.observed!=0&&s.observed<=boot(),"readback lacks preceding successful fence fact");
    const auto readEntered=boot();const auto mismatch=compareShm(s.egl);const auto readReturned=boot();
    require(s.observed<=readEntered,"readback preceded fence observation");
    emit(s.inputs,"READBACK",mismatch?"PIXELS_DIFFER":"EXACT_RGBA_192_BYTES",mismatch,readEntered,readReturned);
    require(mismatch==0,"SHM pixel/padding/scissor oracle failed");emit(s.inputs,test,"PASS",0,entered,readReturned);
}
SOwnerTextureInfo textureInfo(WorkerState& s,const PrShmDescription& desc){
    SOwnerTextureInfo result;require(s.texture->describe(result)==eOwnerResourceStatus::OK,"actual texture description unavailable");
    require(same(result.description,desc)&&result.textureName>0&&result.hasAlpha==(desc.drm_format==AR24),"actual texture import descriptor/type mismatch");
    require(glIsTexture(result.textureName)==GL_TRUE,"owner texture name is not live");return result;
}
void preserve(WorkerState& s,const PrSnapshot* snapshot,const SOwnerTextureInfo& texture,const SOwnerFenceFD& fence){
    const auto after=textureInfo(s,texture.description);const auto fd=borrowed(s);
    require(after.textureName==texture.textureName&&after.hasAlpha==texture.hasAlpha&&fd.fd==fence.fd&&fd.tag==fence.tag,"refusal changed retained resources");
    require(s.texture->matches(snapshot)==eOwnerResourceStatus::OK,"refusal changed allocation binding");
}
std::array<GLint,7> unpackState(){
    std::array<GLint,7> state{};
    const std::array<GLenum,7> names={GL_ACTIVE_TEXTURE,GL_TEXTURE_BINDING_2D,GL_PIXEL_UNPACK_BUFFER_BINDING,GL_UNPACK_ALIGNMENT,GL_UNPACK_ROW_LENGTH,GL_UNPACK_SKIP_PIXELS,GL_UNPACK_SKIP_ROWS};
    for(size_t n=0;n<names.size();++n)glGetIntegerv(names[n],&state[n]);
    return state;
}
void upload(WorkerState& s,const PrSnapshot* snapshot){
    // Valid nondefault state deliberately detects stale PBO/row/skip assumptions.
    if(!s.unpack){glGenBuffers(1,&s.unpack);require(s.unpack!=0,"unpack PBO allocation failed");}
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER,s.unpack);glBufferData(GL_PIXEL_UNPACK_BUFFER,512,nullptr,GL_STATIC_DRAW);
    glActiveTexture(GL_TEXTURE3);glBindTexture(GL_TEXTURE_2D,s.egl.color);
    glPixelStorei(GL_UNPACK_ALIGNMENT,8);glPixelStorei(GL_UNPACK_ROW_LENGTH,13);
    glPixelStorei(GL_UNPACK_SKIP_PIXELS,1);glPixelStorei(GL_UNPACK_SKIP_ROWS,1);
    const auto before=unpackState();require(glGetError()==GL_NO_ERROR,"unpack negative-control setup failed");
    s.helperAcquisitionStarted=true;
    require(s.texture->upload(snapshot)==eOwnerResourceStatus::OK,"actual owner SHM upload failed");
    require(before==unpackState()&&glGetError()==GL_NO_ERROR,"SHM upload failed to restore unpack/texture state");
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER,0);glPixelStorei(GL_UNPACK_ALIGNMENT,4);glPixelStorei(GL_UNPACK_ROW_LENGTH,0);
    glPixelStorei(GL_UNPACK_SKIP_PIXELS,0);glPixelStorei(GL_UNPACK_SKIP_ROWS,0);glBindTexture(GL_TEXTURE_2D,0);glActiveTexture(GL_TEXTURE0);
}
void draw(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program){
    const auto key=COwnerFramePrimitives::shaderKey(d,s.inputs.format==AR24);require(key.has_value(),"SHM shader key unavailable");
    require(s.inputs.commandSequence<UINT64_MAX,"command sequence overflow");++s.inputs.commandSequence;
    const auto entered=boot();const auto result=s.primitives->drawShm(output,d,snapshot,*s.texture,SOwnerFrameProgram{program,*key});const auto returned=boot();
    require(result==eOwnerFramePrimitiveStatus::COMMANDS_ISSUED,"SHM primitive refused valid actual import");
    emit(s.inputs,"SHM_DRAW","COMMANDS_ISSUED_NOT_COMPLETION",0,entered,returned);
}
void wrongOwnerCalls(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program){
    const auto wrong=eOwnerResourceStatus::WRONG_OWNER_CONTEXT;const auto key=COwnerFramePrimitives::shaderKey(d,false);require(key.has_value(),"negative test shader key");
    SOwnerTextureInfo info{{11,12,13,AR24,1,1,4,4,0},99,true};const auto initial=info;
    SOwnerFenceFD fd{42,{11,12,13,14,15,16}};const auto fdBefore=fd;
    require(s.texture->upload(snapshot)==wrong&&s.texture->describe(info)==wrong&&s.texture->matches(snapshot)==wrong&&s.texture->retire()==wrong,"wrong-owner texture operation did not refuse");
    require(same(info.description,initial.description)&&info.textureName==initial.textureName&&info.hasAlpha==initial.hasAlpha,"wrong-owner describe changed output");
    const SOwnerFenceTag tag{s.inputs.run,s.inputs.resource,1,1,1,s.inputs.fenceSequence+1};
    require(s.fence->issue(tag)==wrong&&s.fence->borrowedFD(fd)==wrong&&s.fence->retire()==wrong,"wrong-owner fence operation did not refuse");
    require(fd.fd==fdBefore.fd&&fd.tag==fdBefore.tag,"wrong-owner fence changed output");
    require(s.primitives->drawShm(output,d,snapshot,*s.texture,SOwnerFrameProgram{program,*key})==eOwnerFramePrimitiveStatus::WRONG_OWNER_CONTEXT,"wrong-owner draw did not refuse");
}
void wrongThread(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program,const PrShmDescription& desc){
    s.activeCase="NGL-05";const auto entered=boot();const auto info=textureInfo(s,desc);const auto fd=borrowed(s);
    std::exception_ptr failure;uint64_t tid=0;
    std::thread negative([&]{try{tid=static_cast<uint64_t>(syscall(SYS_gettid));wrongOwnerCalls(s,output,d,snapshot,program);}catch(...){failure=std::current_exception();}});
    negative.join();if(failure)std::rethrow_exception(failure);
    require(tid!=static_cast<uint64_t>(syscall(SYS_gettid)),"wrong-thread test did not change thread");
    emit(s.inputs,"WRONG_THREAD","REFUSED_WITHOUT_TRANSFER",tid,entered,boot());preserve(s,snapshot,info,fd);
    retireFence(s);issueAndObserve(s);provePixels(s,"NGL-05",entered);
}
void wrongContext(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program,const PrShmDescription& desc){
    const auto entered=boot();const auto info=textureInfo(s,desc);const auto fd=borrowed(s);
    s.activeCase="NGL-06";const EGLint attrs[]={EGL_CONTEXT_CLIENT_VERSION,3,EGL_NONE};
    s.alternate=eglCreateContext(s.egl.display,EGL_NO_CONFIG_KHR,EGL_NO_CONTEXT,attrs);
    require(s.alternate!=EGL_NO_CONTEXT&&s.alternate!=s.egl.context,"second actual context unavailable");
    require(eglMakeCurrent(s.egl.display,EGL_NO_SURFACE,EGL_NO_SURFACE,s.alternate),"second context make-current failed");
    GLint bound=0;glGetIntegerv(GL_TEXTURE_BINDING_2D,&bound);require(glGetError()==GL_NO_ERROR,"second context state unavailable");
    wrongOwnerCalls(s,output,d,snapshot,program);
    GLint after=0;glGetIntegerv(GL_TEXTURE_BINDING_2D,&after);require(bound==after&&glGetError()==GL_NO_ERROR,"wrong-context call changed alternate state");
    require(eglGetCurrentContext()==s.alternate,"wrong-context helper moved current context");
    require(eglMakeCurrent(s.egl.display,EGL_NO_SURFACE,EGL_NO_SURFACE,s.egl.context),"original context restore failed");
    require(eglDestroyContext(s.egl.display,s.alternate),"alternate context destroy failed");s.alternate=EGL_NO_CONTEXT;
    preserve(s,snapshot,info,fd);retireFence(s);issueAndObserve(s);provePixels(s,"NGL-06",entered);
}
void mismatch(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program,const PrShmDescription& desc){
    const auto entered=boot();const auto info=textureInfo(s,desc);const auto fd=borrowed(s);
    s.activeCase="NGL-07";PrSnapshot* clone=pr_snapshot_clone(snapshot);require(clone!=nullptr,"same-allocation explicit clone failed");
    const auto cloneStatus=s.texture->matches(clone);pr_snapshot_release(clone);
    require(cloneStatus==eOwnerResourceStatus::OK,"actual same-allocation clone rejected");
    std::array<uint8_t,96> bytes{};bytes.fill(0xCC);auto* store=pr_snapshot_store_create(96);PrSnapshot* other=nullptr;
    const auto captured=store?pr_snapshot_capture(store,&desc,bytes.data(),bytes.size(),&other):static_cast<int32_t>(PR_ALLOCATION);
    if(store)pr_snapshot_store_release(store);
    bytes.fill(0x55);
    require(captured==PR_OK&&other,"different-allocation actual snapshot capture failed");
    PrSnapshotView originalView{},otherView{};
    const bool distinct=pr_snapshot_view(snapshot,&originalView)==PR_OK&&pr_snapshot_view(other,&otherView)==PR_OK&&originalView.pixels!=otherView.pixels;
    const auto key=COwnerFramePrimitives::shaderKey(d,false);require(key.has_value(),"mismatch shader key");
    const auto match=s.texture->matches(other);
    const auto rejected=s.primitives->drawShm(output,d,other,*s.texture,SOwnerFrameProgram{program,*key});pr_snapshot_release(other);
    require(distinct&&match==eOwnerResourceStatus::RESOURCE_MISMATCH&&rejected==eOwnerFramePrimitiveStatus::RESOURCE_MISMATCH,"forged import identity accepted");
    auto wrong=d;++wrong.commitSequence;
    require(s.primitives->drawShm(output,wrong,snapshot,*s.texture,SOwnerFrameProgram{program,*key})==eOwnerFramePrimitiveStatus::RESOURCE_MISMATCH,"draw commit mismatch accepted");
    preserve(s,snapshot,info,fd);retireFence(s);issueAndObserve(s);provePixels(s,"NGL-07",entered);
}
void retireTexture(WorkerState& s,const PrShmDescription& desc){
    const auto info=textureInfo(s,desc);require(s.texture->retire()==eOwnerResourceStatus::OK,"correct-owner texture retirement failed");
    require(glIsTexture(info.textureName)==GL_FALSE,"retired texture name remains live");
    SOwnerTextureInfo output{{11,12,13,AR24,1,1,4,4,0},99,true};const auto before=output;
    require(s.texture->describe(output)==eOwnerResourceStatus::NOT_READY&&same(output.description,before.description)&&output.textureName==before.textureName&&output.hasAlpha==before.hasAlpha,"retired texture describe mutated output");
}
void finalFence(WorkerState& s,const SOwnerFrameTarget& output,const SFrameDraw& d,const PrSnapshot* snapshot,CShader& program,const PrShmDescription& desc){
    s.activeCase="NGL-08";const auto entered=boot();retireFence(s);draw(s,output,d,snapshot,program);issueAndObserve(s);
    const auto fd=borrowed(s);s.duplicate=fcntl(fd.fd,F_DUPFD_CLOEXEC,3);require(s.duplicate>=0,"explicit fence duplicate failed");
    int32_t status=0;uint32_t count=0;syncInfo(s.duplicate,status,count);require(status>0&&count>0,"duplicate has no actual successful sync_file fact");
    // Actual independent completion fact already precedes this readback.
    provePixels(s,"FINAL_PIXEL_ORACLE",entered);retireFence(s);
    syncInfo(s.duplicate,status,count);require(status>0&&count>0,"retirement invalidated separately owned sync_file");
    const int owned=s.duplicate;s.duplicate=-1;require(close(owned)==0,"explicit duplicate close failed");
    retireTexture(s,desc);emit(s.inputs,"NGL-08","PASS_COMPLETION_THEN_LOCAL_RETIREMENT_NO_PRESENTATION",0,entered,boot());
}
struct Visit {WorkerState& state;bool first;};
int32_t visit(const PrFrameBorrowV1* v,void* raw) noexcept {
    auto& c=*static_cast<Visit*>(raw);auto& s=c.state;auto& i=s.inputs;
    try{
        require(v&&v->abi_version==1&&v->struct_size==sizeof(*v)&&v->draw_count==3&&v->resource_count==1&&v->rect_count==5,"actual retained frame shape mismatch");
        require(v->draws&&v->resources&&v->rects&&v->draws[2].resource_index==0,"actual frame resource index invalid");
        const auto& resource=v->resources[0];PrSnapshotView source{};
        require(resource.snapshot&&pr_snapshot_view(resource.snapshot,&source)==PR_OK&&source.pixels&&source.pixel_bytes==96,"retained snapshot view unavailable");
        const PrShmDescription expected{i.resource,i.resource,i.resource,i.format,5,3,32,4,0};
        require(same(source.description,expected)&&same(resource.description,expected),"actual retained immutable descriptor mismatch");
        uint64_t differences=0;for(size_t n=0;n<96;++n)if(source.pixels[n]!=pattern(i.format,n))++differences;
        emit(i,"CUSTODY_BYTES",differences?"BYTES_DIFFER":"EXACT_96_BYTES_AND_PADDING",differences);require(differences==0,"retained CPU pixel/padding custody failed");
        require(i.producer_dropped>0&&i.producer_dropped<=boot(),"producer drop observation missing");
        emit(i,"CUSTODY_PRODUCER","SOURCE_SNAPSHOT_STORE_MAPPING_DROPPED",i.producer_tid, i.producer_dropped,boot());
        const auto t=target(v->target);const SOwnerFrameTarget output{t,s.egl.fb};const auto clear=leaf(*v,0),rect=leaf(*v,1),shm=leaf(*v,2);
        auto entered=boot();require(s.primitives->drawClear(output,clear)==eOwnerFramePrimitiveStatus::COMMANDS_ISSUED,"CLEAR command failed");
        if(c.first){packState();const auto bad=compare(s.egl,false);emit(i,"NGL-01",bad?"CONTROL_FAILED":"PASS",bad,entered,boot());require(!bad,"actual CLEAR control pixels differ");}
        const auto key=COwnerFramePrimitives::shaderKey(rect);require(key.has_value(),"RECT shader key unavailable");
        entered=boot();require(s.primitives->drawRect(output,rect,SOwnerFrameProgram{s.quad,*key})==eOwnerFramePrimitiveStatus::COMMANDS_ISSUED,"RECT command failed");
        if(c.first){packState();const auto bad=compare(s.egl,true);emit(i,"NGL-02",bad?"CONTROL_FAILED":"PASS",bad,entered,boot());require(!bad,"actual RECT control pixels differ");}
        s.activeCase=c.first?"NGL-03":"NGL-04";entered=boot();upload(s,resource.snapshot);textureInfo(s,expected);require(s.texture->matches(resource.snapshot)==eOwnerResourceStatus::OK,"genuine import binding refused");
        auto& program=c.first?s.rgba:s.rgbx;draw(s,output,shm,resource.snapshot,program);issueAndObserve(s);provePixels(s,c.first?"NGL-03":"NGL-04",entered);
        if(c.first){retireFence(s);retireTexture(s,expected);}
        else{wrongThread(s,output,shm,resource.snapshot,program,expected);wrongContext(s,output,shm,resource.snapshot,program,expected);
            mismatch(s,output,shm,resource.snapshot,program,expected);finalFence(s,output,shm,resource.snapshot,program,expected);}
        return 0;
    }catch(...){s.failure=std::current_exception();return 1;}
}
[[noreturn]] void keepOwnedFailure(WorkerState& s) noexcept {
    (void)s;
    // Test-only failed owner quarantine: no C++ native-resource destructors, false
    // DONE, unload or CPU-thread deletion. Root's unchanged process deadline closes
    // this owned compositor. This is unavailable/failure, never a successful drain.
    for(;;)std::this_thread::sleep_for(std::chrono::seconds(1));
}
void work(Inputs* i) noexcept {
    std::unique_ptr<WorkerState> state;
    try{
        state=std::make_unique<WorkerState>(*i);auto& s=*state;
        emit(*i,"CAPS","REAL_DEVICE",s.egl.init(i->node));profile(*i);
        const auto exts=eglQueryString(s.egl.display,EGL_EXTENSIONS);
        const bool nativeFence=extension(exts,"EGL_ANDROID_native_fence_sync")&&eglGetProcAddress("eglCreateSyncKHR")&&
            eglGetProcAddress("eglDupNativeFenceFDANDROID")&&eglGetProcAddress("eglDestroySyncKHR");
        if(!nativeFence)throw SetupUnavailable("native fence extension/procedures absent");
        emit(*i,"NATIVE_FENCE_CAPABILITY","EXTENSION_AND_PROCS_PRESENT_NOT_COMPLETION");
        CShaderLoader loader({"defines.h"},[]{std::array<std::string,SH_FRAG_LAST> files;files.fill("quad.frag");files[SH_FRAG_SURFACE]="surface.frag";return files;}(),i->shaders);
        const std::map<std::string,std::string> zero={{"USE_RGBA","0"},{"USE_DISCARD","0"},{"USE_TINT","0"},{"USE_ROUNDING","0"},
            {"USE_CM","0"},{"USE_TONEMAP","0"},{"USE_ALT_TONEMAP","0"},{"USE_SDR_MOD","0"},{"USE_BLUR","0"},{"USE_ICC","0"},
            {"USE_MIRROR","0"},{"USE_MOTION_BLUR","0"},{"USE_BLUR_ALPHA_MASK","0"},{"USE_BLUR_MATTE","0"}};
        auto rgba=zero;rgba["USE_RGBA"]="1";
        const auto vertex=loader.process("tex300.vert");
        require(s.quad.createProgram(vertex,loader.process("quad.frag",zero),true,true)&&
            s.rgba.createProgram(vertex,loader.process("surface.frag",rgba),true,true)&&
            s.rgbx.createProgram(vertex,loader.process("surface.frag",zero),true,true),"matched original shader compilation failed");
        s.primitives=std::make_unique<COwnerFramePrimitives>(s.egl.display,s.egl.context);
        s.texture=std::make_unique<COwnerShmTexture>(s.egl.display,s.egl.context);s.fence=std::make_unique<COwnerNativeFence>(s.egl.display,s.egl.context);
        qualifyNativeFence(s);
        Visit first{s,true};require(pr_frame_with_view_v1(i->frame,visit,&first)==0,"AR actual frame visitor failed");
        // The synchronous callback and ALL its nested borrows have returned.
        pr_frame_release(i->frame);i->frame=nullptr;
        std::exception_ptr producerFailure;
        std::thread producer([&]{try{capture(*i,XR24,2);}catch(...){producerFailure=std::current_exception();}});
        producer.join();if(producerFailure)std::rethrow_exception(producerFailure);
        Visit second{s,false};require(pr_frame_with_view_v1(i->frame,visit,&second)==0,"XR actual frame visitor failed");
        pr_frame_release(i->frame);i->frame=nullptr;
        require(s.texture->retire()==eOwnerResourceStatus::OK&&s.fence->retire()==eOwnerResourceStatus::OK,"empty correct-owner idempotent retirement failed");
        s.texture.reset();s.fence.reset();s.primitives.reset();
        if(s.unpack){glDeleteBuffers(1,&s.unpack);s.unpack=0;}
        glUseProgram(0);s.quad.destroy();s.rgba.destroy();s.rgbx.destroy();require(glGetError()==GL_NO_ERROR,"worker program/PBO cleanup failed");
        // Program deletion is local resource cleanup, not an extra GPU-drain fact.
        s.egl.finish();emit(*i,"WORKER_CLEANUP","CONTEXT_CLOSED_AFTER_OBSERVED_FENCE_LOCAL_RETIREMENT");state.reset();
        emit(*i,"DONE","EIGHT_CASES_CLOSED_NO_SOURCE_OR_PRESENTATION_CLAIM");
    }catch(const SetupUnavailable&){
        try{emit(*i,"NATIVE_FENCE_CAPABILITY","SETUP_UNAVAILABLE");}catch(...){}
        if(state&&(state->capabilitySync!=EGL_NO_SYNC_KHR||state->capabilityFD>=0))keepOwnedFailure(*state);
        state.reset();
    }catch(...){
        try{emit(*i,state&&state->helperAcquisitionStarted?state->activeCase:"SETUP",state&&state->helperAcquisitionStarted?"BEHAVIOR_FAILED_RETAINED_OWNER":"SETUP_FAILED");}catch(...){}
        if(state&&(state->helperAcquisitionStarted||state->capabilitySync!=EGL_NO_SYNC_KHR||state->capabilityFD>=0))keepOwnedFailure(*state);
        state.reset();
    }
    if(i->frame){pr_frame_release(i->frame);i->frame=nullptr;}
    i->done.store(true,std::memory_order_release);
    if(i->fd>=0){close(i->fd);i->fd=-1;}
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION(){return HYPRLAND_API_VERSION;}
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE){
    static_assert(std::is_nothrow_move_constructible_v<PLUGIN_DESCRIPTION_INFO>);
    PLUGIN_DESCRIPTION_INFO info{"owner-gl-behavior","Private future real immutable SHM/fence boundary actor","Pokelinux","0"};
    if(actor||std::string(__hyprland_api_get_hash())!=__hyprland_api_get_client_hash())throw std::runtime_error("exact future SDK mismatch");
    auto next=std::make_unique<Inputs>();next->run=decimal("POKELINUX_OWNER_GL_RUN");next->until=decimal("POKELINUX_OWNER_GL_UNTIL_NS");
    require(next->run&&boot()<next->until,"nonzero run/original future actor deadline required");
    next->node=path("POKELINUX_OWNER_GL_RENDER_NODE",255);next->shaders=path("POKELINUX_OWNER_GL_SHADER_DIR",4095);
    // Connect only at explicit plugin load, after compositor startup. There is
    // no receipt FD inherited through that exec or its early Xwayland children.
    const auto resultPath=path("POKELINUX_OWNER_GL_RESULT_SOCKET",sizeof(sockaddr_un::sun_path)-1);
    const auto controllerPid=decimal("POKELINUX_OWNER_GL_RESULT_PID");
    if(controllerPid<=1||controllerPid>static_cast<uint64_t>(std::numeric_limits<pid_t>::max()))
        throw std::runtime_error("invalid result owner PID");
    next->fd=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);
    if(next->fd<0)throw std::runtime_error("result socket ownership");
    sockaddr_un address{};address.sun_family=AF_UNIX;
    std::memcpy(address.sun_path,resultPath.c_str(),resultPath.size()+1);
    if(connect(next->fd,reinterpret_cast<const sockaddr*>(&address),sizeof(address)))
        throw std::runtime_error("result owner connect unavailable");
    ucred peer{};socklen_t length=sizeof(peer);
    if(getsockopt(next->fd,SOL_SOCKET,SO_PEERCRED,&peer,&length)||length!=sizeof(peer)||
       peer.pid!=static_cast<pid_t>(controllerPid)||peer.uid!=0||peer.gid!=0)
        throw std::runtime_error("result owner credentials mismatch");
    capture(*next);actor=std::move(next);
    try{actor->worker=std::thread(work,actor.get());}catch(...){actor.reset();throw;}
    return info;
}
APICALL EXPORT void PLUGIN_EXIT(){
    if(!actor)return;
    if(!actor->done.load(std::memory_order_acquire))std::abort();
    if(actor->worker.joinable())actor->worker.join();
    actor.reset();
}
