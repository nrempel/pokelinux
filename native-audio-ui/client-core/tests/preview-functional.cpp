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
#include <utility>
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
        assert(pr_sounds_client_view_v1(p, &out) == PR_SOUNDS_CLIENT_OK && out.reserved == 0 && out.preview_reserved == 0 && out.mute_reserved == 0 && out.route_count <= 32);
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
static std::string state(const char *preview="idle",bool settled=true,const char *requested="\"media-a7\"",const char *applied="null",bool available=true,const char *generation=nullptr,const char *muted=nullptr) {
 const bool paired=std::strcmp(preview,"starting")==0||std::strcmp(preview,"playing")==0;
 return std::string("{\"connected\":true,\"available\":")+(available?"true":"false")+",\"pending\":false,\"requested\":"+requested+",\"applied\":"+applied+",\"policy_settled\":"+(settled?"true":"false")+",\"preview\":\""+preview+"\",\"preview_generation\":"+(generation?generation:(paired?"17":"null"))+",\"preview_muted\":"+(muted?muted:(paired?"false":"null"))+"}";
}
static std::string status(const std::string&s=state()){return "{\"result\":\"media_status\",\"state\":"+s+",\"routes\":[{\"id\":\"media-a7\",\"label\":\"Speaker\"}],\"next_offset\":null,\"can_choose\":true,\"can_reconnect\":false}";}
static void online(Client&c,const std::string&s=status()){c.begin();assert(c.reply(s)==PR_SOUNDS_CLIENT_OK&&c.view().stage==PR_SOUNDS_CLIENT_IDLE);}
static std::string text(const PrSoundsClientPacketV1&p){return {reinterpret_cast<const char*>(p.bytes),p.length};}
static uint64_t number(const std::string&s,const char*name){const auto key=std::string("\"")+name+"\":";auto i=s.find(key);assert(i!=std::string::npos);i+=key.size();assert(i<s.size()&&s[i]>='0'&&s[i]<='9');uint64_t n=0;while(i<s.size()&&s[i]>='0'&&s[i]<='9'){const auto d=unsigned(s[i++]-'0');assert(n<=(UINT64_MAX-d)/10);n=n*10+d;}return n;}
static void refresh(Client&c){assert(pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);c.sent(c.packet());}
static void advance(uint64_t until){while(now()<until){bounded();timespec pause{0,1000000};(void)nanosleep(&pause,nullptr);}}
static void projection(){
 static_assert(sizeof(PrSoundsClientViewV1)==5648&&offsetof(PrSoundsClientViewV1,routes)==208&&offsetof(PrSoundsClientViewV1,policy_settled)==5584&&offsetof(PrSoundsClientViewV1,selected_level_percent)==5600&&offsetof(PrSoundsClientViewV1,preview_reserved)==5612&&offsetof(PrSoundsClientViewV1,preview_generation)==5616&&offsetof(PrSoundsClientViewV1,playback_muted)==5624&&offsetof(PrSoundsClientViewV1,playback_mute_known)==5628&&offsetof(PrSoundsClientViewV1,can_toggle_mute)==5632&&offsetof(PrSoundsClientViewV1,mute_checked)==5636&&offsetof(PrSoundsClientViewV1,mute_is_live)==5640&&offsetof(PrSoundsClientViewV1,mute_reserved)==5644);
 for(unsigned i=0;i<6;++i){const char*names[]={"idle","starting","playing","retiring","completed","failed"};Client c;online(c,status(state(names[i])));auto v=c.view();assert(v.policy_settled==1&&v.preview==i);assert(bool(v.can_play)==(i==0||i==4||i==5));assert(bool(v.can_stop)==(i==1||i==2||i==3));if(i==0)assert(v.status==PR_SOUNDS_CLIENT_SELECTED_IDLE);if(i==1)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_STARTING);if(i==2)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_PLAYING);if(i==3)assert(v.status==PR_SOUNDS_CLIENT_PREVIEW_RETIRING);}
 Client missing;online(missing,status(state("idle",false)));assert(!missing.view().can_play);Client route;online(route,status(state("idle",true,"null")));assert(!route.view().can_play);pass("explicit_preview_and_policy_projection",8);
}
static void playDeadline(){
 Client c;online(c);const auto before=now();assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto after=now();auto first=c.packet();const auto until=first.original_until_boottime_ns;assert(until>=before+1000000000ULL&&until<=after+1000000000ULL);const auto wire=text(first);assert(number(wire,"level_percent")==100&&wire.find("\"muted\":false")!=std::string::npos);assert(wire.find("\"op\":\"media_preview_play\"")!=std::string::npos&&wire.find("\"route\":\"media-a7\"")!=std::string::npos);assert(number(wire,"until_boottime_ns")==until&&number(wire,"ttl_ms")>0&&number(wire,"ttl_ms")<=1000);assert(c.view().can_stop&&!c.view().can_play&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_STARTING);assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_BUSY);
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

