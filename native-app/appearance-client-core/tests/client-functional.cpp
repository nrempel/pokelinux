// Independent policy consumer. Authored native-result reports are CPU facts only.
// Actual credentials/SCM_RIGHTS/QObject/QML remain covered by unchanged Qt29.
#include "pokelinux_appearance_client.h"
#include <array>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <vector>
#include <utility>
#include <cerrno>
#include <climits>
#include <poll.h>
#include <sys/timerfd.h>
#include <time.h>
#include <unistd.h>

static_assert(sizeof(PrAppearanceRgba16V1)==8);
static_assert(sizeof(PrAppearanceStepV1)==72);
static_assert(sizeof(PrAppearanceViewV1)==232);
static_assert(std::is_trivially_copyable_v<PrAppearanceStepV1>);
static_assert(std::is_trivially_copyable_v<PrAppearanceViewV1>);
static uint64_t work_until=0,close_until=0;
static bool fail_next_boottime=false;
extern "C" int __real_clock_gettime(clockid_t,timespec*);
extern "C" int __wrap_clock_gettime(clockid_t clock,timespec*out){if(clock==CLOCK_BOOTTIME&&fail_next_boottime){fail_next_boottime=false;errno=EIO;return -1;}return __real_clock_gettime(clock,out);}
static uint64_t now(){timespec t{};if(clock_gettime(CLOCK_BOOTTIME,&t)||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000)throw std::runtime_error("real BOOTTIME");const auto s=uint64_t(t.tv_sec),n=uint64_t(t.tv_nsec);if(s>(UINT64_MAX-n)/1000000000ULL)throw std::runtime_error("clock overflow");return s*1000000000ULL+n;}
static void need(bool value,const char *label){if(!value)throw std::runtime_error(label);if(work_until&&now()>=work_until)throw std::runtime_error("original work bound");}
static void wait_until(uint64_t until){
 need(until<work_until,"timer within original work");const int fd=timerfd_create(CLOCK_BOOTTIME,TFD_CLOEXEC|TFD_NONBLOCK);need(fd>=0,"actual timerfd");
 itimerspec t{};t.it_value.tv_sec=time_t(until/1000000000ULL);t.it_value.tv_nsec=long(until%1000000000ULL);
 if(timerfd_settime(fd,TFD_TIMER_ABSTIME,&t,nullptr)){close(fd);throw std::runtime_error("absolute timer arm");}
 bool fired=false;
 while(true){const auto at=now();if(at>=work_until)break;pollfd p{fd,POLLIN,0};const auto remaining=(work_until-at+999999ULL)/1000000ULL;const int result=poll(&p,1,int(remaining>uint64_t(INT_MAX)?INT_MAX:remaining));if(result<0&&errno==EINTR)continue;if(result<=0)break;uint64_t count=0;if(read(fd,&count,sizeof(count))==ssize_t(sizeof(count))&&count){fired=true;break;}}
 const int closed=close(fd);need(closed==0&&fired&&now()>=until,"actual timer delivery and fd close");
}
struct Core {
 PrAppearanceClient *p=nullptr;
 Core(){need(pr_appearance_client_new_v1(&p)==PR_APPEARANCE_OK&&p,"new");}
 Core(const Core&)=delete;Core&operator=(const Core&)=delete;
 ~Core(){if(p){pr_appearance_client_stop_v1(p);pr_appearance_client_free_v1(p);}}
 PrAppearanceViewV1 view()const{PrAppearanceViewV1 v{};need(pr_appearance_client_view_v1(p,&v)==PR_APPEARANCE_OK,"copied view");return v;}
 bool available()const{uint32_t value=9;need(pr_appearance_client_available_v1(p,&value)==PR_APPEARANCE_OK&&value<=1,"fresh available");return value!=0;}
};
static PrAppearanceStepV1 take(Core&c,uint32_t action){PrAppearanceStepV1 p{};need(pr_appearance_client_step_v1(c.p,&p)==PR_APPEARANCE_OK&&p.action==action,"exact effect");return p;}
static void ready(Core&c,uint64_t generation,uint32_t kind){need(pr_appearance_client_ready_v1(c.p,generation,kind)==PR_APPEARANCE_OK,"actual notifier delivery");}
static void report(Core&c,const PrAppearanceStepV1&p,uint32_t outcome,uint32_t bytes=0){need(pr_appearance_client_report_v1(c.p,&p,outcome,bytes)==PR_APPEARANCE_OK,"native-result report");}
static PrAppearanceStepV1 send_plan(Core&c){need(pr_appearance_client_refresh_v1(c.p)==PR_APPEARANCE_OK,"idle refresh");PrAppearanceStepV1 p{};need(pr_appearance_client_step_v1(c.p,&p)==PR_APPEARANCE_OK,"start effect");if(p.action==PR_APPEARANCE_CONNECT){report(c,p,PR_APPEARANCE_CONNECTED);p=take(c,PR_APPEARANCE_SEND);}need(p.action==PR_APPEARANCE_SEND,"snapshot send");return p;}
static PrAppearanceStepV1 receive_plan(Core&c){const auto p=send_plan(c);need(p.packet_length==17&&std::memcmp(p.packet,"{\"op\":\"snapshot\"}",17)==0,"canonical snapshot bytes");report(c,p,PR_APPEARANCE_SENT,p.packet_length);ready(c,p.connection_generation,PR_APPEARANCE_READ_READY);return take(c,PR_APPEARANCE_RECEIVE);}
static int32_t receive(Core&c,const PrAppearanceStepV1&p,const std::string&wire){return pr_appearance_client_received_v1(c.p,&p,reinterpret_cast<const uint8_t*>(wire.data()),uint32_t(wire.size()));}
static const std::array<const char*,9> roles{"background","foreground","surface","border","accent","muted","urgent","positive","ink"};
static const std::array<const char*,8> fonts{"caption","bodySmall","body","subtitle","title","heading","display","displayLarge"};
static const std::array<const char*,6> spacing{"controlGap","panelPadding","panelGap","rowGap","controlPaddingX","controlPaddingY"};
static const std::array<int,8> fv{15,16,18,19,21,24,36,42};
static const std::array<int,6> sv{12,27,21,12,15,9};
static std::string wire(uint64_t until=UINT64_MAX,uint32_t revision=1,const std::string&owner="11111111111111111111111111111111",bool published=true,const std::string&alpha="32768",const std::string&base="18"){
 std::string s="{\"schema\":3,\"owner\":\""+owner+"\",\"revision\":"+std::to_string(revision)+",\"until_ns\":\""+std::to_string(until)+"\",\"available\":"+(published?"true":"false")+",\"appearance\":";
 if(!published)return s+"null}";
 s+="{\"palette\":{";
 for(size_t i=0;i<roles.size();i++){if(i)s+=',';s+='\"';s+=roles[i];s+="\":{\"r\":"+std::to_string(1234+i)+",\"g\":"+std::to_string(2345+i)+",\"b\":"+std::to_string(3456+i)+",\"a\":"+(i?std::to_string(65535-i):alpha)+"}";}
 s+="},\"style\":{\"base_size\":"+base+",\"font_tokens\":{";
 for(size_t i=0;i<fonts.size();i++){if(i)s+=',';s+='\"';s+=fonts[i];s+="\":"+std::to_string(fv[i]);}
 s+="},\"spacing_tokens\":{";
 for(size_t i=0;i<spacing.size();i++){if(i)s+=',';s+='\"';s+=spacing[i];s+="\":"+std::to_string(sv[i]);}
 return s+"}}}}";
}
static std::string replaced(std::string text,const std::string&old,const std::string&value){const auto at=text.find(old);need(at!=std::string::npos,"fixture replacement found");text.replace(at,old.size(),value);return text;}
static void accepted(Core&c,const std::string&text){const auto p=receive_plan(c);need(receive(c,p,text)==PR_APPEARANCE_OK,"accepted schema3");}
static void normalized(const PrAppearanceViewV1&v){need(v.has_snapshot&&v.base_size==18&&v.font_scale==1.5,"full style normalization");for(size_t i=0;i<roles.size();i++)need(v.palette[i].r==uint16_t(1234+i)&&v.palette[i].g==uint16_t(2345+i)&&v.palette[i].b==uint16_t(3456+i)&&v.palette[i].a==uint16_t(i?65535-i:32768),"exact all RGBA16");for(size_t i=0;i<fonts.size();i++)need(v.font_tokens[i]==uint32_t(fv[i]),"all eight font tokens");for(size_t i=0;i<spacing.size();i++)need(v.spacing_tokens[i]==uint32_t(sv[i]),"all six spacing tokens");}
static void closed(Core&c){auto p=take(c,PR_APPEARANCE_CLOSE);report(c,p,PR_APPEARANCE_CLOSED);}
static void group_initial(){
 Core c;const auto initial=c.view();need(!initial.valid&&!initial.has_snapshot&&initial.poll_interval_ms==250&&!c.available(),"initial unavailable and poll");
 PrAppearanceStepV1 untouched;std::memset(&untouched,0xa5,sizeof(untouched));const auto old=untouched;need(pr_appearance_client_step_v1(c.p,&untouched)==PR_APPEARANCE_EMPTY&&std::memcmp(&untouched,&old,sizeof(old))==0,"idle no output");
 need(pr_appearance_client_refresh_v1(c.p)==PR_APPEARANCE_OK,"first refresh");auto p=take(c,PR_APPEARANCE_CONNECT);report(c,p,PR_APPEARANCE_CONNECT_PENDING);ready(c,p.connection_generation,PR_APPEARANCE_WRITE_READY);p=take(c,PR_APPEARANCE_FINISH_CONNECT);report(c,p,PR_APPEARANCE_CONNECTED);p=take(c,PR_APPEARANCE_SEND);
 need(p.packet_length==17&&std::memcmp(p.packet,"{\"op\":\"snapshot\"}",17)==0,"exact request bytes");const auto deadline=p.original_until_boottime_ns,request=p.request_token,generation=p.connection_generation;
 need(deadline>now()&&deadline-now()<=500000000ULL,"one original500ms exchange");need(pr_appearance_client_step_v1(c.p,&untouched)==PR_APPEARANCE_WAIT&&std::memcmp(&untouched,&old,sizeof(old))==0,"one outstanding effect");
 report(c,p,PR_APPEARANCE_WOULD_BLOCK);need(pr_appearance_client_report_v1(c.p,&p,PR_APPEARANCE_SENT,17)==PR_APPEARANCE_STALE,"no duplicate old effect");need(pr_appearance_client_step_v1(c.p,&untouched)==PR_APPEARANCE_WAIT&&std::memcmp(&untouched,&old,sizeof(old))==0,"EAGAIN cannot busy-spin");ready(c,generation,PR_APPEARANCE_WRITE_READY);auto next=take(c,PR_APPEARANCE_SEND);need(next.effect_token>p.effect_token&&next.request_token==request&&next.connection_generation==generation&&next.original_until_boottime_ns==deadline,"EAGAIN preserves exchange");report(c,next,PR_APPEARANCE_INTERRUPTED);ready(c,generation,PR_APPEARANCE_WRITE_READY);next=take(c,PR_APPEARANCE_SEND);need(next.original_until_boottime_ns==deadline,"EINTR no renewal");report(c,next,PR_APPEARANCE_SENT,17);ready(c,generation,PR_APPEARANCE_READ_READY);next=take(c,PR_APPEARANCE_RECEIVE);need(receive(c,next,wire())==PR_APPEARANCE_OK,"initial publication");normalized(c.view());need(c.available()&&c.view().expiry_timer_ms==uint32_t(INT_MAX),"fresh lease and bounded expiry timer");
}
static void group_revision(){
 Core c;accepted(c,wire(UINT64_MAX,9));const auto first=c.view();accepted(c,wire(UINT64_MAX-1,9));need(c.view().change_sequence==first.change_sequence,"same model does not signal");auto p=receive_plan(c);need(receive(c,p,wire(UINT64_MAX,9,"11111111111111111111111111111111",true,"32769"))==PR_APPEARANCE_REFUSED,"same revision alpha mutation refuses");need(!c.available()&&c.view().palette[0].a==32768,"refusal retains cached RGBA16");closed(c);accepted(c,wire(UINT64_MAX,0,"22222222222222222222222222222222"));need(c.available()&&c.view().revision==0&&c.view().owner[0]==0x22,"new owner may restart revision");
 Core rollback;accepted(rollback,wire(UINT64_MAX,9));p=receive_plan(rollback);need(receive(rollback,p,wire(UINT64_MAX,8))==PR_APPEARANCE_REFUSED&&!rollback.available()&&rollback.view().revision==9,"same-owner rollback atomic refusal");
}
static void group_style(){
 Core c;accepted(c,wire());accepted(c,wire(UINT64_MAX,2,"11111111111111111111111111111111",true,"32768","24"));const auto updated=c.view();need(updated.base_size==24&&updated.font_scale==2.0,"new revision style accepted");auto p=receive_plan(c);need(receive(c,p,wire(UINT64_MAX,2,"11111111111111111111111111111111",true,"32768","30"))==PR_APPEARANCE_REFUSED&&c.view().base_size==24&&!c.available(),"same revision style remains immutable");
 Core numeric;auto json=wire();json=replaced(json,"\"schema\":3","\"schema\":99,\"schema\":3");json=replaced(json,"\"r\":1234","\"r\":1234.0");json=replaced(json,"\"caption\":15","\"caption\":15e0");accepted(numeric,json);normalized(numeric.view());
 Core boundary;json=wire(UINT64_MAX,1,"11111111111111111111111111111111",true,"0","2147483647");json=replaced(json,"\"caption\":15","\"caption\":2147483647");json=replaced(json,"\"controlGap\":12","\"controlGap\":0");accepted(boundary,json);need(boundary.view().font_tokens[0]==uint32_t(INT_MAX)&&boundary.view().spacing_tokens[0]==0&&boundary.view().palette[0].a==0,"INT32/zero-spacing/alpha boundaries");
}
static void group_shapes(){
 const auto good=wire();std::vector<std::string> bad{
 replaced(good,"\"schema\":3","\"schema\":2"),replaced(good,"\"schema\":3","\"extra\":0,\"schema\":3"),
 replaced(good,"\"base_size\":18","\"base_size\":18,\"extra\":0"),replaced(good,"\"r\":1234","\"r\":1234,\"extra\":0"),
 replaced(good,"\"r\":1234","\"r\":65536"),replaced(good,"\"a\":32768","\"a\":true"),
 replaced(good,"\"caption\":15","\"caption\":0"),replaced(good,"\"caption\":15","\"caption\":2147483648"),
 replaced(good,"\"controlGap\":12","\"controlGap\":-1"),replaced(good,"\"base_size\":18","\"base_size\":18.5"),
 replaced(good,"\"until_ns\":\"18446744073709551615\"","\"until_ns\":\"018446744073709551615\""),
 replaced(good,"\"until_ns\":\"18446744073709551615\"","\"until_ns\":18446744073709551615"),
 replaced(good,"11111111111111111111111111111111","AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
 replaced(good,"11111111111111111111111111111111","11111111111111111111111111111111\\n"),
 replaced(good,"\"available\":true","\"available\":false"),replaced(good,"\"available\":true","\"available\":1")};
 for(const auto&json:bad){Core c;const auto p=receive_plan(c);need(receive(c,p,json)==PR_APPEARANCE_REFUSED&&!c.available()&&!c.view().has_snapshot,"bounded malformed shape refusal");}
 Core absent;auto json=wire(UINT64_MAX,1,"11111111111111111111111111111111",false);json=replaced(json,",\"appearance\":null","");const auto p=receive_plan(absent);need(receive(absent,p,json)==PR_APPEARANCE_REFUSED,"nullable appearance key is required");
 Core oversized;auto plan=receive_plan(oversized);json=good+std::string(4096,' ');need(receive(oversized,plan,json)==PR_APPEARANCE_REFUSED,"max4096 packet");
}
static void group_retention(){
 Core c;auto p=receive_plan(c);auto bytes=wire();need(receive(c,p,bytes)==PR_APPEARANCE_OK,"owned publication");std::memset(bytes.data(),'x',bytes.size());normalized(c.view());const auto published=c.view();
 accepted(c,wire(UINT64_MAX,2,"11111111111111111111111111111111",false));need(!c.available()&&!c.view().valid&&c.view().has_snapshot&&c.view().revision==2,"valid unavailable stores metadata retains snapshot");normalized(c.view());const auto unavailable=c.view();need(unavailable.change_sequence>published.change_sequence,"availability edge signals once");accepted(c,wire(UINT64_MAX-1,2,"11111111111111111111111111111111",false));need(c.view().change_sequence==unavailable.change_sequence,"repeated unavailable no extra changed");
}
static void group_deadlines(){
 Core c;const auto lease=now()+80000000ULL;accepted(c,wire(lease));const auto pending=receive_plan(c);wait_until(lease);need(!c.available()&&c.view().valid,"getter fences lease before timer delivery");need(pr_appearance_client_tick_v1(c.p)==PR_APPEARANCE_OK&&!c.view().valid,"separate cached expiry");need(c.view().original_until_boottime_ns==pending.original_until_boottime_ns&&c.view().stage==PR_APPEARANCE_RECEIVING,"lease does not renew or cancel current RPC");need(receive(c,pending,wire())==PR_APPEARANCE_OK&&c.available(),"fresh current response restores publication");
 const auto sent=send_plan(c);report(c,sent,PR_APPEARANCE_WOULD_BLOCK);wait_until(sent.original_until_boottime_ns);PrAppearanceStepV1 out;std::memset(&out,0xa5,sizeof(out));const auto before=out;need(pr_appearance_client_step_v1(c.p,&out)==PR_APPEARANCE_EXPIRED&&std::memcmp(&out,&before,sizeof(out))==0&&!c.available(),"no expired native send or renewed deadline");closed(c);
 Core late;const auto p=receive_plan(late);wait_until(p.original_until_boottime_ns);need(receive(late,p,wire())==PR_APPEARANCE_EXPIRED&&!late.available()&&!late.view().has_snapshot,"timely native read delayed past original exchange cannot publish");
}
static void group_generations(){
 Core c;accepted(c,wire());auto old=receive_plan(c);report(c,old,PR_APPEARANCE_IO_FAILED);closed(c);const auto current=receive_plan(c);const auto before=c.view();need(current.connection_generation>old.connection_generation,"closed generation never reused");need(pr_appearance_client_ready_v1(c.p,old.connection_generation,PR_APPEARANCE_READ_READY)==PR_APPEARANCE_STALE,"old queued readiness cannot select current work");need(pr_appearance_client_report_v1(c.p,&old,PR_APPEARANCE_IO_FAILED,0)==PR_APPEARANCE_STALE,"old notifier cannot close current channel");need(receive(c,old,wire())==PR_APPEARANCE_STALE&&c.view().request_token==before.request_token&&c.view().stage==before.stage,"old packet cannot reactivate or cancel current request");need(receive(c,current,wire(UINT64_MAX,2))==PR_APPEARANCE_OK&&c.available(),"current generation still publishes");
}
static void group_stop(){
 Core c;need(pr_appearance_client_refresh_v1(c.p)==PR_APPEARANCE_OK,"start before Stop");const auto pending=take(c,PR_APPEARANCE_CONNECT);need(pr_appearance_client_stop_v1(c.p)==PR_APPEARANCE_OK,"Stop");need(pr_appearance_client_report_v1(c.p,&pending,PR_APPEARANCE_CONNECTED,0)==PR_APPEARANCE_REFUSED,"late connect cannot revive stopped policy");closed(c);need(c.view().stage==PR_APPEARANCE_STOPPED&&c.view().poll_interval_ms==0&&!c.available(),"stopped timers and publication");need(pr_appearance_client_refresh_v1(c.p)==PR_APPEARANCE_REFUSED,"no retry after Stop");
 Core clock;accepted(clock,wire());fail_next_boottime=true;need(!clock.available()&&clock.view().valid,"pure getter refuses failed clock without mutation");fail_next_boottime=true;need(pr_appearance_client_tick_v1(clock.p)==PR_APPEARANCE_REFUSED&&!clock.available(),"actual libc EIO fail closes");closed(clock);accepted(clock,wire());need(clock.available(),"existing periodic retry may recover clock failure");
 PrAppearanceStepV1 out{};need(pr_appearance_client_step_v1(c.p,&out)==PR_APPEARANCE_EMPTY,"no new native operation after close");need(pr_appearance_client_new_v1(nullptr)==PR_APPEARANCE_INVALID&&pr_appearance_client_view_v1(c.p,nullptr)==PR_APPEARANCE_INVALID&&pr_appearance_client_available_v1(c.p,nullptr)==PR_APPEARANCE_INVALID,"bounded pointer validation");
}
int main(){
 try{
  const auto start=now();work_until=start+5000000000ULL;close_until=start+8000000000ULL;
  const std::array<std::pair<const char*,void(*)()>,8> groups{{{"initial_query_one_effect_original_deadline",group_initial},{"owner_revision_and_RGBA16_immutability",group_revision},{"style_numeric_projection_and_duplicate_last_wins",group_style},{"schema3_shape_and_bounded_refusals",group_shapes},{"owned_snapshot_and_unavailable_retention",group_retention},{"real_BOOTTIME_RPC_and_independent_cached_expiry",group_deadlines},{"stale_connection_generation_isolation",group_generations},{"Stop_native_close_and_no_reactivation",group_stop}}};
  for(const auto&group:groups){group.second();std::printf("APPEARANCE_CLIENT_PASS %s\n",group.first);}
  const auto end=now();need(end<work_until&&end<close_until,"original final bounds");
  std::printf("APPEARANCE_CLIENT_OBSERVATION pid=%ld original_work_until_ns=%llu original_close_until_ns=%llu completed_boottime_ns=%llu authored_CPU_native_result_reports=true Qt_integration=false\n",long(getpid()),static_cast<unsigned long long>(work_until),static_cast<unsigned long long>(close_until),static_cast<unsigned long long>(end));
  std::puts("APPEARANCE_CLIENT_ALL_PASS groups=8");return 0;
 }catch(const std::exception&error){std::fprintf(stderr,"APPEARANCE_CLIENT_FAILURE %s\n",error.what());return 101;}
}
