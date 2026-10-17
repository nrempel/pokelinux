// Frozen real C++ -> Rust CPU-only observation ABI; no executor, broker or native graph.
#include "pokelinux_render_executor.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <thread>
#include <type_traits>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>
static_assert(sizeof(PrFirstStopFactsV1)==80 && alignof(PrFirstStopFactsV1)==8);
static_assert(sizeof(PrFirstStopSnapshotV1)==112 && alignof(PrFirstStopSnapshotV1)==8);
static_assert(offsetof(PrFirstStopFactsV1,revoke_session)==24);
static_assert(offsetof(PrFirstStopFactsV1,broker_pending_sequence)==56);
static_assert(offsetof(PrFirstStopFactsV1,reserved)==64);
static_assert(offsetof(PrFirstStopSnapshotV1,first)==32);
static_assert(std::is_trivially_copyable_v<PrFirstStopFactsV1>);
static_assert(std::is_trivially_copyable_v<PrFirstStopSnapshotV1>);
static_assert(std::is_same_v<decltype(&pr_first_stop_create_v1),int32_t(*)(PrFirstStop**)>);
static_assert(std::is_same_v<decltype(&pr_first_stop_record_v1),int32_t(*)(PrFirstStop*,const PrFirstStopFactsV1*)>);
static_assert(std::is_same_v<decltype(&pr_first_stop_snapshot_v1),int32_t(*)(PrFirstStop*,PrFirstStopSnapshotV1*)>);
static_assert(std::is_same_v<decltype(&pr_first_stop_release),int32_t(*)(PrFirstStop**)>);
namespace {
void need(bool v,const char* what){if(!v)throw std::runtime_error(what);}
uint64_t now(){timespec t{};need(clock_gettime(CLOCK_BOOTTIME,&t)==0&&t.tv_sec>=0&&t.tv_nsec>=0&&t.tv_nsec<1000000000L,"SETUP CLOCK_BOOTTIME");return uint64_t(t.tv_sec)*1000000000ULL+uint64_t(t.tv_nsec);}
template<class T> auto bytes(const T&v){std::array<unsigned char,sizeof(T)> b{};std::memcpy(b.data(),&v,sizeof(v));return b;}
template<class T> T poison(){T v;std::memset(&v,0xA5,sizeof(v));return v;}
PrFirstStopFactsV1 facts(uint32_t origin){PrFirstStopFactsV1 v{};v.abi_version=PR_FIRST_STOP_ABI_V1;v.struct_size=sizeof(v);v.origin=origin;return v;}
struct Handle{PrFirstStop* value=nullptr;~Handle(){if(value)pr_first_stop_release(&value);}};
PrFirstStopSnapshotV1 snapshot(PrFirstStop* h){auto v=poison<PrFirstStopSnapshotV1>();need(pr_first_stop_snapshot_v1(h,&v)==PR_EXECUTOR_OK,"snapshot");return v;}
void same_first(const PrFirstStopSnapshotV1&a,const PrFirstStopSnapshotV1&b){need(a.has_first==b.has_first&&a.clock_known==b.clock_known&&a.observed_boottime_ns==b.observed_boottime_ns&&a.clock_errno==b.clock_errno&&a.reserved==b.reserved&&bytes(a.first)==bytes(b.first),"immutable first record");}
}
int main(){try{
 const uint64_t until=now()+5000000000ULL;
 Handle h;need(pr_first_stop_create_v1(&h.value)==PR_EXECUTOR_OK&&h.value,"create");
 auto empty=snapshot(h.value);PrFirstStopSnapshotV1 zero{};need(bytes(empty)==bytes(zero),"empty snapshot all bytes zero");
 auto invalid_first=facts(PR_FIRST_STOP_TERMINAL_REOBSERVED);need(pr_first_stop_record_v1(h.value,&invalid_first)==PR_EXECUTOR_INVALID,"terminal reobserved cannot be first");need(bytes(snapshot(h.value))==bytes(empty),"first refusal unchanged");
 auto first=facts(PR_FIRST_STOP_AQ_REVOKE);first.revoke_session=771;first.revoke_reason=2;const auto first_bytes=bytes(first);
 // Authored pre-executor branch facts only. This CPU fixture makes NO authority/native Stop claim.
 const uint64_t before=now();need(pr_first_stop_record_v1(h.value,&first)==PR_EXECUTOR_OK,"preexecutor record");const uint64_t after=now();auto original=snapshot(h.value);
 need(bytes(first)==first_bytes&&original.stop_requests==1&&original.has_first==1&&original.first.executor_present==0&&original.first.broker_present==0&&bytes(original.first)==first_bytes,"preexecutor copied values");
 need((original.clock_known==1&&original.clock_errno==0&&before<=original.observed_boottime_ns&&original.observed_boottime_ns<=after)||(original.clock_known==0&&original.clock_errno>0&&original.observed_boottime_ns==0),"known clock or explicit unavailable");
 std::memset(&first,0xA5,sizeof(first));need(bytes(snapshot(h.value))==bytes(original),"caller facts not retained by pointer");std::puts("FIRST_STOP_PASS preexecutor_fixed_first_and_clock_observation");
 auto second=facts(PR_FIRST_STOP_PUBLIC_STOP);second.executor_present=1;second.broker_present=1;second.broker_transport_failed=1;
 need(pr_first_stop_record_v1(h.value,&second)==PR_EXECUTOR_OK,"second record");auto two=snapshot(h.value);same_first(original,two);need(two.stop_requests==2,"second increments once");
 auto terminal=facts(PR_FIRST_STOP_TERMINAL_REOBSERVED);need(pr_first_stop_record_v1(h.value,&terminal)==PR_EXECUTOR_OK,"terminal reobservation");auto stable=snapshot(h.value);same_first(original,stable);need(stable.stop_requests==3&&bytes(snapshot(h.value))==bytes(stable),"snapshot does not count");std::puts("FIRST_STOP_PASS immutable_first_subsequent_count");
 auto invalid=facts(PR_FIRST_STOP_PUBLIC_STOP);auto refuse=[&](const PrFirstStopFactsV1&v){const auto vbytes=bytes(v);need(pr_first_stop_record_v1(h.value,&v)==PR_EXECUTOR_INVALID,"invalid facts refused");need(bytes(v)==vbytes&&bytes(snapshot(h.value))==bytes(stable),"invalid preserves input/state");};
 invalid.abi_version=0;refuse(invalid);invalid=facts(1);invalid.struct_size-=1;refuse(invalid);invalid=facts(1);invalid.reserved[3]=1;refuse(invalid);invalid=facts(0);refuse(invalid);invalid=facts(21);refuse(invalid);invalid=facts(1);invalid.executor_present=2;refuse(invalid);invalid=facts(1);invalid.error=9;refuse(invalid);invalid=facts(1);invalid.event_mask=1;refuse(invalid);invalid=facts(1);invalid.revoke_session=1;refuse(invalid);invalid=facts(3);invalid.revoke_reason=6;refuse(invalid);invalid=facts(1);invalid.broker_pending_present=1;refuse(invalid);invalid=facts(1);invalid.broker_present=1;invalid.broker_probe_failed=1;invalid.broker_transport_failed=1;refuse(invalid);
 need(pr_first_stop_record_v1(h.value,nullptr)==PR_EXECUTOR_INVALID&&pr_first_stop_snapshot_v1(h.value,nullptr)==PR_EXECUTOR_INVALID,"null borrows refused");auto poisoned=poison<PrFirstStopSnapshotV1>();const auto output_bytes=bytes(poisoned);need(pr_first_stop_snapshot_v1(nullptr,&poisoned)==PR_EXECUTOR_INVALID&&bytes(poisoned)==output_bytes,"null handle output unchanged");PrFirstStop* occupied=h.value;need(pr_first_stop_create_v1(&occupied)==PR_EXECUTOR_INVALID&&occupied==h.value,"occupied output unchanged");need(bytes(snapshot(h.value))==bytes(stable),"all refusals unchanged");std::puts("FIRST_STOP_PASS invalid_input_preserves_output_and_state");
 struct Observation{bool ok=false;long tid=0;std::thread::id cpp;};Observation observations[2];
 for(auto& observation:observations){std::thread foreign([&]{try{observation.tid=syscall(SYS_gettid);observation.cpp=std::this_thread::get_id();auto out=poison<PrFirstStopSnapshotV1>();const auto outbytes=bytes(out);PrFirstStop* slot=h.value;need(pr_first_stop_record_v1(h.value,&second)==PR_EXECUTOR_INVALID,"foreign first Rust call record");need(pr_first_stop_snapshot_v1(h.value,&out)==PR_EXECUTOR_INVALID&&bytes(out)==outbytes,"foreign snapshot poison unchanged");need(pr_first_stop_release(&slot)==PR_EXECUTOR_INVALID&&slot==h.value,"foreign release slot unchanged");Handle own;need(pr_first_stop_create_v1(&own.value)==PR_EXECUTOR_OK&&own.value,"foreign independent handle");need(bytes(snapshot(own.value))==bytes(zero),"independent empty handle");need(pr_first_stop_record_v1(h.value,&second)==PR_EXECUTOR_INVALID,"cached foreign Rust identity refused");need(pr_first_stop_release(&own.value)==PR_EXECUTOR_OK&&!own.value,"own release");observation.ok=true;}catch(...){observation.ok=false;}});foreign.join();need(observation.ok&&observation.tid>0,"actual joined foreign thread");need(bytes(snapshot(h.value))==bytes(stable),"foreign calls preserve state");}
 need(observations[0].tid!=observations[1].tid&&observations[0].tid!=syscall(SYS_gettid)&&observations[1].tid!=syscall(SYS_gettid),"distinct actual kernel threads");std::printf("FIRST_STOP_OBSERVATION main_tid=%ld foreign1_tid=%ld foreign2_tid=%ld two_actual_joins=true cpp_identity_reused=%s\n",syscall(SYS_gettid),observations[0].tid,observations[1].tid,observations[0].cpp==observations[1].cpp?"true":"false");std::puts("FIRST_STOP_PASS wrong_threads_preserve_bytes_and_handle");
 need(pr_first_stop_release(&h.value)==PR_EXECUTOR_OK&&!h.value&&pr_first_stop_release(&h.value)==PR_EXECUTOR_OK,"release nulls and idempotent");need(pr_first_stop_release(nullptr)==PR_EXECUTOR_INVALID,"null slot invalid");need(now()<until,"original5s case bound");std::puts("FIRST_STOP_PASS independent_observer_lifetime_release");std::puts("FIRST_STOP_ALL_PASS groups=5");return 0;
 }catch(const std::exception&e){std::fprintf(stderr,"FIRST_STOP_FAIL %s\n",e.what());return 1;}}