static void localLevel() {
 Client c;
 assert(c.view().selected_level_percent==100&&!c.view().selected_muted&&c.view().can_adjust_preview);
 assert(pr_sounds_client_preview_level_v1(nullptr,50,0)==PR_SOUNDS_CLIENT_INVALID);
 Client connecting;
 assert(pr_sounds_client_start_v1(connecting.p)==PR_SOUNDS_CLIENT_OK);
 const auto connection=connecting.view();
 assert(connection.can_adjust_preview&&pr_sounds_client_preview_level_v1(connecting.p,50,1)==PR_SOUNDS_CLIENT_OK);
 assert(connecting.view().connection_epoch==connection.connection_epoch&&connecting.view().request_until_boottime_ns==connection.request_until_boottime_ns);
 online(c);
 for(uint32_t value:{0u,50u,100u}) {
  assert(pr_sounds_client_preview_level_v1(c.p,value,1)==PR_SOUNDS_CLIENT_OK);
  assert(c.view().selected_level_percent==value&&c.view().selected_muted==1);
 }
 for(const auto &bad:std::vector<std::pair<uint32_t,uint32_t>>{{101,0},{UINT32_MAX,0},{50,2},{0,UINT32_MAX}}) {
  const auto before=c.view();
  assert(pr_sounds_client_preview_level_v1(c.p,bad.first,bad.second)==PR_SOUNDS_CLIENT_INVALID);
  const auto after=c.view();
  assert(after.selected_level_percent==before.selected_level_percent&&after.selected_muted==before.selected_muted);
  assert(after.request_kind==before.request_kind&&after.request_until_boottime_ns==before.request_until_boottime_ns&&after.queued_kind==0);
 }
 PrSoundsClientPacketV1 none{};none.token=99;
 assert(pr_sounds_client_prepare_v1(c.p,&none)==PR_SOUNDS_CLIENT_EMPTY&&none.token==99);
 assert(pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 auto request=c.packet();const auto until=request.original_until_boottime_ns;
 assert(pr_sounds_client_preview_level_v1(c.p,50,0)==PR_SOUNDS_CLIENT_OK&&c.view().can_adjust_preview);
 auto same=c.packet();assert(same.token==request.token&&same.original_until_boottime_ns==until&&text(same)==text(request));
 c.sent(same);
 assert(pr_sounds_client_preview_level_v1(c.p,100,1)==PR_SOUNDS_CLIENT_OK);
 assert(c.view().request_until_boottime_ns==until&&c.view().request_kind==PR_SOUNDS_CLIENT_STATUS&&c.view().queued_kind==0);
 assert(c.reply(status())==PR_SOUNDS_CLIENT_OK);
 assert(c.view().selected_level_percent==100&&c.view().selected_muted==1);
 assert(pr_sounds_client_prepare_v1(c.p,&none)==PR_SOUNDS_CLIENT_EMPTY&&none.token==99);
 pass("atomic_local_level_validation_no_IPC_and_readonly_status");
}
static void levelBusy() {
 for(const char *active:{"starting","playing","retiring"}) {
  Client c;online(c,status(state(active)));assert(!c.view().can_adjust_preview);
  assert(pr_sounds_client_preview_level_v1(c.p,50,1)==PR_SOUNDS_CLIENT_BUSY);
  assert(c.view().selected_level_percent==100&&!c.view().selected_muted);
 }
 Client effect;online(effect);const auto route=effect.view().requested;
 assert(pr_sounds_client_choose_v1(effect.p,&route)==PR_SOUNDS_CLIENT_OK);
 assert(!effect.view().can_adjust_preview&&pr_sounds_client_preview_level_v1(effect.p,50,1)==PR_SOUNDS_CLIENT_BUSY);
 effect.sent(effect.packet());
 assert(pr_sounds_client_preview_level_v1(effect.p,50,1)==PR_SOUNDS_CLIENT_BUSY&&effect.view().selected_level_percent==100);
 Client queued;online(queued);refresh(queued);
 assert(pr_sounds_client_choose_v1(queued.p,&route)==PR_SOUNDS_CLIENT_OK&&queued.view().queued_kind==PR_SOUNDS_CLIENT_ROUTE_INTENT);
 assert(!queued.view().can_adjust_preview&&pr_sounds_client_preview_level_v1(queued.p,50,1)==PR_SOUNDS_CLIENT_BUSY);
 Client stop;online(stop,status(state("playing")));
 assert(pr_sounds_client_preview_stop_v1(stop.p)==PR_SOUNDS_CLIENT_OK);
 assert(!stop.view().can_adjust_preview&&pr_sounds_client_preview_level_v1(stop.p,50,1)==PR_SOUNDS_CLIENT_BUSY);
 pass("level_refuses_effect_queue_and_all_active_preview_states");
}
static void levelSnapshot() {
 Client c;online(c);assert(pr_sounds_client_preview_level_v1(c.p,50,0)==PR_SOUNDS_CLIENT_OK);
 const auto before=now();assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto after=now();
 const auto first=c.packet();const auto until=first.original_until_boottime_ns;
 assert(until>=before+1000000000ULL&&until<=after+1000000000ULL);
 assert(!c.view().can_adjust_preview&&pr_sounds_client_preview_level_v1(c.p,100,1)==PR_SOUNDS_CLIENT_BUSY);
 uint64_t ttl=number(text(first),"ttl_ms");
 for(uint32_t outcome:{PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK,PR_SOUNDS_CLIENT_SEND_INTERRUPTED}) {
  assert(pr_sounds_client_sent_v1(c.p,first.connection_epoch,first.token,outcome,0)==PR_SOUNDS_CLIENT_OK);
  advance(now()+2000000ULL);const auto next=c.packet();const auto wire=text(next);
  assert(next.token==first.token&&next.original_until_boottime_ns==until&&number(wire,"until_boottime_ns")==until);
  assert(number(wire,"level_percent")==50&&wire.find("\"muted\":false")!=std::string::npos&&wire.find("\"route\":\"media-a7\"")!=std::string::npos);
  assert(number(wire,"ttl_ms")<ttl);ttl=number(wire,"ttl_ms");
 }
 c.sent(c.packet());assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("starting")+"}")==PR_SOUNDS_CLIENT_OK);
 refresh(c);assert(c.reply(status(state("idle")))==PR_SOUNDS_CLIENT_OK);
 assert(pr_sounds_client_preview_level_v1(c.p,50,1)==PR_SOUNDS_CLIENT_OK);
 assert(pr_sounds_client_preview_play_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 const auto muted=c.packet();assert(number(text(muted),"level_percent")==50&&text(muted).find("\"muted\":true")!=std::string::npos);
 pass("Play_level_and_mute_snapshot_survive_reprepare_and_next_choice");
}


