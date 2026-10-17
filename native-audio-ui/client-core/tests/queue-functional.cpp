#include "pokelinux_sounds_intent.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <time.h>
#include <unistd.h>
#include <cstdint>
#include <limits>
#include <type_traits>
static uint64_t now() {
 timespec ts{}; assert(clock_gettime(CLOCK_BOOTTIME,&ts)==0 && ts.tv_sec>=0);
 return uint64_t(ts.tv_sec)*1000000000ULL+uint64_t(ts.tv_nsec);
}
static PrSoundsRouteV1 id(const char *s) {
 PrSoundsRouteV1 v{}; const auto n=std::strlen(s);assert(n>0&&n<=64);
 v.length=uint32_t(n);std::memcpy(v.bytes,s,n);return v;
}
static bool same(const PrSoundsRouteV1 &a,const PrSoundsRouteV1 &b) {
 return a.length<=64&&b.length<=64&&a.length==b.length
  &&std::memcmp(a.bytes,b.bytes,a.length)==0;
}
struct Queue {
 PrSoundsQueue *p=nullptr;
 Queue(){assert(pr_sounds_queue_new_v1(&p)==PR_SOUNDS_OK&&p);}
 ~Queue(){pr_sounds_queue_free_v1(p);}
 Queue(const Queue&)=delete;Queue&operator=(const Queue&)=delete;
};
static PrSoundsFreshV1 fresh() {
 PrSoundsFreshV1 f{};f.transport_online=f.status_complete=f.available=f.can_choose=1;
 f.route_count=2;f.routes[0]=id("media-a7");f.routes[1]=id("media-z2");return f;
}
static int prepare(Queue &q,const PrSoundsFreshV1 &f,PrSoundsCommandV1 &out) {
 return pr_sounds_queue_prepare_v1(q.p,&f,&out);
}
static void pass(const char *s){std::printf("SOUNDS_QUEUE_PASS %s\n",s);}
int main() {
 static_assert(std::is_trivially_copyable_v<PrSoundsCommandV1>);
 const uint64_t began=now(),work=began+5000000000ULL,close=began+8000000000ULL;
 {
  Queue q;auto route=id("media-a7");const auto entered=now();
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
  const auto returned=now();route=id("media-z2");
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_RECONNECT,nullptr)==PR_SOUNDS_BUSY);
  assert(pr_sounds_queue_pending_v1(q.p)==1);auto f=fresh();f.status_complete=0;
  PrSoundsCommandV1 out{};out.token=0x55;
  assert(prepare(q,f,out)==PR_SOUNDS_BUSY&&out.token==0x55);
  f.status_complete=1;assert(prepare(q,f,out)==PR_SOUNDS_OK);
  assert(out.kind==PR_SOUNDS_ROUTE&&same(out.route,id("media-a7"))&&out.token>0);
  assert(out.original_until_boottime_ns>=entered+1000000000ULL);
  assert(out.original_until_boottime_ns<=returned+1000000000ULL);
  assert(out.ttl_ms>0&&out.ttl_ms<=1000);assert(pr_sounds_queue_pending_v1(q.p)==1);
 }
 pass("copied_first_intent_waits_for_final_status");
 {
  Queue q;const auto route=id("media-a7");auto f=fresh();PrSoundsCommandV1 a{},b{};
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
  assert(prepare(q,f,a)==PR_SOUNDS_OK);
  const auto tick=now()+2000000ULL;while(now()<tick){assert(now()<work);}
  assert(prepare(q,f,b)==PR_SOUNDS_OK);
  assert(a.token==b.token&&a.original_until_boottime_ns==b.original_until_boottime_ns);
  assert(same(a.route,b.route)&&b.ttl_ms<a.ttl_ms&&b.ttl_ms>0);
  assert(pr_sounds_queue_pending_v1(q.p)==1); // no actual send was reported
 }
 pass("repeated_prepare_keeps_original_deadline");
 {
  const auto route=id("media-a7");
  for(int fault=0;fault<4;++fault){Queue q;auto f=fresh();PrSoundsCommandV1 out{};
   assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
   if(fault==0)f.can_choose=0;
   if(fault==1)f.available=0;
   if(fault==2)f.transport_online=0;
   if(fault==3){f.route_count=1;f.routes[0]=id("media-z2");}
   out.token=0x66;assert(prepare(q,f,out)==PR_SOUNDS_REFUSED&&out.token==0x66);
   assert(pr_sounds_queue_pending_v1(q.p)==0);
  }
  for(int fault=0;fault<4;++fault){Queue q;auto f=fresh();PrSoundsCommandV1 out{};
   assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
   if(fault==0)f.can_choose=2;
   if(fault==1)f.route_count=33;
   if(fault==2)f.routes[1]=f.routes[0];
   if(fault==3)f.routes[0].length=65;
   out.token=0x67;assert(prepare(q,f,out)==PR_SOUNDS_INVALID&&out.token==0x67);
   assert(pr_sounds_queue_pending_v1(q.p)==0);
  }
 }
 pass("fresh_permission_availability_membership_cancels");
 {
  Queue q;auto f=fresh();f.available=f.can_choose=0;f.can_reconnect=1;
  PrSoundsCommandV1 out{};
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_RECONNECT,nullptr)==PR_SOUNDS_OK);
  assert(prepare(q,f,out)==PR_SOUNDS_OK&&out.kind==PR_SOUNDS_RECONNECT);
  assert(out.route.length==0&&out.ttl_ms==0&&out.original_until_boottime_ns>now());
  assert(pr_sounds_queue_sent_v1(q.p,out.token)==PR_SOUNDS_OK);
  assert(pr_sounds_queue_pending_v1(q.p)==0);
  assert(pr_sounds_queue_sent_v1(q.p,out.token)==PR_SOUNDS_STALE);
  assert(prepare(q,f,out)==PR_SOUNDS_EMPTY);
 }
 pass("one_explicit_reconnect_never_replays");
 {
  Queue routeq,reconnectq;const auto route=id("media-a7");auto f=fresh();f.can_reconnect=1;
  PrSoundsCommandV1 a{},b{};
  assert(pr_sounds_queue_offer_v1(routeq.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
  assert(pr_sounds_queue_offer_v1(reconnectq.p,PR_SOUNDS_RECONNECT,nullptr)==PR_SOUNDS_OK);
  assert(prepare(routeq,f,a)==PR_SOUNDS_OK&&prepare(reconnectq,f,b)==PR_SOUNDS_OK);
  const auto until=a.original_until_boottime_ns>b.original_until_boottime_ns?
   a.original_until_boottime_ns:b.original_until_boottime_ns;
  while(now()<until){assert(now()<work);timespec pause{0,1000000};nanosleep(&pause,nullptr);}
  // Root-requested preimplementation order: expiry precedes incomplete status.
  f.status_complete=0;
  a.token=b.token=0x77;
  assert(prepare(routeq,f,a)==PR_SOUNDS_EXPIRED&&a.token==0x77);
  assert(prepare(reconnectq,f,b)==PR_SOUNDS_EXPIRED&&b.token==0x77);
  assert(pr_sounds_queue_pending_v1(routeq.p)==0&&pr_sounds_queue_pending_v1(reconnectq.p)==0);
 }
 pass("actual_boottime_expiry_without_timer_delivery");
 {
  Queue q;const auto route=id("media-a7");auto f=fresh();PrSoundsCommandV1 a{},b{};
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
  assert(prepare(q,f,a)==PR_SOUNDS_OK);pr_sounds_queue_cancel_v1(q.p);
  assert(pr_sounds_queue_sent_v1(q.p,a.token)==PR_SOUNDS_STALE);
  assert(pr_sounds_queue_offer_v1(q.p,PR_SOUNDS_ROUTE,&route)==PR_SOUNDS_OK);
  assert(prepare(q,f,b)==PR_SOUNDS_OK&&b.token!=a.token);
  assert(pr_sounds_queue_sent_v1(q.p,a.token)==PR_SOUNDS_STALE);
  assert(pr_sounds_queue_pending_v1(q.p)==1);
  assert(pr_sounds_queue_sent_v1(q.p,b.token)==PR_SOUNDS_OK);
  assert(pr_sounds_queue_pending_v1(q.p)==0&&prepare(q,f,b)==PR_SOUNDS_EMPTY);
 }
 pass("cancel_and_one_shot_send_token_fences");
 const auto completed=now();assert(completed<work&&completed<close);
 std::printf("SOUNDS_QUEUE_ALL_PASS groups=6 pid=%ld original_work_until_ns=%llu original_close_until_ns=%llu completed_boottime_ns=%llu\n",
  long(getpid()),(unsigned long long)work,(unsigned long long)close,(unsigned long long)completed);
}
