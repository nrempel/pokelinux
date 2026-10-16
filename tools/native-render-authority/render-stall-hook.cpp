// Test-only real EGL native-fence creation latch. No Source facts, policy or Wayland state.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <atomic>
#include <cerrno>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <type_traits>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/un.h>
#include <poll.h>
#include <time.h>
#include <unistd.h>

namespace {
constexpr const char* CONTROL_PATH="/run/pokelinux-render-stall/control.sock";
constexpr uint32_t MAGIC=0x504c5253, VERSION=1;
enum Kind:uint32_t { HELLO=1,ARM=2,ENTER=3,RELEASE=4,RETURN=5,RESTORED=6 };
struct Packet {uint32_t magic,version,kind,status;uint64_t token,at_ns,until_ns,value0,value1,value2;};
static_assert(sizeof(Packet)==64);
std::atomic_flag active=ATOMIC_FLAG_INIT;
std::atomic<uint64_t> entered=0,returned=0,activeCalls=0;
struct CallGuard { CallGuard(){activeCalls.fetch_add(1,std::memory_order_acq_rel);} ~CallGuard(){activeCalls.fetch_sub(1,std::memory_order_acq_rel);} };
PFNEGLCREATESYNCKHRPROC original=nullptr;
int channel=-1;
uint64_t clockNs() noexcept {
 timespec ts{};if(clock_gettime(CLOCK_BOOTTIME,&ts)||ts.tv_sec<0||ts.tv_nsec<0||ts.tv_nsec>=1000000000L)return 0;
 const auto sec=static_cast<uint64_t>(ts.tv_sec);
 if(sec>(std::numeric_limits<uint64_t>::max()-static_cast<uint64_t>(ts.tv_nsec))/1000000000ULL)return 0;
 return sec*1000000000ULL+static_cast<uint64_t>(ts.tv_nsec);
}
bool packet(Kind kind,uint32_t status,uint64_t token,uint64_t until,uint64_t v0=0,uint64_t v1=0,uint64_t v2=0) noexcept {
 const Packet p{MAGIC,VERSION,kind,status,token,clockNs(),until,v0,v1,v2};
 return channel>=0&&send(channel,&p,sizeof(p),MSG_DONTWAIT|MSG_NOSIGNAL)==static_cast<ssize_t>(sizeof(p));
}
EGLSyncKHR EGLAPIENTRY heldCreate(EGLDisplay display,EGLenum type,const EGLint* attributes) noexcept {
 CallGuard callGuard;
 const int incomingErrno=errno;
 // Only the real CEGLSync::create native-fence shape is eligible. Never inspect/log opaque arguments.
 bool owned=false,claimed=false,released=false;Packet arm{};uint64_t enteredAt=0,releaseAt=0;
 if(type==EGL_SYNC_NATIVE_FENCE_ANDROID&&attributes==nullptr&&!active.test_and_set(std::memory_order_acquire)){
  owned=true;
  const auto n=recv(channel,&arm,sizeof(arm),MSG_DONTWAIT|MSG_TRUNC);
  if(n==static_cast<ssize_t>(sizeof(arm))&&arm.magic==MAGIC&&arm.version==VERSION&&arm.kind==ARM&&arm.status==0&&arm.token>0){
   const auto current=clockNs();
   if(current>0&&current<arm.until_ns&&arm.until_ns-current<=5'000'000'000ULL){
    enteredAt=current;entered.fetch_add(1,std::memory_order_relaxed);
    claimed=packet(ENTER,0,arm.token,arm.until_ns,static_cast<uint64_t>(getpid()),static_cast<uint64_t>(syscall(SYS_gettid)),EGL_SYNC_NATIVE_FENCE_ANDROID);
    while(claimed){
     const auto currentWait=clockNs();if(currentWait==0||currentWait>=arm.until_ns)break;
     pollfd fd{channel,POLLIN,0};const auto left=arm.until_ns-currentWait;
     const auto ms=static_cast<int>(left/1000000ULL);const int timeout=ms>10?10:ms>0?ms:1;
     const int ready=poll(&fd,1,timeout);
     if(ready<0&&errno==EINTR)continue;
     if(ready<0||(fd.revents&(POLLHUP|POLLERR|POLLNVAL)))break;
     if(!(fd.revents&POLLIN))continue;
     Packet reply{};const auto count=recv(channel,&reply,sizeof(reply),MSG_DONTWAIT|MSG_TRUNC);
     if(count!=static_cast<ssize_t>(sizeof(reply))||reply.magic!=MAGIC||reply.version!=VERSION||reply.kind!=RELEASE||reply.status!=0||reply.token!=arm.token)break;
     releaseAt=clockNs();released=releaseAt>enteredAt&&releaseAt<arm.until_ns;break;
    }
   }
  }
 }
 errno=incomingErrno;
 const EGLSyncKHR sync=original(display,type,attributes); // Exact original call/arguments, including failures.
 const int delegatedErrno=errno;
 if(owned){
  if(claimed){returned.fetch_add(1,std::memory_order_relaxed);packet(RETURN,released?0:1,arm.token,arm.until_ns,releaseAt>enteredAt?releaseAt-enteredAt:0,sync!=EGL_NO_SYNC_KHR?1:0,static_cast<uint64_t>(delegatedErrno));}
  active.clear(std::memory_order_release);
 }
 errno=delegatedErrno;return sync; // Do not call eglGetError or infer GPU completion from this return.
}
}
APICALL EXPORT std::string PLUGIN_API_VERSION(){return HYPRLAND_API_VERSION;}
APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE){
 static_assert(std::is_nothrow_move_constructible_v<PLUGIN_DESCRIPTION_INFO>);
 PLUGIN_DESCRIPTION_INFO description{"render-stall-fixture","One private native EGL fence-call hold; no Source or policy","Pokelinux","0"};
 if(std::string(__hyprland_api_get_hash())!=__hyprland_api_get_client_hash())throw std::runtime_error("exact fixture/compositor SDK mismatch");
 if(!Render::GL::g_pHyprOpenGL||!Render::GL::g_pHyprOpenGL->m_proc.eglCreateSyncKHR)throw std::runtime_error("native fence procedure unavailable");
 channel=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);
 if(channel<0)throw std::runtime_error("private latch socket");
 sockaddr_un address{};address.sun_family=AF_UNIX;std::strncpy(address.sun_path,CONTROL_PATH,sizeof(address.sun_path)-1);
 if(connect(channel,reinterpret_cast<sockaddr*>(&address),sizeof(address))<0){close(channel);channel=-1;throw std::runtime_error("private Root latch connect");}
 ucred peer{};socklen_t n=sizeof(peer);
 if(getsockopt(channel,SOL_SOCKET,SO_PEERCRED,&peer,&n)||n!=sizeof(peer)||peer.uid!=0||peer.pid<=0){close(channel);channel=-1;throw std::runtime_error("private Root peer identity");}
 original=Render::GL::g_pHyprOpenGL->m_proc.eglCreateSyncKHR;
 if(!packet(HELLO,0,0,0,static_cast<uint64_t>(getpid()),static_cast<uint64_t>(syscall(SYS_gettid)),EGL_SYNC_NATIVE_FENCE_ANDROID)){close(channel);channel=-1;throw std::runtime_error("private latch HELLO");}
 // Baseline PluginInit is serialized on the original loop. A future render owner must be
 // externally proven quiescent/no pointer readers before this plain public pointer write.
 Render::GL::g_pHyprOpenGL->m_proc.eglCreateSyncKHR=heldCreate;
 return description;
}
APICALL EXPORT void PLUGIN_EXIT(){
 // Controller must prove no new render admission and actual owned render drain before unload.
 // activeCalls covers every eligible/ineligible/concurrent wrapper, but cannot itself prove quiescence.
 // Unloading active code cannot be made safe by leaving a dangling function pointer.
 if(activeCalls.load(std::memory_order_acquire)!=0||active.test(std::memory_order_acquire))std::abort();
 const bool selected=Render::GL::g_pHyprOpenGL&&Render::GL::g_pHyprOpenGL->m_proc.eglCreateSyncKHR==heldCreate;
 if(selected)Render::GL::g_pHyprOpenGL->m_proc.eglCreateSyncKHR=original;
 packet(RESTORED,selected?0:1,0,0,entered.load(),returned.load());
 if(channel>=0){close(channel);channel=-1;}
}