static void localMuteToggle() {
 static_assert(PR_SOUNDS_CLIENT_PREVIEW_MUTE==7&&PR_SOUNDS_CLIENT_PREVIEW_MUTE_PENDING==35);
 assert(pr_sounds_client_preview_toggle_mute_v1(nullptr)==PR_SOUNDS_CLIENT_INVALID);
 Client c;online(c);assert(pr_sounds_client_preview_level_v1(c.p,50,0)==PR_SOUNDS_CLIENT_OK);
 auto before=c.view();assert(before.can_toggle_mute&&!before.mute_checked&&!before.mute_is_live&&!before.playback_mute_known&&before.preview_generation==0);
 assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 auto after=c.view();assert(after.selected_level_percent==50&&after.selected_muted==1&&after.mute_checked==1&&!after.mute_is_live&&after.can_toggle_mute);
 assert(after.stage==before.stage&&after.connection_epoch==before.connection_epoch&&after.request_kind==before.request_kind&&after.request_until_boottime_ns==before.request_until_boottime_ns&&after.queued_kind==0);
 PrSoundsClientPacketV1 empty{};empty.token=99;
 assert(pr_sounds_client_prepare_v1(c.p,&empty)==PR_SOUNDS_CLIENT_EMPTY&&empty.token==99);
 // Idle STATUS remains read-only: local choice must neither replace nor renew it.
 assert(pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 const auto inspect=c.packet();const auto status_before=c.view();
 assert(status_before.request_kind==PR_SOUNDS_CLIENT_STATUS&&status_before.can_toggle_mute);
 assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 const auto same_inspect=c.packet();after=c.view();
 assert(after.selected_level_percent==50&&!after.selected_muted&&!after.mute_checked&&!after.mute_is_live);
 assert(same_inspect.token==inspect.token&&same_inspect.original_until_boottime_ns==inspect.original_until_boottime_ns&&text(same_inspect)==text(inspect));
 assert(after.connection_epoch==status_before.connection_epoch&&after.request_kind==PR_SOUNDS_CLIENT_STATUS&&after.request_until_boottime_ns==status_before.request_until_boottime_ns&&after.queued_kind==0);
 c.sent(same_inspect);const auto receiving=c.view();
 assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);
 after=c.view();assert(after.stage==PR_SOUNDS_CLIENT_RECEIVING&&after.can_toggle_mute&&after.selected_muted==1&&after.mute_checked==1);
 assert(after.request_kind==PR_SOUNDS_CLIENT_STATUS&&after.request_until_boottime_ns==receiving.request_until_boottime_ns&&after.connection_epoch==receiving.connection_epoch&&after.queued_kind==0);
 assert(c.reply(status())==PR_SOUNDS_CLIENT_OK);
 assert(c.view().selected_level_percent==50&&c.view().selected_muted==1);
 assert(pr_sounds_client_prepare_v1(c.p,&empty)==PR_SOUNDS_CLIENT_EMPTY&&empty.token==99);
 pass("idle_mute_toggle_is_local_no_IPC_or_deadline_change");
}
static void mutePacket(const PrSoundsClientPacketV1&packet,uint64_t generation,bool muted) {
 const auto wire=text(packet);
 assert(wire.find("\"op\":\"media_preview_mute\"")!=std::string::npos);
 assert(number(wire,"generation")==generation&&number(wire,"until_boottime_ns")==packet.original_until_boottime_ns);
 assert(number(wire,"ttl_ms")>0&&number(wire,"ttl_ms")<=1000);
 assert(wire.find(muted?"\"muted\":true":"\"muted\":false")!=std::string::npos);
 assert(wire.find("\"route\"")==std::string::npos&&wire.find("\"level_percent\"")==std::string::npos);
}
static void liveMuteDeadlineAndAck() {
 constexpr uint64_t generation=9007199254740993ULL; // Exact u64, beyond double's integer range.
 Client c;assert(pr_sounds_client_preview_level_v1(c.p,50,1)==PR_SOUNDS_CLIENT_OK);
 online(c,status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"9007199254740993","false")));
 auto v=c.view();assert(v.preview_generation==generation&&v.playback_mute_known&&!v.playback_muted&&v.can_toggle_mute&&!v.mute_checked&&v.mute_is_live&&v.selected_level_percent==50&&v.selected_muted==1);
 const auto before=now();assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto after=now();
 auto first=c.packet();const auto until=first.original_until_boottime_ns;
 assert(until>=before+1000000000ULL&&until<=after+1000000000ULL);mutePacket(first,generation,true);
 v=c.view();assert(v.request_kind==PR_SOUNDS_CLIENT_PREVIEW_MUTE&&v.status==PR_SOUNDS_CLIENT_PREVIEW_MUTE_PENDING&&v.can_stop&&!v.can_toggle_mute&&v.queued_kind==0);
 assert(v.preview_generation==generation&&v.playback_mute_known&&!v.playback_muted&&!v.mute_checked&&v.mute_is_live&&v.selected_level_percent==50&&v.selected_muted==1);
 assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_BUSY);
 auto ttl=number(text(first),"ttl_ms");
 for(uint32_t outcome:{PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK,PR_SOUNDS_CLIENT_SEND_INTERRUPTED}) {
  assert(pr_sounds_client_sent_v1(c.p,first.connection_epoch,first.token,outcome,0)==PR_SOUNDS_CLIENT_OK);
  advance(now()+3000000ULL);const auto next=c.packet();mutePacket(next,generation,true);
  assert(next.token==first.token&&next.original_until_boottime_ns==until&&number(text(next),"ttl_ms")<ttl);ttl=number(text(next),"ttl_ms");
  const auto pending=c.view();assert(!pending.playback_muted&&!pending.mute_checked&&pending.playback_mute_known&&pending.selected_muted==1&&pending.selected_level_percent==50);
 }
 c.sent(c.packet());assert(!c.view().playback_muted&&!c.view().mute_checked&&c.view().status==PR_SOUNDS_CLIENT_PREVIEW_MUTE_PENDING);
 assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("playing",true,"\"media-a7\"","\"media-a7\"",true,"9007199254740993","true")+"}")==PR_SOUNDS_CLIENT_OK);
 v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_IDLE&&v.status==PR_SOUNDS_CLIENT_PREVIEW_PLAYING&&v.preview_generation==generation&&v.playback_mute_known&&v.playback_muted&&v.mute_checked&&v.mute_is_live&&v.can_toggle_mute&&v.selected_level_percent==50&&v.selected_muted==1&&v.queued_kind==0);
 // A second explicit action inverses CONFIRMED live policy, not local next choice.
 assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto unmute=c.packet();mutePacket(unmute,generation,false);
 assert(c.view().playback_muted&&c.view().mute_checked&&c.view().selected_muted==1);c.sent(unmute);
 assert(c.reply("{\"result\":\"media_preview\",\"state\":"+state("playing",true,"\"media-a7\"","\"media-a7\"",true,"9007199254740993","false")+"}")==PR_SOUNDS_CLIENT_OK);
 v=c.view();assert(!v.playback_muted&&!v.mute_checked&&v.playback_mute_known&&v.mute_is_live&&v.selected_muted==1&&v.selected_level_percent==50&&v.preview_generation==generation&&v.can_toggle_mute);
 PrSoundsClientPacketV1 empty{};empty.token=99;assert(pr_sounds_client_prepare_v1(c.p,&empty)==PR_SOUNDS_CLIENT_EMPTY&&empty.token==99);
 pass("live_mute_frozen_generation_decreasing_ttl_EAGAIN_and_confirmed_ack");
}
static void liveMuteWrongAck() {
 for(bool wrong_generation:{true,false}) {
  Client c;assert(pr_sounds_client_preview_level_v1(c.p,50,0)==PR_SOUNDS_CLIENT_OK);
  online(c,status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"41","false")));
  assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto request=c.packet();mutePacket(request,41,true);c.sent(request);
  const auto reply=state("playing",true,"\"media-a7\"","\"media-a7\"",true,wrong_generation?"42":"41",wrong_generation?"true":"false");
  assert(c.reply("{\"result\":\"media_preview\",\"state\":"+reply+"}")==PR_SOUNDS_CLIENT_REFUSED);
  const auto v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&!v.online&&!v.can_play&&!v.can_stop&&v.queued_kind==0);
  assert(v.preview_generation==0&&!v.playback_mute_known&&!v.playback_muted&&!v.mute_is_live&&v.selected_level_percent==50&&!v.selected_muted);
 }
 pass("live_mute_wrong_generation_or_boolean_ack_fails_closed",2);
}
static void liveMuteBusyAndStop() {
 Client status_pending;online(status_pending,status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"41","false")));refresh(status_pending);
 auto before=status_pending.view();assert(before.request_kind==PR_SOUNDS_CLIENT_STATUS&&!before.can_toggle_mute);
 assert(pr_sounds_client_preview_toggle_mute_v1(status_pending.p)==PR_SOUNDS_CLIENT_BUSY);
 auto after=status_pending.view();assert(after.request_kind==before.request_kind&&after.request_until_boottime_ns==before.request_until_boottime_ns&&after.queued_kind==0&&after.preview_generation==41&&!after.playback_muted&&!after.mute_checked&&!after.selected_muted);
 assert(status_pending.reply(status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"43","true")))==PR_SOUNDS_CLIENT_OK);
 PrSoundsClientPacketV1 empty{};empty.token=99;assert(pr_sounds_client_prepare_v1(status_pending.p,&empty)==PR_SOUNDS_CLIENT_EMPTY&&empty.token==99);
 assert(status_pending.view().preview_generation==43&&status_pending.view().playback_muted&&status_pending.view().mute_checked&&status_pending.view().can_toggle_mute&&!status_pending.view().selected_muted);
 Client effect;online(effect);const auto route=effect.view().requested;assert(pr_sounds_client_choose_v1(effect.p,&route)==PR_SOUNDS_CLIENT_OK);before=effect.view();
 assert(pr_sounds_client_preview_toggle_mute_v1(effect.p)==PR_SOUNDS_CLIENT_BUSY);after=effect.view();
 assert(after.request_kind==before.request_kind&&after.request_until_boottime_ns==before.request_until_boottime_ns&&after.queued_kind==0&&!after.selected_muted&&!after.can_toggle_mute);
 Client queued;online(queued,status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"41","false")));refresh(queued);assert(pr_sounds_client_choose_v1(queued.p,&route)==PR_SOUNDS_CLIENT_OK);before=queued.view();
 assert(before.queued_kind==PR_SOUNDS_CLIENT_ROUTE_INTENT&&pr_sounds_client_preview_toggle_mute_v1(queued.p)==PR_SOUNDS_CLIENT_BUSY);after=queued.view();
 assert(after.request_kind==before.request_kind&&after.request_until_boottime_ns==before.request_until_boottime_ns&&after.queued_kind==before.queued_kind&&after.queued_until_boottime_ns==before.queued_until_boottime_ns&&!after.can_toggle_mute&&!after.playback_muted&&!after.mute_checked);
 for(bool sent:{false,true}) {
  Client c;online(c,status(state("playing",true,"\"media-a7\"","\"media-a7\"",true,"41","false")));
  assert(pr_sounds_client_preview_toggle_mute_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto mute=c.packet();if(sent)c.sent(mute);
  assert(c.view().can_stop&&pr_sounds_client_preview_stop_v1(c.p)==PR_SOUNDS_CLIENT_OK);
  auto v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&v.status==PR_SOUNDS_CLIENT_PREVIEW_CANCELLED&&!v.online&&!v.can_play&&!v.can_stop&&v.queued_kind==0&&v.preview_generation==0&&!v.playback_mute_known&&!v.mute_is_live);
  assert(pr_sounds_client_reconnect_v1(c.p)==PR_SOUNDS_CLIENT_OK);const auto epoch=c.view().connection_epoch;
  assert(pr_sounds_client_connected_v1(c.p,epoch)==PR_SOUNDS_CLIENT_OK);const auto inspect=c.packet();assert(text(inspect)=="{\"offset\":0,\"op\":\"media_status\"}");c.sent(inspect);
  assert(c.reply(status())==PR_SOUNDS_CLIENT_OK&&c.view().queued_kind==0);assert(pr_sounds_client_prepare_v1(c.p,&empty)==PR_SOUNDS_CLIENT_EMPTY&&empty.token==99);
 }
 pass("live_mute_busy_STATUS_no_queue_and_Stop_withdraws_inflight",5);
}

int main(){work=now()+5000000000ULL;projection();playDeadline();noQueuedPlay();queuedStop();cancelInflight();deadlinesAndFailure();replyContext();localLevel();levelBusy();levelSnapshot();localMuteToggle();liveMuteDeadlineAndAck();liveMuteWrongAck();liveMuteBusyAndStop();bounded();std::puts("SOUNDS_PREVIEW_ALL_PASS groups=14 authored_CPU_facts_only=true");}
