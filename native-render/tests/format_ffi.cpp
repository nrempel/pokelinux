// Frozen real C++ -> selected Rust format-capability ABI. Authored CPU facts only.
#include "pokelinux_render_format.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <type_traits>
#include <vector>
#include <time.h>
#include <unistd.h>
static_assert(PR_FORMAT_ABI_V1==1);
static_assert(sizeof(void*)==8);
static_assert(sizeof(PrFormatContextV1)==64 && alignof(PrFormatContextV1)==8);
static_assert(sizeof(PrFormatNativeV1)==16 && alignof(PrFormatNativeV1)==4);
static_assert(sizeof(PrFormatPairV1)==16 && alignof(PrFormatPairV1)==8);
static_assert(sizeof(PrFormatPropertyV1)==16 && sizeof(PrFormatCapV1)==24);
static_assert(sizeof(PrFormatPlaneV1)==80 && alignof(PrFormatPlaneV1)==8);
static_assert(sizeof(PrFormatFactsV1)==224 && alignof(PrFormatFactsV1)==8);
static_assert(sizeof(PrFormatSelectionV1)==104 && alignof(PrFormatSelectionV1)==8);
static_assert(offsetof(PrFormatFactsV1,context)==8);
static_assert(offsetof(PrFormatFactsV1,cap)==72);
static_assert(offsetof(PrFormatFactsV1,current_encoder_state)==104);
static_assert(offsetof(PrFormatFactsV1,native_formats)==152);
static_assert(offsetof(PrFormatPlaneV1,type)==48 && offsetof(PrFormatPlaneV1,in_formats)==64);
static_assert(offsetof(PrFormatSelectionV1,context)==16 && offsetof(PrFormatSelectionV1,modifier)==96);
static_assert(std::is_same_v<decltype(&pr_format_select_v1),int32_t(*)(const PrFormatFactsV1*,PrFormatSelectionV1*)>);
static_assert(std::is_trivially_copyable_v<PrFormatFactsV1> && std::is_trivially_copyable_v<PrFormatSelectionV1>);
namespace {
constexpr uint32_t XR24=0x34325258,AR24=0x34325241,PRIMARY=1;
uint64_t work_until=0,close_until=0;
uint64_t now(){timespec t{};if(clock_gettime(CLOCK_BOOTTIME,&t)||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000L)throw std::runtime_error("CLOCK_BOOTTIME");return static_cast<uint64_t>(t.tv_sec)*1000000000ULL+static_cast<uint64_t>(t.tv_nsec);}
void check(bool ok,const char* reason){if(!ok)throw std::runtime_error(reason);if(now()>=work_until)throw std::runtime_error("original5s work bound");}
template<class T> std::array<unsigned char,sizeof(T)> bytes(const T& value){std::array<unsigned char,sizeof(T)> out{};std::memcpy(out.data(),&value,sizeof(value));return out;}
PrFormatSelectionV1 sentinel(){PrFormatSelectionV1 out;std::memset(&out,0xA5,sizeof(out));return out;}
struct Fixture {
 PrFormatFactsV1 facts{};
 std::array<PrFormatNativeV1,3> native{};
 std::array<uint64_t,4> modifiers{};
 std::array<PrFormatPlaneV1,2> planes{};
 std::array<uint32_t,4> legacy{};
 std::array<PrFormatPairV1,4> pairs{};
 std::array<uint32_t,2> crtcs{},encoders{};
 Fixture(){
  facts.abi_version=PR_FORMAT_ABI_V1;facts.struct_size=sizeof(facts);
  facts.context={101,4,131,3,57856,11,7,40,0};
  facts.cap={PR_FORMAT_READ_OK,0,0,0,1};facts.route_state=PR_FORMAT_READ_OK;
  facts.current_encoder_state=PR_FORMAT_PROPERTY_PRESENT;facts.current_encoder_id=38;facts.current_crtc_id=39;
  facts.native_format_count=facts.modifier_count=facts.plane_count=facts.legacy_format_count=facts.explicit_pair_count=facts.encoder_count=1;
  facts.crtc_count=2;facts.native_formats=native.data();facts.modifiers=modifiers.data();facts.planes=planes.data();facts.legacy_formats=legacy.data();facts.explicit_pairs=pairs.data();facts.crtc_ids=crtcs.data();facts.encoder_possible_crtcs=encoders.data();
  native[0]={XR24,0,1,0};modifiers[0]=0;legacy[0]=XR24;pairs[0]={XR24,0,0};crtcs={39,41};encoders[0]=1;
  auto& plane=planes[0];plane.plane_id=35;plane.current_crtc_id=39;plane.possible_crtcs=1;plane.query_state=PR_FORMAT_READ_OK;
  plane.legacy_count=plane.pair_count=1;plane.blob_state=PR_FORMAT_READ_OK;plane.blob_length=64;
  plane.type={PR_FORMAT_PROPERTY_PRESENT,0,PRIMARY};plane.in_formats={PR_FORMAT_PROPERTY_PRESENT,0,99};
 }
 Fixture(const Fixture&)=delete;Fixture& operator=(const Fixture&)=delete;
 std::vector<unsigned char> copied()const{std::vector<unsigned char> out;auto add=[&](const auto& value){const auto b=bytes(value);out.insert(out.end(),b.begin(),b.end());};add(facts);add(native);add(modifiers);add(planes);add(legacy);add(pairs);add(crtcs);add(encoders);return out;}
 void no_blob(){planes[0].blob_state=PR_FORMAT_READ_UNKNOWN;planes[0].blob_errno=0;planes[0].blob_length=0;planes[0].pair_first=planes[0].pair_count=0;}
};
PrFormatSelectionV1 expected(const Fixture& f,uint32_t decision,uint32_t reason,uint32_t source,uint32_t plane=0){PrFormatSelectionV1 v{};v.abi_version=PR_FORMAT_ABI_V1;v.struct_size=sizeof(v);v.decision=decision;v.reason=reason;v.context=f.facts.context;v.refusal_source=source;v.plane_id=plane;if(decision==PR_FORMAT_CORROBORATED_EXPLICIT){v.drm_format=XR24;v.modifier=0;}return v;}
PrFormatSelectionV1 decision(Fixture& f){const auto before=f.copied();auto out=sentinel();check(pr_format_select_v1(&f.facts,&out)==PR_FORMAT_OK,"actual Rust returned owned decision");check(f.copied()==before,"all borrowed caller bytes unchanged");return out;}
void refuses(Fixture& f,uint32_t reason,uint32_t source,uint32_t plane=0){const auto out=decision(f);check(bytes(out)==bytes(expected(f,PR_FORMAT_REFUSED,reason,source,plane)),"exact owned refusal, source and canonical bytes");}
void malformed(Fixture& f,int32_t status){const auto before=f.copied();auto out=sentinel();const auto original=bytes(out);check(pr_format_select_v1(&f.facts,&out)==status,"actual malformed-input refusal");check(bytes(out)==original&&f.copied()==before,"malformed refusal preserves all caller/output bytes");}
void pass(const char* name){check(true,"group cutoff");std::printf("FORMAT_SELECT_PASS %s\n",name);}
void explicit_and_order(){
 Fixture f;auto out=decision(f);const auto want=expected(f,PR_FORMAT_CORROBORATED_EXPLICIT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35);check(bytes(out)==bytes(want),"exact explicit LINEAR pair corroborated only from known blob");
 // A prior initialized result is valid output storage and must be fully overwritten.
 f.facts.cap.value=0;const auto before=f.copied();check(pr_format_select_v1(&f.facts,&out)==PR_FORMAT_OK,"reuse initialized output");check(bytes(out)==bytes(expected(f,PR_FORMAT_REFUSED,PR_FORMAT_REASON_CAP_UNSUPPORTED,PR_FORMAT_SOURCE_CAP))&&f.copied()==before,"refusal overwrites old choice without stale fields");
 f.facts.cap.value=1;check(pr_format_select_v1(&f.facts,&out)==PR_FORMAT_OK&&bytes(out)==bytes(want),"choice overwrites prior refusal canonically");
 const auto owned=bytes(out);std::memset(&f.facts,0,sizeof(f.facts));f.native.fill({});f.modifiers.fill(77);f.planes.fill({});f.legacy.fill(0);f.pairs.fill({});f.crtcs.fill(0);f.encoders.fill(0);check(bytes(out)==owned,"owned result survives source overwrite");
 {Fixture x;x.facts.native_format_count=2;x.native[0].modifier_count=0;x.native[1]={AR24,0,1,0};x.legacy[0]=AR24;x.pairs[0].drm_format=AR24;refuses(x,PR_FORMAT_REASON_EMPTY_FIRST_MODIFIERS,PR_FORMAT_SOURCE_NATIVE);}
 {Fixture x;x.facts.native_format_count=2;x.facts.modifier_count=2;x.native[1]={AR24,1,1,0};x.modifiers[0]=17;x.modifiers[1]=0;x.legacy[0]=AR24;x.pairs[0].drm_format=AR24;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.modifier_count=2;x.native[0].modifier_count=2;x.modifiers[0]=17;x.modifiers[1]=0;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.modifier_count=2;x.native[0].modifier_count=2;x.modifiers[0]=PR_FORMAT_MOD_INVALID;x.modifiers[1]=0;refuses(x,PR_FORMAT_REASON_IMPLICIT_UNQUALIFIED,PR_FORMAT_SOURCE_NATIVE);}
 {Fixture x;x.facts.native_format_count=0;x.facts.native_formats=nullptr;refuses(x,PR_FORMAT_REASON_EMPTY_NATIVE,PR_FORMAT_SOURCE_NATIVE);}
 pass("explicit_exact_pair_owned_result_and_first_tuple_only");
}
void missing_facts(){
 // Authored copy of the recorded cap0/primary35/XR24/no-IN_FORMATS mismatch;
 // the C ABI call below performs no DRM query and supplies no native evidence.
 {Fixture x;x.facts.cap.value=0;x.planes[0].in_formats={PR_FORMAT_PROPERTY_ABSENT,0,0};x.no_blob();refuses(x,PR_FORMAT_REASON_CAP_UNSUPPORTED,PR_FORMAT_SOURCE_CAP);}
 {Fixture x;x.facts.cap={PR_FORMAT_READ_UNKNOWN,0,0,0,0};refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_CAP);}
 {Fixture x;x.facts.cap={PR_FORMAT_READ_ERROR,-1,5,0,0};refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_CAP);}
 {Fixture x;x.facts.cap.value=2;refuses(x,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_CAP);}
 {Fixture x;x.planes[0].type={PR_FORMAT_PROPERTY_UNKNOWN,0,0};refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_TYPE,35);}
 {Fixture x;x.planes[0].type={PR_FORMAT_PROPERTY_ABSENT,0,0};refuses(x,PR_FORMAT_REASON_PROPERTY_ABSENT,PR_FORMAT_SOURCE_TYPE,35);}
 {Fixture x;x.planes[0].type={PR_FORMAT_PROPERTY_ERROR,5,0};refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_TYPE,35);}
 {Fixture x;x.planes[0].in_formats={PR_FORMAT_PROPERTY_UNKNOWN,0,0};x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_IN_FORMATS,35);}
 {Fixture x;x.planes[0].in_formats={PR_FORMAT_PROPERTY_ABSENT,0,0};x.no_blob();refuses(x,PR_FORMAT_REASON_PROPERTY_ABSENT,PR_FORMAT_SOURCE_IN_FORMATS,35);}
 {Fixture x;x.planes[0].in_formats={PR_FORMAT_PROPERTY_ERROR,5,0};x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_IN_FORMATS,35);}
 {Fixture x;x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_BLOB,35);}
 {Fixture x;x.no_blob();x.planes[0].blob_state=PR_FORMAT_READ_ERROR;x.planes[0].blob_errno=5;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_BLOB,35);}
 {Fixture x;x.planes[0].in_formats.value=0;x.no_blob();refuses(x,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_IN_FORMATS,35);}
 pass("recorded_cap0_and_absent_unknown_error_are_distinct");
}
void routes_and_pairs(){
 {Fixture x;x.facts.route_state=PR_FORMAT_READ_UNKNOWN;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.route_state=PR_FORMAT_READ_ERROR;x.facts.route_errno=5;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_UNKNOWN;x.facts.current_crtc_id=0;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_ERROR;x.facts.current_encoder_errno=5;x.facts.current_crtc_id=0;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_ABSENT;x.facts.current_encoder_id=x.facts.current_crtc_id=0;check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_CORROBORATED_EXPLICIT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"known absent encoder is not a failed query; raw possible route remains");}
 {Fixture x;x.encoders[0]=2;refuses(x,PR_FORMAT_REASON_NO_ROUTE,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_crtc_id=999;refuses(x,PR_FORMAT_REASON_NO_ROUTE,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.planes[0].possible_crtcs=2;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.planes[0].type.value=2;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.legacy[0]=AR24;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.pairs[0].modifier=1;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.plane_count=2;x.planes[1]=x.planes[0];x.planes[0].plane_id=34;x.planes[0].type.value=2;check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_CORROBORATED_EXPLICIT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"overlay cannot corroborate but later actual primary can");}
 pass("raw_route_primary_legacy_and_exact_pair_controls");
}
void atomic_invalid(){
 constexpr auto MAX=std::numeric_limits<uint32_t>::max();
 {Fixture x;x.facts.abi_version=2;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;--x.facts.struct_size;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.context.reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.reserved[0]=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.reserved_count=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.native[0].reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.pairs[0].reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.cap.state=9;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.planes[0].type.state=9;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.planes[0].type={PR_FORMAT_PROPERTY_ABSENT,0,1};malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.cap={PR_FORMAT_READ_ERROR,0,5,0,0};malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_ABSENT;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.native[0].modifier_first=MAX;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.native[0].modifier_count=MAX;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.planes[0].legacy_first=MAX;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.planes[0].pair_first=MAX;x.planes[0].pair_count=2;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.native_format_count=PR_FORMAT_MAX_NATIVE_FORMATS+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.modifier_count=MAX;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.plane_count=PR_FORMAT_MAX_PLANES+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.legacy_format_count=PR_FORMAT_MAX_LEGACY_FORMATS+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.explicit_pair_count=PR_FORMAT_MAX_EXPLICIT_PAIRS+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.crtc_count=PR_FORMAT_MAX_CRTCS+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.encoder_count=PR_FORMAT_MAX_ENCODERS+1;malformed(x,PR_FORMAT_CAPACITY);}
 {Fixture x;x.facts.modifiers=nullptr;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.native_format_count=0;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;auto out=sentinel();const auto ob=bytes(out);const auto ib=x.copied();check(pr_format_select_v1(nullptr,&out)==PR_FORMAT_INVALID&&bytes(out)==ob,"null facts preserves initialized result");check(pr_format_select_v1(&x.facts,nullptr)==PR_FORMAT_INVALID&&x.copied()==ib,"null output preserves input");}
 {Fixture x;alignas(PrFormatFactsV1) std::array<unsigned char,sizeof(PrFormatFactsV1)+8> storage{};std::memcpy(storage.data()+1,&x.facts,sizeof(x.facts));const auto original=storage;auto out=sentinel();const auto ob=bytes(out);const auto* bad=reinterpret_cast<const PrFormatFactsV1*>(storage.data()+1);check(pr_format_select_v1(bad,&out)==PR_FORMAT_INVALID&&bytes(out)==ob&&storage==original,"misaligned allocated input refused before typed access");}
 {Fixture x;alignas(PrFormatSelectionV1) std::array<unsigned char,sizeof(PrFormatSelectionV1)+8> storage{};storage.fill(0xA5);const auto original=storage;const auto ib=x.copied();auto* bad=reinterpret_cast<PrFormatSelectionV1*>(storage.data()+1);check(pr_format_select_v1(&x.facts,bad)==PR_FORMAT_INVALID&&storage==original&&x.copied()==ib,"misaligned allocated output refused before write");}
 {Fixture x;alignas(uint64_t) std::array<unsigned char,32> storage{};const auto original=storage;x.facts.modifiers=reinterpret_cast<const uint64_t*>(storage.data()+1);malformed(x,PR_FORMAT_INVALID);check(storage==original,"misaligned allocated slice unchanged");}
 {Fixture x;const auto ib=x.copied();auto* overlap=reinterpret_cast<PrFormatSelectionV1*>(&x.facts);check(pr_format_select_v1(&x.facts,overlap)==PR_FORMAT_INVALID&&x.copied()==ib,"overlap rejected without accessing output");}
 // Validate every span before deciding, including an unused later native group.
 {Fixture x;x.facts.native_format_count=2;x.native[1]={AR24,MAX,2,0};malformed(x,PR_FORMAT_INVALID);}
 // A prior successful owned value also survives malformed-input refusal.
 {Fixture x;auto out=decision(x);const auto ob=bytes(out);x.native[0].modifier_first=MAX;const auto ib=x.copied();check(pr_format_select_v1(&x.facts,&out)==PR_FORMAT_INVALID&&bytes(out)==ob&&x.copied()==ib,"prior owned decision atomically preserved on malformed input");}
 pass("whole_input_bounds_alignment_and_atomic_refusal");
}
}
int main(){try{const auto started=now();work_until=started+5000000000ULL;close_until=started+8000000000ULL;explicit_and_order();missing_facts();routes_and_pairs();atomic_invalid();const auto completed=now();check(completed<work_until&&work_until<close_until,"original work/close cutoffs");std::printf("FORMAT_SELECT_OBSERVATION pid=%ld original_work_until_ns=%llu original_close_until_ns=%llu completed_boottime_ns=%llu authored_CPU_facts=true\n",static_cast<long>(getpid()),static_cast<unsigned long long>(work_until),static_cast<unsigned long long>(close_until),static_cast<unsigned long long>(completed));std::puts("FORMAT_SELECT_ALL_PASS groups=4");return 0;}catch(const std::exception& e){std::fprintf(stderr,"FORMAT_SELECT_FAIL %s\n",e.what());return 1;}}
