// App-local preview contract through the public C ABI. No native socket, authority or PCM.
#include "pokelinux_sounds_client.h"
#include <algorithm>
#include <cassert>
#include <cstdint>
#include <cstddef>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>
#include <time.h>

static uint64_t work = 0;
static uint64_t now() {
    timespec t{};
    assert(clock_gettime(CLOCK_BOOTTIME, &t) == 0 && t.tv_sec >= 0 && t.tv_nsec >= 0 && t.tv_nsec < 1000000000);
    return uint64_t(t.tv_sec) * 1000000000ULL + uint64_t(t.tv_nsec);
}
static void bounded() { assert(now() < work); }
static void pass(const char *name, unsigned cases = 1) {
    bounded();
    std::printf("SOUNDS_PREVIEW_PASS %s cases=%u\n", name, cases);
    std::fflush(stdout);
}
[[maybe_unused]] static bool same(const PrSoundsClientIdV1 &value, const char *text) {
    return value.length <= 64 && value.length == std::strlen(text) && std::memcmp(value.bytes, text, value.length) == 0;
}
struct Client final {
    PrSoundsClient *p = nullptr;
    Client() { bounded(); assert(pr_sounds_client_new_v1(&p) == PR_SOUNDS_CLIENT_OK && p); }
    ~Client() { pr_sounds_client_free_v1(p); }
    Client(const Client &) = delete;
    Client &operator=(const Client &) = delete;
    PrSoundsClientViewV1 view() const {
        PrSoundsClientViewV1 out{};
        assert(pr_sounds_client_view_v1(p, &out) == PR_SOUNDS_CLIENT_OK && out.reserved == 0 && out.route_count <= 32);
        return out;
    }
    PrSoundsClientPacketV1 packet() {
        PrSoundsClientPacketV1 out{};
        assert(pr_sounds_client_prepare_v1(p, &out) == PR_SOUNDS_CLIENT_OK);
        assert(out.connection_epoch == view().connection_epoch && out.token > 0 && out.reserved == 0 && out.length > 0 && out.length <= 4096);
        return out;
    }
    void sent(const PrSoundsClientPacketV1 &packet) {
        assert(pr_sounds_client_sent_v1(p, packet.connection_epoch, packet.token, PR_SOUNDS_CLIENT_SEND_COMPLETE, packet.length) == PR_SOUNDS_CLIENT_OK);
    }
    void begin() {
        bounded();
        assert(pr_sounds_client_start_v1(p) == PR_SOUNDS_CLIENT_OK);
        const auto connecting = view();
        assert(connecting.stage == PR_SOUNDS_CLIENT_CONNECTING && !connecting.online && connecting.connection_epoch > 0);
        assert(connecting.request_until_boottime_ns > now());
        assert(pr_sounds_client_connected_v1(p, connecting.connection_epoch) == PR_SOUNDS_CLIENT_OK);
        const auto request = packet();
        assert(std::string(reinterpret_cast<const char *>(request.bytes), request.length) == "{\"offset\":0,\"op\":\"media_status\"}");
        assert(request.original_until_boottime_ns == connecting.request_until_boottime_ns);
        sent(request);
        assert(view().stage == PR_SOUNDS_CLIENT_RECEIVING);
    }
    int reply(const std::string &bytes) {
        bounded();
        assert(!bytes.empty() && bytes.size() <= 4096);
        return pr_sounds_client_reply_v1(p, view().connection_epoch, reinterpret_cast<const uint8_t *>(bytes.data()), uint32_t(bytes.size()));
    }
};
static std::string state(const char *preview="idle",bool settled=true,const char *requested="\"media-a7\"",const char *applied="null",bool available=true) {
 return std::string("{\"connected\":true,\"available\":")+(available?"true":"false")+",\"pending\":false,\"requested\":"+requested+",\"applied\":"+applied+",\"policy_settled\":"+(settled?"true":"false")+",\"preview\":\""+preview+"\"}";
}
static std::string status(const std::string&s=state()){return "{\"result\":\"media_status\",\"state\":"+s+",\"routes\":[{\"id\":\"media-a7\",\"label\":\"Speaker\"}],\"next_offset\":null,\"can_choose\":true,\"can_reconnect\":false}";}
static void online(Client&c,const std::string&s=status()){c.begin();assert(c.reply(s)==PR_SOUNDS_CLIENT_OK&&c.view().stage==PR_SOUNDS_CLIENT_IDLE);}
static std::string text(const PrSoundsClientPacketV1&p){return {reinterpret_cast<const char*>(p.bytes),p.length};}
static uint64_t number(const std::string&s,const char*name){const auto key=std::string("\"")+name+"\":";auto i=s.find(key);assert(i!=std::string::npos);i+=key.size();assert(i<s.size()&&s[i]>='0'&&s[i]<='9');uint64_t n=0;while(i<s.size()&&s[i]>='0'&&s[i]<='9'){const auto d=unsigned(s[i++]-'0');assert(n<=(UINT64_MAX-d)/10);n=n*10+d;}return n;}
static void refresh(Client&c){assert(pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);c.sent(c.packet());}
static void advance(uint64_t until){while(now()<until){bounded();timespec pause{0,1000000};(void)nanosleep(&pause,nullptr);}}
static void projection(){
 static_assert(sizeof(PrSoundsClientViewV1)==5600&&offsetof(PrSoundsClientViewV1,routes)==208&&offsetof(PrSoundsClientViewV1,policy_settled)==5584);
 for(unsigned i=0;i<6;++i){const char*names[]={"idle","starting","playing","retiring","completed","failed"};Client c;online(c,status(state(names[i])));auto v=c.view();assert(v.policy_settled==1&&v.preview==i);assert(bool(v.can_play)==(i==0||i==4||i==5));assert(bool(v.can_stop)==(i==1||i==2||i==3));if(i==0)assert(v.status==PR_SOUNDS_CLIENT_SELECTED_IDLE);if(i==1)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_STARTING);if(i==2)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_PLAYING);if(i==3)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_RETIRING);}
 Client missing;online(missing,status(state("idle",false)));assert(!missing.view().can_play);Client route;online(route,status(state("idle",true,"null")));assert(!route.view().can_play);pass("explicit_preview_and_policy_projection",8);
}
static void playDeadline(){
 Client c;online(c);const auto before=now();assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto after=now();auto first=c.packet();const auto until=first.original_until_boottime_ns;assert(until>=before+1000000000ULL&&until<=after+1000000000ULL);const auto wire=text(first);assert(wire.find("\"op\":\"media_preview_play\"")!=std::string::npos&&wire.find("\"route\":\"media-a7\"")!=std::string::npos);assert(number(wire,"until_boottime_ns")==until&&number(wire,"ttl_ms")>0&&number(wire,"ttl_ms")<=1000);assert(c.view().can_stop&&!c.view().can_play&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_STARTING);assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_BUSY);
 advance(now()+4000000ULL);auto second=c.packet();assert(second.token==first.token&&second.original_until_boottime_ns==until&&number(text(second),"until_boottime_ns")==until&&number(text(second),"ttl_ms")<number(wire,"ttl_ms"));assert(pr_sounds_client_sent_v1(c.p,second.connection_epoch,second.token,PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK,0)==PR_SOUNDS_CLIENT_OK);advance(now()+2000000ULL);auto third=c.packet();assert(third.token==first.token&&third.original_until_boottime_ns==until&&number(text(third),"ttl_ms")<number(text(second),"ttl_ms"));c.sent(third);assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("starting")+"}")==PR_SOUNDS_CLIENT_OK);assert(c.view().preview==PR_SOUNDS_CLIENT_PREVIEW_STARTING_STATE&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_STARTING);refresh(c);assert(c.reply(status(state("playing",true,"\"media-a7\"","\"media-a7\"")))==PR_SOUNDS_CLIENT_OK);assert(c.view().preview==PR_SOUNDS_CLIENT_PREVIEW_PLAYING_STATE&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_PLAYING);pass("play_original_absolute_and_decreasing_remaining_ttl");
}
static void noQueuedPlay(){Client c;online(c);refresh(c);const auto until=c.view().request_until_boottime_ns;assert(!c.view().can_play&&pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_BUSY&&c.view().queued_kind==0&&c.view().request_until_boottime_ns==until);assert(c.reply(status())==PR_SOUNDS_CLIENT_OK&&c.view().can_play&&c.view().queued_kind==0);PrSoundsClientPacketV1 p{};assert(pr_sounds_client_prepare_v1(c.p,&p)==PR_SOUNDS_CLIENT_EMPTY);pass("readonly_status_never_queues_or_replays_play");}
static void queuedStop(){
 Client c;online(c,status(state("playing")));refresh(c);const auto before=now();assert(pr_sounds_client_preview_stop_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto after=now();auto v=c.view();assert(v.queued_kind==PR_SOUNDS_CLIENT_STOP_INTENT&&v.queued_until_boottime_ns>=before+1000000000ULL&&v.queued_until_boottime_ns<=after+1000000000ULL);const auto until=v.queued_until_boottime_ns;assert(pr_sounds_client_preview_stop_v1(c.p)==PR_SOUNDS_CLIENT_BUSY&&!c.view().can_stop);assert(c.reply(status(state("playing",false,"\"media-a7\"","null",false)))==PR_SOUNDS_CLIENT_OK);auto packet=c.packet();assert(text(packet)=="{\"op\":\"media_preview_stop\"}"&&packet.original_until_boottime_ns==until&&c.view().request_kind==PR_SOUNDS_CLIENT_PREVIEW_STOP);c.sent(packet);assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("retiring",false)+"}")==PR_SOUNDS_CLIENT_OK);assert(c.view().preview==PR_SOUNDS_CLIENT_PREVIEW_RETIRING_STATE&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_RETIRING);pass("queued_cleanup_stop_survives_negative_status");
}
static void cancelInflight(){for(bool sent:{false,true}){Client c;online(c);assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto packet=c.packet();if(sent)c.sent(packet);assert(c.view().can_stop);assert(pr_sounds_client_preview_stop_v1(c.p)==PR_SOUNDS_CLIENT_OK);auto v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&!v.online&&!v.can_play&&!v.can_stop&&v.can_reconnect&&v.status==PR_SOUNDS_CLIENT_PREVIEW_CANCELLED);assert(pr_sounds_client_reconnect_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto epoch=c.view().connection_epoch;assert(pr_sounds_client_connected_v1(c.p,epoch)==PR_SOUNDS_CLIENT_OK);auto inspect=c.packet();assert(text(inspect)=="{\"offset\":0,\"op\":\"media_status\"}");c.sent(inspect);assert(c.reply(status())==PR_SOUNDS_CLIENT_OK&&c.view().queued_kind==0);PrSoundsClientPacketV1 empty{};assert(pr_sounds_client_prepare_v1(c.p,&empty)==PR_SOUNDS_CLIENT_EMPTY);}pass("stop_inflight_play_closes_channel_without_replay",2);}
static void deadlinesAndFailure(){
 Client unsent,received,queued;online(unsent);online(received);online(queued,status(state("playing")));assert(pr_sounds_client_preview_play_v1(unsent.p)==PR_SOUNDS_CLIENT_OK);auto p=unsent.packet();assert(pr_sounds_client_preview_play_v1(received.p)==PR_SOUNDS_CLIENT_OK);received.sent(received.packet());refresh(queued);assert(pr_sounds_client_preview_stop_v1(queued.p)==PR_SOUNDS_CLIENT_OK);const auto until=std::max({p.original_until_boottime_ns,received.view().request_until_boottime_ns,queued.view().request_until_boottime_ns,queued.view().queued_until_boottime_ns});advance(until);PrSoundsClientPacketV1 out{};out.token=99;assert(pr_sounds_client_prepare_v1(unsent.p,&out)==PR_SOUNDS_CLIENT_EXPIRED&&out.token==99);assert(received.reply("{\"result\":\"media_preview\",\"state\":"+state("playing")+"}")==PR_SOUNDS_CLIENT_EXPIRED);assert(pr_sounds_client_prepare_v1(queued.p,&out)==PR_SOUNDS_CLIENT_EXPIRED);for(Client*c:{&unsent,&received,&queued})assert(c->view().stage==PR_SOUNDS_CLIENT_CLOSED&&!c->view().can_play&&!c->view().can_stop&&c->view().queued_kind==0);
 Client failed;online(failed);assert(pr_sounds_client_preview_play_v1(failed.p)==PR_SOUNDS_CLIENT_OK);auto packet=failed.packet();assert(pr_sounds_client_sent_v1(failed.p,packet.connection_epoch,packet.token,PR_SOUNDS_CLIENT_SEND_FAILED,0)==PR_SOUNDS_CLIENT_REFUSED);assert(failed.view().stage==PR_SOUNDS_CLIENT_CLOSED&&failed.view().queued_kind==0);pass("original_deadline_expiry_and_send_failure_withdraw",4);
}
static void replyContext(){Client c;online(c);refresh(c);assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("playing")+"}")==PR_SOUNDS_CLIENT_REFUSED&&c.view().stage==PR_SOUNDS_CLIENT_CLOSED);pass("preview_reply_requires_matching_effect_request");}
int main(){work=now()+5000000000ULL;projection();playDeadline();noQueuedPlay();queuedStop();cancelInflight();deadlinesAndFailure();replyContext();bounded();std::puts("SOUNDS_PREVIEW_ALL_PASS groups=7 authored_CPU_facts_only=true");}
