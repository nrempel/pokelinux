// A negative status envelope must not silently discard an already accepted cleanup Stop.
#include "pokelinux_sounds_client.h"
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <time.h>
static uint64_t now(){timespec t{};assert(clock_gettime(CLOCK_BOOTTIME,&t)==0);return uint64_t(t.tv_sec)*1000000000ULL+uint64_t(t.tv_nsec);}
struct Client{
 PrSoundsClient*p=nullptr;
 Client(){assert(pr_sounds_client_new_v1(&p)==PR_SOUNDS_CLIENT_OK&&p);}
 ~Client(){pr_sounds_client_free_v1(p);}
 PrSoundsClientViewV1 view(){PrSoundsClientViewV1 v{};assert(pr_sounds_client_view_v1(p,&v)==PR_SOUNDS_CLIENT_OK);return v;}
 void sent(){PrSoundsClientPacketV1 p{};assert(pr_sounds_client_prepare_v1(this->p,&p)==PR_SOUNDS_CLIENT_OK&&p.length>0&&p.length<=4096);assert(pr_sounds_client_sent_v1(this->p,p.connection_epoch,p.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,p.length)==PR_SOUNDS_CLIENT_OK);}
 int reply(const std::string&s){return pr_sounds_client_reply_v1(p,view().connection_epoch,reinterpret_cast<const uint8_t*>(s.data()),uint32_t(s.size()));}
};
int main(){
 const auto until=now()+5000000000ULL;
 for(const char*result:{"denied","unavailable"}){
  Client c;assert(pr_sounds_client_start_v1(c.p)==PR_SOUNDS_CLIENT_OK);assert(pr_sounds_client_connected_v1(c.p,c.view().connection_epoch)==PR_SOUNDS_CLIENT_OK);c.sent();
  assert(c.reply("{\"result\":\"media_status\",\"state\":{\"connected\":true,\"available\":true,\"pending\":false,\"requested\":\"media-a7\",\"applied\":null,\"policy_settled\":true,\"preview\":\"playing\"},\"routes\":[{\"id\":\"media-a7\",\"label\":\"Speaker\"}],\"next_offset\":null,\"can_choose\":true,\"can_reconnect\":false}")==PR_SOUNDS_CLIENT_OK);
  assert(c.view().can_stop&&pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);c.sent();assert(pr_sounds_client_preview_stop_v1(c.p)==PR_SOUNDS_CLIENT_OK&&c.view().queued_kind==PR_SOUNDS_CLIENT_STOP_INTENT);
  assert(c.reply(std::string("{\"result\":\"")+result+"\"}")==PR_SOUNDS_CLIENT_OK);const auto v=c.view();
  assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&!v.online&&!v.can_play&&!v.can_stop&&v.queued_kind==0&&v.can_reconnect&&v.status==PR_SOUNDS_CLIENT_PREVIEW_CANCELLED);
  assert(now()<until);std::printf("SOUNDS_PREVIEW_CLEANUP_PASS negative_envelope=%s channel_closed_no_quiet_claim=true\n",result);
 }
 std::puts("SOUNDS_PREVIEW_CLEANUP_ALL_PASS groups=1 cases=2");
}
