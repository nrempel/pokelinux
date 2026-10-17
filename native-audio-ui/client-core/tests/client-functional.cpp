#include "pokelinux_sounds_client.h"
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>
#include <algorithm>
#include <type_traits>
#include <time.h>
#include <unistd.h>
static uint64_t now(){timespec t{};assert(clock_gettime(CLOCK_BOOTTIME,&t)==0&&t.tv_sec>=0&&t.tv_nsec>=0&&t.tv_nsec<1000000000);return uint64_t(t.tv_sec)*1000000000ULL+uint64_t(t.tv_nsec);}
static uint64_t work=0,close_until=0;
static void bounded(){assert(now()<work);}
static void pass(const char*s){bounded();std::printf("SOUNDS_CLIENT_PASS %s\n",s);}
static PrSoundsClientIdV1 id(const char*s){PrSoundsClientIdV1 r{};r.length=uint32_t(std::strlen(s));assert(r.length>0&&r.length<=64);std::memcpy(r.bytes,s,r.length);return r;}
static bool same(const PrSoundsClientIdV1&a,const char*s){return a.length<=64&&a.length==std::strlen(s)&&std::memcmp(a.bytes,s,a.length)==0;}
static unsigned ttl(const PrSoundsClientPacketV1&p){const std::string s(reinterpret_cast<const char*>(p.bytes),p.length);const auto n=s.find("\"ttl_ms\":");assert(n!=std::string::npos);const auto begin=n+9;assert(begin<s.size()&&s[begin]>='0'&&s[begin]<='9');unsigned v=0;for(auto i=begin;i<s.size()&&s[i]>='0'&&s[i]<='9';++i)v=v*10+unsigned(s[i]-'0');return v;}
static std::string text(const PrSoundsClientPacketV1&p){assert(p.length>0&&p.length<=4096&&p.reserved==0);return {reinterpret_cast<const char*>(p.bytes),p.length};}
struct Client{
 PrSoundsClient*p=nullptr;
 Client(){assert(pr_sounds_client_new_v1(&p)==PR_SOUNDS_CLIENT_OK&&p);}
 ~Client(){pr_sounds_client_free_v1(p);}
 Client(const Client&)=delete;Client&operator=(const Client&)=delete;
 PrSoundsClientViewV1 view()const{PrSoundsClientViewV1 v{};assert(pr_sounds_client_view_v1(p,&v)==PR_SOUNDS_CLIENT_OK&&v.reserved==0&&v.route_count<=32);return v;}
 PrSoundsClientPacketV1 packet(){PrSoundsClientPacketV1 q{};assert(pr_sounds_client_prepare_v1(p,&q)==PR_SOUNDS_CLIENT_OK);assert(q.connection_epoch==view().connection_epoch&&q.token>0);(void)text(q);return q;}
 void sent(const PrSoundsClientPacketV1&q){assert(pr_sounds_client_sent_v1(p,q.connection_epoch,q.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,q.length)==PR_SOUNDS_CLIENT_OK);}
 int reply(const std::string&s,uint64_t epoch=0){return pr_sounds_client_reply_v1(p,epoch?epoch:view().connection_epoch,reinterpret_cast<const uint8_t*>(s.data()),uint32_t(s.size()));}
 uint64_t begin(){const auto before=now();assert(pr_sounds_client_start_v1(p)==PR_SOUNDS_CLIENT_OK);const auto after=now();auto v=view();assert(v.stage==PR_SOUNDS_CLIENT_CONNECTING&&!v.online&&v.connection_epoch>0&&v.request_kind==PR_SOUNDS_CLIENT_CONNECT);assert(v.request_until_boottime_ns>=before+1000000000ULL&&v.request_until_boottime_ns<=after+1000000000ULL);assert(pr_sounds_client_connected_v1(p,v.connection_epoch)==PR_SOUNDS_CLIENT_OK);const auto q=packet();assert(text(q)=="{\"offset\":0,\"op\":\"media_status\"}"&&q.original_until_boottime_ns==v.request_until_boottime_ns);sent(q);return v.connection_epoch;}
};
static std::string state(const char*requested="null",const char*applied="null",bool pending=false,bool connected=true,bool available=true){return std::string("{\"connected\":")+(connected?"true":"false")+",\"available\":"+(available?"true":"false")+",\"pending\":"+(pending?"true":"false")+",\"requested\":"+requested+",\"applied\":"+applied+"}";}
static std::string rows(int offset=0,int n=2,const std::string&label="Output") {std::string s="[";for(int i=0;i<n;++i){char name[32];if(offset==0&&n==2)std::snprintf(name,sizeof(name),"%s",i?"media-z2":"media-a7");else std::snprintf(name,sizeof(name),"route-%02d",offset+i);if(i)s+=",";s+="{\"id\":\""+std::string(name)+"\",\"label\":\""+label+"\"}";}return s+"]";}
static std::string status(const std::string&r=rows(),const char*next="null",bool choose=true,bool reconnect=false,const std::string&s=state()){return "{\"result\":\"media_status\",\"state\":"+s+",\"routes\":"+r+",\"next_offset\":"+next+",\"can_choose\":"+(choose?"true":"false")+",\"can_reconnect\":"+(reconnect?"true":"false")+"}";}
static void online(Client&c,const std::string&s=status()){c.begin();assert(c.reply(s)==PR_SOUNDS_CLIENT_OK);auto v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_IDLE&&v.online&&v.request_kind==PR_SOUNDS_CLIENT_NO_REQUEST);}
static void refresh(Client&c){assert(pr_sounds_client_refresh_v1(c.p)==PR_SOUNDS_CLIENT_OK);auto p=c.packet();assert(text(p)=="{\"offset\":0,\"op\":\"media_status\"}");c.sent(p);}
static void select(Client&c,const char*name="media-a7"){auto route=id(name);assert(pr_sounds_client_choose_v1(c.p,&route)==PR_SOUNDS_CLIENT_OK);}
static void rejected(const std::string&s){Client c;c.begin();assert(c.reply(s)==PR_SOUNDS_CLIENT_REFUSED);auto v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&!v.online&&!v.available&&!v.can_choose&&v.can_reconnect&&v.route_count==0&&!v.requested_present&&!v.applied_present&&v.status==PR_SOUNDS_CLIENT_REPLY_REFUSED);}
static void replace(std::string&s,const std::string&from,const std::string&to){const auto n=s.find(from);assert(n!=std::string::npos);s.replace(n,from.size(),to);}
static void direct_model(){
 Client c;online(c);auto v=c.view();assert(v.route_count==2&&v.can_choose&&!v.can_reconnect&&same(v.routes[0].id,"media-a7")&&v.status==PR_SOUNDS_CLIENT_CHOOSE_OUTPUT);
 auto unknown=id("private.sink");assert(pr_sounds_client_choose_v1(c.p,&unknown)==PR_SOUNDS_CLIENT_REFUSED);assert(c.view().stage==PR_SOUNDS_CLIENT_IDLE);
 select(c);auto p=c.packet();assert(text(p)=="{\"op\":\"media_route\",\"request\":{\"route\":\"media-a7\",\"ttl_ms\":1000}}");assert(!c.view().applied_present);c.sent(p);assert(c.view().status==PR_SOUNDS_CLIENT_OUTPUT_REQUESTED);
 assert(c.reply("{\"result\":\"media_selected\",\"state\":"+state("\"media-a7\"","null",true)+"}")==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.requested_present&&same(v.requested,"media-a7")&&!v.applied_present&&v.status==PR_SOUNDS_CLIENT_APPLYING);
 refresh(c);assert(c.reply(status(rows(),"null",true,false,state("\"media-a7\"","\"media-a7\"")))==PR_SOUNDS_CLIENT_OK);assert(c.view().applied_present&&c.view().status==PR_SOUNDS_CLIENT_APPLIED);
 refresh(c);assert(c.reply(status(rows(),"null",true,false,state("\"media-a7\"")))==PR_SOUNDS_CLIENT_OK);assert(c.view().status==PR_SOUNDS_CLIENT_INTERRUPTED&&c.view().requested_present&&!c.view().applied_present);
 for(const char*result:{"denied","unavailable"}){refresh(c);assert(c.reply(status(rows(),"null",true,false,state("\"media-a7\"","\"media-a7\"")))==PR_SOUNDS_CLIENT_OK);select(c);p=c.packet();c.sent(p);assert(c.reply(std::string("{\"result\":\"")+result+"\"}")==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.route_count==2&&v.requested_present&&same(v.requested,"media-a7")&&!v.applied_present&&!v.can_choose&&!v.can_reconnect&&v.online);assert(v.status==(std::strcmp(result,"denied")==0?PR_SOUNDS_CLIENT_DENIED:PR_SOUNDS_CLIENT_UNAVAILABLE));}
 pass("direct_request_and_accepted_model");
}
static void parser_pages(){
 Client c;online(c);refresh(c);const auto original=c.view().request_until_boottime_ns;assert(c.reply(status(rows(0,8),"8.0",false,true,state("null","null",false,false,false)))==PR_SOUNDS_CLIENT_OK);auto v=c.view();assert(v.route_count==2&&v.can_choose&&v.request_until_boottime_ns==original);auto p=c.packet();assert(text(p)=="{\"offset\":8,\"op\":\"media_status\"}"&&p.original_until_boottime_ns==original);c.sent(p);assert(c.reply(status(rows(8,2),"null",false,true,state("null","null",false,false,false)))==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.route_count==10&&!v.can_choose&&v.can_reconnect&&!v.available&&v.status==PR_SOUNDS_CLIENT_UNAVAILABLE_INSPECT);
 Client max;max.begin();for(int offset=0;offset<32;offset+=8){const auto n=std::to_string(offset+8);assert(max.reply(status(rows(offset,8),offset==24?"null":n.c_str()))==PR_SOUNDS_CLIENT_OK);if(offset!=24){assert(max.view().route_count==0);auto q=max.packet();max.sent(q);}}assert(max.view().route_count==32);
 for(const auto&fault:std::vector<std::pair<std::string,std::string>>{{",\"requested\":null",""},{",\"applied\":null",""},{",\"next_offset\":null",""},{"\"requested\":null","\"requested\":false"},{"\"can_choose\":true","\"can_choose\":2"},{"\"result\":\"media_status\"","\"result\":\"ok\""},{"\"label\":\"Output\"","\"label\":\"Output\",\"target\":\"private.sink\""}}){auto s=status();replace(s,fault.first,fault.second);rejected(s);}
 rejected(status(rows(0,9)));rejected(status("[{\"id\":\"same\",\"label\":\"A\"},{\"id\":\"same\",\"label\":\"B\"}]"));rejected(status("[{\"id\":\"z\",\"label\":\"A\"},{\"id\":\"a\",\"label\":\"B\"}]"));rejected(status("[{\"id\":\""+std::string(65,'a')+"\",\"label\":\"A\"}]"));
 for(const char*next:{"8.000000000000002","\"8\"","7"})rejected(status(rows(0,8),next));
 for(const char*next:{"8","8.0","8e0"}){Client k;k.begin();assert(k.reply(status(rows(0,8),next))==PR_SOUNDS_CLIENT_OK);auto q=k.packet();k.sent(q);assert(k.reply(status(rows(8,1)))==PR_SOUNDS_CLIENT_OK&&k.view().route_count==9);}
 auto s=status();replace(s,"\"result\":\"media_status\"","\"result\":\"bad\",\"result\":\"media_status\"");Client duplicate;online(duplicate,s);assert(duplicate.view().route_count==2);s=status();replace(s,"\"result\":\"media_status\"","\"result\":\"media_status\",\"result\":\"bad\"");rejected(s);
 auto labeldup=status();replace(labeldup,"\"label\":\"Output\"","\"label\":\"\\u0000\",\"label\":\"Output\"");Client label_last;online(label_last,labeldup);assert(label_last.view().route_count==2);labeldup=status();replace(labeldup,"\"label\":\"Output\"","\"label\":\"Output\",\"label\":\"\\u0000\"");rejected(labeldup);
 for(const std::string&label:{std::string(""),std::string("   "),std::string("\\u00a0"),std::string("\\u0085"),std::string("x\\u0000y"),std::string("\\ud800"),std::string("\\udc00"),std::string(97,'x'),std::string(1,char(0xff))})rejected(status(rows(0,1,label)));
 for(const std::string&label:{std::string("\\u200b"),std::string("\\ufeff"),std::string("\\ud83d\\ude00"),std::string(96,'x')}){Client k;online(k,status(rows(0,1,label)));assert(k.view().route_count==1);}
 std::string emoji;for(int i=0;i<24;++i)emoji+="\\ud83d\\ude00";Client unicode;online(unicode,status(rows(0,1,emoji)));assert(unicode.view().routes[0].label.length==96);rejected(status(rows(0,1,emoji+"\\ud83d\\ude00")));
 pass("atomic_pages_and_explicit_wire_semantics");
}
static void send_fences(){
 Client c;online(c,status(rows(),"null",true,false,state("\"media-a7\"","\"media-a7\"")));select(c);auto a=c.packet();const auto epoch=a.connection_epoch,token=a.token,until=a.original_until_boottime_ns;assert(c.view().applied_present);std::memset(a.bytes,0xa5,sizeof(a.bytes));auto b=c.packet();assert(b.connection_epoch==epoch&&b.token==token&&b.original_until_boottime_ns==until&&text(b).find("media-a7")!=std::string::npos);
 for(const auto outcome:{PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK,PR_SOUNDS_CLIENT_SEND_INTERRUPTED}){assert(pr_sounds_client_sent_v1(c.p,epoch,token,outcome,0)==PR_SOUNDS_CLIENT_OK);auto q=c.packet();assert(q.token==token&&q.original_until_boottime_ns==until&&c.view().applied_present);}
 auto q=c.packet();assert(pr_sounds_client_sent_v1(c.p,epoch,token+1,PR_SOUNDS_CLIENT_SEND_COMPLETE,q.length)==PR_SOUNDS_CLIENT_STALE);assert(c.view().applied_present);c.sent(q);assert(!c.view().applied_present&&c.view().stage==PR_SOUNDS_CLIENT_RECEIVING);assert(pr_sounds_client_sent_v1(c.p,epoch,token,PR_SOUNDS_CLIENT_SEND_COMPLETE,q.length)==PR_SOUNDS_CLIENT_STALE);
 for(int fault=0;fault<2;++fault){Client k;online(k);select(k);auto p=k.packet();assert(pr_sounds_client_sent_v1(k.p,p.connection_epoch,p.token,fault?PR_SOUNDS_CLIENT_SEND_FAILED:PR_SOUNDS_CLIENT_SEND_COMPLETE,fault?0:p.length-1)==PR_SOUNDS_CLIENT_REFUSED);assert(k.view().stage==PR_SOUNDS_CLIENT_CLOSED&&!k.view().applied_present);}
 PrSoundsClientPacketV1 untouched{};std::memset(&untouched,0x55,sizeof(untouched));const auto saved=untouched;assert(pr_sounds_client_prepare_v1(c.p,&untouched)==PR_SOUNDS_CLIENT_EMPTY&&std::memcmp(&untouched,&saved,sizeof(saved))==0);
 pass("send_reports_and_copied_packet_fences");
}
static void queued_fresh(){
 Client c;online(c,status(rows(),"null",true,true,state("\"media-a7\"","\"media-a7\"")));refresh(c);auto route=id("media-a7");const auto entered=now();assert(pr_sounds_client_choose_v1(c.p,&route)==PR_SOUNDS_CLIENT_OK);const auto returned=now();route=id("media-z2");auto v=c.view();assert(v.queued_kind==PR_SOUNDS_CLIENT_ROUTE_INTENT&&v.queued_until_boottime_ns>=entered+1000000000ULL&&v.queued_until_boottime_ns<=returned+1000000000ULL);assert(!v.can_choose&&!v.can_reconnect&&v.applied_present);assert(pr_sounds_client_reconnect_v1(c.p)==PR_SOUNDS_CLIENT_BUSY);assert(pr_sounds_client_choose_v1(c.p,&route)==PR_SOUNDS_CLIENT_BUSY);
 PrSoundsClientPacketV1 out{};out.token=0x55;assert(pr_sounds_client_prepare_v1(c.p,&out)==PR_SOUNDS_CLIENT_EMPTY&&out.token==0x55);assert(c.reply(status(rows(0,8),"8"))==PR_SOUNDS_CLIENT_OK);auto page=c.packet();assert(text(page).find("media_route")==std::string::npos);c.sent(page);assert(c.view().queued_kind==PR_SOUNDS_CLIENT_ROUTE_INTENT);
 // Final catalog lacks the selected route: cancellation, no mutation or replay.
 assert(c.reply(status(rows(8,2)))==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_IDLE&&v.queued_kind==0&&v.route_count==10);out.token=0x66;assert(pr_sounds_client_prepare_v1(c.p,&out)==PR_SOUNDS_CLIENT_EMPTY&&out.token==0x66);
 for(int fault=0;fault<4;++fault){Client k;online(k,status(rows(),"null",true,true));refresh(k);select(k);assert(k.reply(status(fault==3?"[{\"id\":\"media-z2\",\"label\":\"Other\"}]":rows(),"null",fault!=0,true,state("null","null",false,fault!=2,fault!=1)))==PR_SOUNDS_CLIENT_OK);assert(k.view().queued_kind==0&&k.view().stage==PR_SOUNDS_CLIENT_IDLE);}
 Client keep;online(keep,status(rows(),"null",true,true,state("\"media-a7\"","\"media-a7\"")));refresh(keep);select(keep);assert(keep.reply(status(rows(),"null",true,true,state("\"media-a7\"","\"media-a7\"")))==PR_SOUNDS_CLIENT_OK);auto first=keep.packet();assert(keep.view().applied_present&&text(first).find("media-a7")!=std::string::npos&&first.original_until_boottime_ns==keep.view().queued_until_boottime_ns);const auto later=now()+2000000ULL;while(now()<later)bounded();auto second=keep.packet();assert(first.token==second.token&&first.original_until_boottime_ns==second.original_until_boottime_ns&&ttl(first)>0&&ttl(first)<=1000&&ttl(second)>0&&ttl(second)<ttl(first));assert(pr_sounds_client_sent_v1(keep.p,second.connection_epoch,second.token,PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK,0)==PR_SOUNDS_CLIENT_OK);assert(keep.view().applied_present);keep.sent(keep.packet());assert(!keep.view().applied_present&&keep.view().queued_kind==0);assert(pr_sounds_client_sent_v1(keep.p,second.connection_epoch,second.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,second.length)==PR_SOUNDS_CLIENT_STALE);
 Client reconnect;online(reconnect,status(rows(),"null",false,true,state("null","null",false,false,false)));refresh(reconnect);assert(pr_sounds_client_reconnect_v1(reconnect.p)==PR_SOUNDS_CLIENT_OK);assert(reconnect.reply(status(rows(),"null",false,true,state("null","null",false,false,false)))==PR_SOUNDS_CLIENT_OK);auto rp=reconnect.packet();assert(text(rp)=="{\"op\":\"media_reconnect\"}");reconnect.sent(rp);assert(reconnect.view().queued_kind==0);const auto old=rp.original_until_boottime_ns;const auto before=now();assert(reconnect.reply("{\"result\":\"ok\"}")==PR_SOUNDS_CLIENT_OK);auto read=reconnect.packet();assert(text(read)=="{\"offset\":0,\"op\":\"media_status\"}"&&read.original_until_boottime_ns>old&&read.original_until_boottime_ns>=before+1000000000ULL);reconnect.sent(read);assert(reconnect.reply(status())==PR_SOUNDS_CLIENT_OK);assert(reconnect.view().queued_kind==0&&reconnect.view().stage==PR_SOUNDS_CLIENT_IDLE);
 Client lost;online(lost,status(rows(),"null",false,true,state("null","null",false,false,false)));refresh(lost);assert(pr_sounds_client_reconnect_v1(lost.p)==PR_SOUNDS_CLIENT_OK);assert(lost.reply(status(rows(),"null",false,false))==PR_SOUNDS_CLIENT_OK);assert(lost.view().queued_kind==0&&lost.view().stage==PR_SOUNDS_CLIENT_IDLE);
 pass("queued_first_intent_and_final_freshness");
}
static void incarnation(){
 Client c;online(c,status(rows(),"null",true,true));refresh(c);select(c);auto v=c.view();const auto old=v.connection_epoch;assert(pr_sounds_client_disconnected_v1(c.p,old,PR_SOUNDS_CLIENT_PACKET_REFUSED)==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&!v.requested_present&&!v.applied_present&&v.route_count==0&&v.queued_kind==0&&v.can_reconnect);
 assert(pr_sounds_client_start_v1(c.p)==PR_SOUNDS_CLIENT_REFUSED);assert(pr_sounds_client_reconnect_v1(c.p)==PR_SOUNDS_CLIENT_OK);v=c.view();assert(v.stage==PR_SOUNDS_CLIENT_CONNECTING&&v.connection_epoch>old);const auto fresh=v.connection_epoch;assert(pr_sounds_client_connected_v1(c.p,old)==PR_SOUNDS_CLIENT_STALE);assert(pr_sounds_client_connected_v1(c.p,fresh)==PR_SOUNDS_CLIENT_OK);auto p=c.packet();assert(text(p)=="{\"offset\":0,\"op\":\"media_status\"}");assert(c.reply(status(),old)==PR_SOUNDS_CLIENT_STALE);assert(pr_sounds_client_disconnected_v1(c.p,old,PR_SOUNDS_CLIENT_PACKET_REFUSED)==PR_SOUNDS_CLIENT_STALE);assert(pr_sounds_client_sent_v1(c.p,old,p.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,p.length)==PR_SOUNDS_CLIENT_STALE);assert(c.view().connection_epoch==fresh&&c.view().stage==PR_SOUNDS_CLIENT_SENDING);c.sent(p);assert(c.reply(status())==PR_SOUNDS_CLIENT_OK);assert(c.view().queued_kind==0&&!c.view().requested_present&&!c.view().applied_present);
 refresh(c);select(c);assert(c.reply(status())==PR_SOUNDS_CLIENT_OK);auto newer=c.packet();assert(newer.token!=p.token);assert(pr_sounds_client_sent_v1(c.p,fresh,p.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,p.length)==PR_SOUNDS_CLIENT_STALE);assert(c.view().queued_kind==PR_SOUNDS_CLIENT_ROUTE_INTENT);c.sent(newer);assert(c.view().queued_kind==0);
 Client cancel;online(cancel);select(cancel);auto cancelled=cancel.packet();assert(pr_sounds_client_disconnected_v1(cancel.p,cancelled.connection_epoch,PR_SOUNDS_CLIENT_PACKET_REFUSED)==PR_SOUNDS_CLIENT_OK);assert(pr_sounds_client_reconnect_v1(cancel.p)==PR_SOUNDS_CLIENT_OK);const auto live=cancel.view().connection_epoch;assert(pr_sounds_client_connected_v1(cancel.p,live)==PR_SOUNDS_CLIENT_OK);auto inspect=cancel.packet();assert(inspect.token!=cancelled.token);assert(pr_sounds_client_sent_v1(cancel.p,live,cancelled.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,cancelled.length)==PR_SOUNDS_CLIENT_STALE);assert(cancel.view().stage==PR_SOUNDS_CLIENT_SENDING&&cancel.view().connection_epoch==live);cancel.sent(inspect);assert(cancel.reply(status())==PR_SOUNDS_CLIENT_OK&&cancel.view().queued_kind==0);
 pass("connection_epoch_cancel_and_no_replay");
}
static void expiry(){
 Client direct,page,queued,unsent,late_reconnect,initial,queued_reconnect,queued_reply;
 online(direct);select(direct);direct.sent(direct.packet());
 page.begin();assert(page.reply(status(rows(0,8),"8"))==PR_SOUNDS_CLIENT_OK);page.sent(page.packet());
 online(queued,status(rows(),"null",true,true));refresh(queued);select(queued);
 online(unsent);select(unsent);auto pending=unsent.packet();
 online(late_reconnect,status(rows(),"null",false,true));assert(pr_sounds_client_reconnect_v1(late_reconnect.p)==PR_SOUNDS_CLIENT_OK);late_reconnect.sent(late_reconnect.packet());
 assert(pr_sounds_client_start_v1(initial.p)==PR_SOUNDS_CLIENT_OK&&initial.view().stage==PR_SOUNDS_CLIENT_CONNECTING);
 online(queued_reconnect,status(rows(),"null",false,true));refresh(queued_reconnect);assert(pr_sounds_client_reconnect_v1(queued_reconnect.p)==PR_SOUNDS_CLIENT_OK);
 online(queued_reply);refresh(queued_reply);select(queued_reply);const auto queued_until=queued_reply.view().queued_until_boottime_ns;assert(queued_reply.reply(status())==PR_SOUNDS_CLIENT_OK);auto qe=queued_reply.packet();assert(qe.original_until_boottime_ns==queued_until);queued_reply.sent(qe);
 uint64_t until=0;for(Client*c:{&direct,&page,&queued,&unsent,&late_reconnect,&initial,&queued_reconnect,&queued_reply}){auto v=c->view();until=std::max(until,std::max(v.request_until_boottime_ns,v.queued_until_boottime_ns));}
 while(now()<until){bounded();timespec pause{0,1000000};(void)nanosleep(&pause,nullptr);}assert(now()>=until); // Clock comparison, not elapsed-sleep inference.
 assert(direct.view().stage==PR_SOUNDS_CLIENT_RECEIVING); // Pure view cannot transition/renew.
 assert(direct.reply("{\"result\":\"media_selected\",\"state\":"+state("\"media-a7\"","\"media-a7\"")+"}")==PR_SOUNDS_CLIENT_EXPIRED);
 assert(page.reply(status(rows(8,2)))==PR_SOUNDS_CLIENT_EXPIRED);
 assert(queued_reply.reply("{\"result\":\"media_selected\",\"state\":"+state("\"media-a7\"","\"media-a7\"")+"}")==PR_SOUNDS_CLIENT_EXPIRED);
 for(Client*c:{&queued,&queued_reconnect}){PrSoundsClientPacketV1 output{};output.token=0x77;assert(pr_sounds_client_prepare_v1(c->p,&output)==PR_SOUNDS_CLIENT_EXPIRED&&output.token==0x77);assert(c->view().queued_kind==0);} // Expiry before incomplete-status EMPTY.
 assert(pr_sounds_client_sent_v1(unsent.p,pending.connection_epoch,pending.token,PR_SOUNDS_CLIENT_SEND_COMPLETE,pending.length)==PR_SOUNDS_CLIENT_EXPIRED);
 assert(late_reconnect.reply("{\"result\":\"ok\"}")==PR_SOUNDS_CLIENT_EXPIRED);assert(pr_sounds_client_tick_v1(initial.p)==PR_SOUNDS_CLIENT_EXPIRED);
 for(Client*c:{&direct,&page,&queued,&unsent,&late_reconnect,&initial,&queued_reconnect,&queued_reply}){auto v=c->view();assert(v.stage==PR_SOUNDS_CLIENT_CLOSED&&v.route_count==0&&!v.requested_present&&!v.applied_present&&v.queued_kind==0);}
 pass("original_boottime_deadlines_without_timer_delivery");
}
int main(){
 static_assert(sizeof(PrSoundsClientIdV1)==68&&sizeof(PrSoundsClientLabelV1)==100&&sizeof(PrSoundsClientRowV1)==168);
 static_assert(sizeof(PrSoundsClientViewV1)==5584&&alignof(PrSoundsClientViewV1)==8&&offsetof(PrSoundsClientViewV1,routes)==208);
 static_assert(sizeof(PrSoundsClientPacketV1)==4128&&offsetof(PrSoundsClientPacketV1,bytes)==32&&std::is_trivially_copyable_v<PrSoundsClientPacketV1>);
 const uint64_t started=now();work=started+5000000000ULL;close_until=started+8000000000ULL;
 direct_model();parser_pages();send_fences();queued_fresh();incarnation();expiry();const auto done=now();assert(done<work&&done<close_until);
 std::printf("SOUNDS_CLIENT_ALL_PASS groups=6 pid=%ld original_work_until_ns=%llu original_close_until_ns=%llu completed_boottime_ns=%llu authored_CPU_facts_only=true\n",long(getpid()),static_cast<unsigned long long>(work),static_cast<unsigned long long>(close_until),static_cast<unsigned long long>(done));
}
