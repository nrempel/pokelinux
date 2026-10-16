// Test-only actual EGL/GL baseline. No owner-resource implementation or AQ commit.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/render/gl/OwnerFramePrimitives.hpp>
#include <hyprland/src/render/Shader.hpp>
#include <hyprland/src/render/ShaderLoader.hpp>
#include "pokelinux_render_frame.h"
#include <EGL/eglext.h>
#include <gbm.h>
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

#if __has_include(<hyprland/src/render/gl/OwnerFrameResources.hpp>)
constexpr bool ownerInterfaceDeclared = true;
#else
constexpr bool ownerInterfaceDeclared = false;
#endif
using namespace Render;
using namespace Render::GL;
namespace {
constexpr uint32_t WIDTH=8, HEIGHT=6, AR24=0x34325241;
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
    uint64_t producer_dropped=0;
    std::thread worker;
    std::atomic<bool> done=false;
    ~Inputs(){
        if(frame)pr_frame_release(frame);
        if(fd>=0)close(fd);
    }
};
std::unique_ptr<Inputs> actor;
void emit(Inputs& i,const char* test,const char* status,uint64_t value=0,uint64_t entered=0,uint64_t returned=0) {
    char line[1024];
    const int n=std::snprintf(line,sizeof(line),
        "{\"schema\":1,\"authority\":\"OFFSCREEN_FIXTURE_IDS_ONLY\",\"run\":%llu,\"case\":\"%s\",\"status\":\"%s\",\"collection_boottime_ns\":%llu,\"operation_enter_ns\":%llu,\"operation_return_ns\":%llu,\"tid\":%llu,\"scene_sequence\":1,\"security_generation\":1,\"output_id\":1,\"output_generation\":1,\"resource_id\":1,\"resource_generation\":1,\"commit_sequence\":1,\"value\":%llu}",
        static_cast<unsigned long long>(i.run),test,status,static_cast<unsigned long long>(boot()),
        static_cast<unsigned long long>(entered),static_cast<unsigned long long>(returned),
        static_cast<unsigned long long>(syscall(SYS_gettid)),static_cast<unsigned long long>(value));
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
        fd=open(node.c_str(),O_RDWR|O_CLOEXEC);
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
void capture(Inputs& i){
    constexpr size_t BYTES=32*3;
    auto* bytes=static_cast<uint8_t*>(mmap(nullptr,BYTES,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0));
    if(bytes==MAP_FAILED)throw std::runtime_error("producer mapping");
    std::memset(bytes,0xA5,BYTES);
    for(uint32_t y=0;y<3;++y)for(uint32_t x=0;x<5;++x){
        const uint8_t pixel[4]={static_cast<uint8_t>((x&1)?255:0),static_cast<uint8_t>((y&1)?255:0),static_cast<uint8_t>(((x+y)&1)?255:0),255};
        std::memcpy(bytes+y*32+x*4,pixel,4);
    }
    PrSnapshotStore* store=pr_snapshot_store_create(4096);
    PrSnapshot* snapshot=nullptr;
    const PrShmDescription desc{1,1,1,AR24,5,3,32,4,0};
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
    draws[2].uv[2]=1;draws[2].uv[3]=1;draws[2].surface_id=1;draws[2].commit_sequence=1;
    std::copy(rectMatrix.begin(),rectMatrix.end(),draws[2].projection);
    const std::array<PrFrameRectV1,5> rects={{{0,0,8,6},{0,0,8,6},{0,0,8,6},{2,2,5,4},{1,1,6,4}}};
    const PrFrameResourceBorrowV1 resource{desc,snapshot};
    PrFrameBorrowV1 frame{};frame.abi_version=1;frame.struct_size=sizeof(frame);frame.flags=PR_FRAME_COMMIT|PR_FRAME_FULL_REPAINT;
    frame.scene_sequence=1;frame.security_generation=1;
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
struct Call {Inputs& inputs;EGLFixture& egl;COwnerFramePrimitives& primitives;CShader& quad;};
int32_t controls(const PrFrameBorrowV1* v,void* token) noexcept {
    try{
        auto& c=*static_cast<Call*>(token);
        if(!v||v->draw_count!=3||v->resource_count!=1||!v->draws||!v->resources||!v->rects)throw std::runtime_error("frame shape");
        PrSnapshotView source{};
        if(pr_snapshot_view(v->resources[0].snapshot,&source)!=PR_OK||!source.pixels||source.pixel_bytes!=96)
            throw std::runtime_error("retained actual snapshot unavailable");
        const auto same=[](const PrShmDescription& d){return d.resource_id==1&&d.generation==1&&d.commit_sequence==1&&
            d.drm_format==AR24&&d.width==5&&d.height==3&&d.stride==32&&d.bytes_per_pixel==4&&d.reserved==0;};
        if(!same(source.description)||!same(v->resources[0].description))throw std::runtime_error("retained full resource identity mismatch");
        uint64_t byteMismatches=0;
        for(size_t index=0;index<96;++index){
            const auto y=index/32,offset=index%32;uint8_t expected=0xA5;
            if(offset<20){const auto x=offset/4,channel=offset%4;
                const std::array<uint8_t,4> pixel={static_cast<uint8_t>((x&1)?255:0),static_cast<uint8_t>((y&1)?255:0),
                    static_cast<uint8_t>(((x+y)&1)?255:0),255};expected=pixel[channel];}
            if(source.pixels[index]!=expected)++byteMismatches;
        }
        emit(c.inputs,"CUSTODY_BYTES",byteMismatches?"CONTROL_FAILED":"EXACT_96_BYTES",byteMismatches);
        if(byteMismatches)throw std::runtime_error("retained pixel/padding bytes differ");
        const auto t=target(v->target);const SOwnerFrameTarget output{t,c.egl.fb};
        const auto clear=leaf(*v,0);
        auto entered=boot();const auto clearStatus=c.primitives.drawClear(output,clear);auto returned=boot();
        if(clearStatus!=eOwnerFramePrimitiveStatus::COMMANDS_ISSUED)throw std::runtime_error("CLEAR did not issue commands");
        auto mismatch=compare(c.egl,false);
        emit(c.inputs,"NGL-01",mismatch?"CONTROL_FAILED":"PASS",mismatch,entered,returned);
        if(mismatch)return 1;
        const auto draw=leaf(*v,1);const auto key=COwnerFramePrimitives::shaderKey(draw);
        if(!key)throw std::runtime_error("RECT key unavailable");
        entered=boot();const auto rectStatus=c.primitives.drawRect(output,draw,SOwnerFrameProgram{c.quad,*key});returned=boot();
        if(rectStatus!=eOwnerFramePrimitiveStatus::COMMANDS_ISSUED)throw std::runtime_error("RECT did not issue commands");
        mismatch=compare(c.egl,true);emit(c.inputs,"NGL-02",mismatch?"CONTROL_FAILED":"PASS",mismatch,entered,returned);return mismatch?1:0;
    }catch(...){return 2;}
}
void work(Inputs* i) noexcept {
    try{
        {
            EGLFixture egl;emit(*i,"CAPS","REAL_DEVICE",egl.init(i->node));profile(*i);
            emit(*i,"CUSTODY","PRODUCER_DROPPED",i->producer_dropped);
            const auto exts=eglQueryString(egl.display,EGL_EXTENSIONS);
            const bool nativeFence=extension(exts,"EGL_ANDROID_native_fence_sync")&&eglGetProcAddress("eglCreateSyncKHR")&&
                eglGetProcAddress("eglDupNativeFenceFDANDROID")&&eglGetProcAddress("eglDestroySyncKHR");
            emit(*i,"NATIVE_FENCE_CAPABILITY",nativeFence?"EXTENSION_AND_PROCS_PRESENT_UNQUALIFIED":"SETUP_UNAVAILABLE");
            CShaderLoader loader({"defines.h"},[] {std::array<std::string,SH_FRAG_LAST> files;files.fill("quad.frag");files[SH_FRAG_SURFACE]="surface.frag";return files;}(),i->shaders);
            CShader quad;
            // process(explicit defines) avoids getVariantSource's live CConfigValue read.
            const std::map<std::string,std::string> zeroFeatures={
                {"USE_RGBA","0"},{"USE_DISCARD","0"},{"USE_TINT","0"},{"USE_ROUNDING","0"},
                {"USE_CM","0"},{"USE_TONEMAP","0"},{"USE_ALT_TONEMAP","0"},{"USE_SDR_MOD","0"},
                {"USE_BLUR","0"},{"USE_ICC","0"},{"USE_MIRROR","0"},{"USE_MOTION_BLUR","0"},
                {"USE_BLUR_ALPHA_MASK","0"},{"USE_BLUR_MATTE","0"}};
            if(!quad.createProgram(loader.process("tex300.vert"),loader.process("quad.frag",zeroFeatures),true,true))
                throw std::runtime_error("real matched RECT shader unavailable");
            COwnerFramePrimitives primitives(egl.display,egl.context);
            Call call{*i,egl,primitives,quad};
            const int status=pr_frame_with_view_v1(i->frame,controls,&call);
            if(status)throw std::runtime_error("actual CLEAR/RECT controls failed");
            // No SHM/fence behavior is called or substituted in this baseline profile.
            if(ownerInterfaceDeclared)throw std::runtime_error("baseline actor refuses a changed source interface");
            for(const char* test:{"NGL-03","NGL-04","NGL-05","NGL-06","NGL-07"})
                emit(*i,test,"STRUCTURAL_INTERFACE_ABSENT");
            emit(*i,"NGL-08",nativeFence?"STRUCTURAL_INTERFACE_ABSENT":"SETUP_UNAVAILABLE");
            pr_frame_release(i->frame);i->frame=nullptr;
            // Unbind before program deletion. This is resource cleanup, never a GPU-drain oracle.
            glUseProgram(0);quad.destroy();
            if(glGetError()!=GL_NO_ERROR)throw std::runtime_error("baseline shader cleanup failed");
            egl.finish();emit(*i,"WORKER_CLEANUP","CONTEXT_CLOSED_NO_GPU_DRAIN_CLAIM");
        } // All worker GL and EGL fixture objects gone before DONE.
        emit(*i,"DONE","BASELINE_CONTROLS_CLOSED");
    }catch(...){try{emit(*i,"SETUP","SETUP_FAILED");}catch(...){}}
    if(i->frame){pr_frame_release(i->frame);i->frame=nullptr;}
    // All native scopes are gone. Publish quiescence before socket EOF so an
    // immediate parent unload can join this CPU-only tail safely.
    i->done.store(true,std::memory_order_release);
    if(i->fd>=0){close(i->fd);i->fd=-1;}
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION(){return HYPRLAND_API_VERSION;}
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE){
    static_assert(std::is_nothrow_move_constructible_v<PLUGIN_DESCRIPTION_INFO>);
    PLUGIN_DESCRIPTION_INFO info{"owner-gl-baseline","Private real EGL CLEAR/RECT plus explicit missing interface","Pokelinux","0"};
    if(actor||std::string(__hyprland_api_get_hash())!=__hyprland_api_get_client_hash())throw std::runtime_error("exact baseline SDK mismatch");
    auto next=std::make_unique<Inputs>();next->run=decimal("POKELINUX_OWNER_GL_RUN");
    if(!next->run)throw std::runtime_error("zero actor run");
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
    // Parent may unload only after DONE/EOF; never unload live worker code.
    if(!actor->done.load(std::memory_order_acquire))std::abort();
    if(actor->worker.joinable())actor->worker.join();
    actor.reset();
}
