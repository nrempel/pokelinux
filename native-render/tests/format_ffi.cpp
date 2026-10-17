// Frozen real C++ -> selected Rust format-plan ABI2. Authored CPU facts only.
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
static_assert(PR_FORMAT_ABI_V2==2);
static_assert(sizeof(void*)==8);
static_assert(sizeof(PrFormatContextV1)==64 && alignof(PrFormatContextV1)==8);
static_assert(sizeof(PrFormatNativeV1)==16 && alignof(PrFormatNativeV1)==4);
static_assert(sizeof(PrFormatPairV1)==16 && alignof(PrFormatPairV1)==8);
static_assert(sizeof(PrFormatPropertyV1)==16 && sizeof(PrFormatCapV1)==24);
static_assert(sizeof(PrFormatPlaneV1)==80 && alignof(PrFormatPlaneV1)==8);
static_assert(sizeof(PrFormatFactsV2)==312 && alignof(PrFormatFactsV2)==8);
static_assert(sizeof(PrFormatSelectionV2)==112 && alignof(PrFormatSelectionV2)==8);
static_assert(offsetof(PrFormatFactsV2,context)==8);
static_assert(offsetof(PrFormatFactsV2,cap)==72);
static_assert(offsetof(PrFormatFactsV2,current_encoder_state)==104);
static_assert(offsetof(PrFormatFactsV2,native_formats)==152);
static_assert(offsetof(PrFormatPlaneV1,type)==48 && offsetof(PrFormatPlaneV1,in_formats)==64);
static_assert(offsetof(PrFormatSelectionV2,context)==16 && offsetof(PrFormatSelectionV2,modifier)==96);
static_assert(std::is_same_v<decltype(&pr_format_select_v2),int32_t(*)(const PrFormatFactsV2*,PrFormatSelectionV2*)>);
static_assert(std::is_trivially_copyable_v<PrFormatFactsV2> && std::is_trivially_copyable_v<PrFormatSelectionV2>);
static_assert(sizeof(PrFormatCallV1)==16 && alignof(PrFormatCallV1)==4);
static_assert(PR_FORMAT_REFUSED==1 && PR_FORMAT_PREPARE_ATTEMPT==2);
static_assert(PR_FORMAT_BUFFER_NONE==0 && PR_FORMAT_BUFFER_IMPLICIT==1 && PR_FORMAT_BUFFER_EXPLICIT==2);
#define OFFSET(T,F,N) static_assert(offsetof(T,F)==N)
OFFSET(PrFormatFactsV2,abi_version,0); OFFSET(PrFormatFactsV2,struct_size,4);
OFFSET(PrFormatFactsV2,route_state,96); OFFSET(PrFormatFactsV2,route_errno,100);
OFFSET(PrFormatFactsV2,current_encoder_errno,108); OFFSET(PrFormatFactsV2,current_encoder_id,112); OFFSET(PrFormatFactsV2,current_crtc_id,116);
OFFSET(PrFormatFactsV2,native_format_count,120); OFFSET(PrFormatFactsV2,modifier_count,124); OFFSET(PrFormatFactsV2,plane_count,128);
OFFSET(PrFormatFactsV2,legacy_format_count,132); OFFSET(PrFormatFactsV2,explicit_pair_count,136); OFFSET(PrFormatFactsV2,crtc_count,140);
OFFSET(PrFormatFactsV2,encoder_count,144); OFFSET(PrFormatFactsV2,reserved_count,148); OFFSET(PrFormatFactsV2,modifiers,160);
OFFSET(PrFormatFactsV2,planes,168); OFFSET(PrFormatFactsV2,legacy_formats,176); OFFSET(PrFormatFactsV2,explicit_pairs,184);
OFFSET(PrFormatFactsV2,crtc_ids,192); OFFSET(PrFormatFactsV2,encoder_possible_crtcs,200); OFFSET(PrFormatFactsV2,reserved,208);
OFFSET(PrFormatFactsV2,allocator_kind,224); OFFSET(PrFormatFactsV2,output_backend_kind,228);
OFFSET(PrFormatFactsV2,width,232); OFFSET(PrFormatFactsV2,height,236); OFFSET(PrFormatFactsV2,scanout,240);
OFFSET(PrFormatFactsV2,cursor,244); OFFSET(PrFormatFactsV2,multigpu,248); OFFSET(PrFormatFactsV2,ordinary_reserved,252);
OFFSET(PrFormatFactsV2,allocator_device_query,256); OFFSET(PrFormatFactsV2,kms_device_query,272);
OFFSET(PrFormatFactsV2,allocator_rdev,288); OFFSET(PrFormatFactsV2,kms_rdev,296);
OFFSET(PrFormatFactsV2,devices_equal,304); OFFSET(PrFormatFactsV2,devices_equal_count,308);
OFFSET(PrFormatSelectionV2,abi_version,0); OFFSET(PrFormatSelectionV2,struct_size,4);
OFFSET(PrFormatSelectionV2,decision,8); OFFSET(PrFormatSelectionV2,reason,12);
OFFSET(PrFormatSelectionV2,plane_id,80); OFFSET(PrFormatSelectionV2,drm_format,84); OFFSET(PrFormatSelectionV2,refusal_source,88);
OFFSET(PrFormatSelectionV2,buffer_mode,92); OFFSET(PrFormatSelectionV2,width,104); OFFSET(PrFormatSelectionV2,height,108);
OFFSET(PrFormatCallV1,state,0); OFFSET(PrFormatCallV1,return_value,4); OFFSET(PrFormatCallV1,error_number,8); OFFSET(PrFormatCallV1,reserved,12);
#undef OFFSET
namespace {
constexpr uint32_t XR24=0x34325258,AR24=0x34325241,PRIMARY=1;
uint64_t work_until=0,close_until=0;
uint64_t now(){timespec t{};if(clock_gettime(CLOCK_BOOTTIME,&t)||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000L)throw std::runtime_error("CLOCK_BOOTTIME");return static_cast<uint64_t>(t.tv_sec)*1000000000ULL+static_cast<uint64_t>(t.tv_nsec);}
void check(bool ok,const char* reason){if(!ok)throw std::runtime_error(reason);if(now()>=work_until)throw std::runtime_error("original5s work bound");}
template<class T> std::array<unsigned char,sizeof(T)> bytes(const T& value){std::array<unsigned char,sizeof(T)> out{};std::memcpy(out.data(),&value,sizeof(value));return out;}
PrFormatSelectionV2 sentinel(){PrFormatSelectionV2 out;std::memset(&out,0xA5,sizeof(out));return out;}
struct Fixture {
 PrFormatFactsV2 facts{};
 std::array<PrFormatNativeV1,3> native{};
 std::array<uint64_t,4> modifiers{};
 std::array<PrFormatPlaneV1,2> planes{};
 std::array<uint32_t,4> legacy{};
 std::array<PrFormatPairV1,4> pairs{};
 std::array<uint32_t,2> crtcs{},encoders{};
 Fixture(){
  facts.abi_version=PR_FORMAT_ABI_V2;facts.struct_size=sizeof(facts);
  facts.context={101,4,131,3,57856,11,7,40,0};
  facts.allocator_kind=PR_FORMAT_ALLOCATOR_GBM;facts.output_backend_kind=PR_FORMAT_BACKEND_DRM;
  facts.width=1080;facts.height=2424;facts.scanout=1;
  facts.allocator_device_query=facts.kms_device_query={PR_FORMAT_READ_OK,0,0,0};
  facts.allocator_rdev=57857;facts.kms_rdev=57856;facts.devices_equal=1;facts.devices_equal_count=1;
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
PrFormatSelectionV2 expected(const Fixture& f,uint32_t decision,uint32_t reason,uint32_t source,uint32_t plane=0){PrFormatSelectionV2 v{};v.abi_version=PR_FORMAT_ABI_V2;v.struct_size=sizeof(v);v.decision=decision;v.reason=reason;v.context=f.facts.context;v.refusal_source=source;v.plane_id=plane;if(decision==PR_FORMAT_PREPARE_ATTEMPT){v.drm_format=f.native[0].drm_format;v.buffer_mode=f.facts.cap.value==0?PR_FORMAT_BUFFER_IMPLICIT:PR_FORMAT_BUFFER_EXPLICIT;v.modifier=v.buffer_mode==PR_FORMAT_BUFFER_IMPLICIT?0:f.modifiers[f.native[0].modifier_first];v.width=f.facts.width;v.height=f.facts.height;}return v;}
PrFormatSelectionV2 decision(Fixture& f){const auto before=f.copied();auto out=sentinel();check(pr_format_select_v2(&f.facts,&out)==PR_FORMAT_OK,"actual Rust returned owned decision");check(f.copied()==before,"all borrowed caller bytes unchanged");return out;}
void refuses(Fixture& f,uint32_t reason,uint32_t source,uint32_t plane=0){const auto out=decision(f);check(bytes(out)==bytes(expected(f,PR_FORMAT_REFUSED,reason,source,plane)),"exact owned refusal, source and canonical bytes");}
void malformed(Fixture& f,int32_t status){const auto before=f.copied();auto out=sentinel();const auto original=bytes(out);check(pr_format_select_v2(&f.facts,&out)==status,"actual malformed-input refusal");check(bytes(out)==original&&f.copied()==before,"malformed refusal preserves all caller/output bytes");}
void pass(const char* name){check(true,"group cutoff");std::printf("FORMAT_SELECT_PASS %s\n",name);}
void explicit_and_order(){
 Fixture f;auto out=decision(f);const auto want=expected(f,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35);check(bytes(out)==bytes(want),"exact explicit LINEAR pair corroborated only from known blob");
 // A prior initialized result is valid output storage and must be fully overwritten.
 f.facts.cap.value=2;const auto before=f.copied();check(pr_format_select_v2(&f.facts,&out)==PR_FORMAT_OK,"reuse initialized output");check(bytes(out)==bytes(expected(f,PR_FORMAT_REFUSED,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_CAP))&&f.copied()==before,"refusal overwrites old choice without stale fields");
 f.facts.cap.value=1;check(pr_format_select_v2(&f.facts,&out)==PR_FORMAT_OK&&bytes(out)==bytes(want),"choice overwrites prior refusal canonically");
 const auto owned=bytes(out);std::memset(&f.facts,0,sizeof(f.facts));f.native.fill({});f.modifiers.fill(77);f.planes.fill({});f.legacy.fill(0);f.pairs.fill({});f.crtcs.fill(0);f.encoders.fill(0);check(bytes(out)==owned,"owned result survives source overwrite");
 {Fixture x;x.facts.native_format_count=2;x.native[0].modifier_count=0;x.native[1]={AR24,0,1,0};x.legacy[0]=AR24;x.pairs[0].drm_format=AR24;refuses(x,PR_FORMAT_REASON_EMPTY_FIRST_MODIFIERS,PR_FORMAT_SOURCE_NATIVE);}
 {Fixture x;x.facts.native_format_count=2;x.facts.modifier_count=2;x.native[1]={AR24,1,1,0};x.modifiers[0]=17;x.modifiers[1]=0;x.legacy[0]=AR24;x.pairs[0].drm_format=AR24;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.modifier_count=2;x.native[0].modifier_count=2;x.modifiers[0]=17;x.modifiers[1]=0;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.modifier_count=2;x.native[0].modifier_count=2;x.modifiers[0]=PR_FORMAT_MOD_INVALID;x.modifiers[1]=0;refuses(x,PR_FORMAT_REASON_EXPLICIT_MODIFIER_INVALID,PR_FORMAT_SOURCE_NATIVE);}
 {Fixture x;x.facts.native_format_count=0;x.facts.native_formats=nullptr;refuses(x,PR_FORMAT_REASON_EMPTY_NATIVE,PR_FORMAT_SOURCE_NATIVE);}
 pass("explicit_exact_pair_owned_result_and_first_tuple_only");
}
void missing_facts(){
 // Authored copy of the recorded cap0/primary35/XR24/no-IN_FORMATS mismatch;
 // the C ABI call below performs no DRM query and supplies no native evidence.
 {Fixture x;x.facts.cap.value=0;x.planes[0].in_formats={PR_FORMAT_PROPERTY_ABSENT,0,0};x.no_blob();check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"recorded cap0/absent property requests implicit, no LINEAR inference");}
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
 for(uint32_t state:{uint32_t{PR_FORMAT_READ_UNKNOWN},uint32_t{PR_FORMAT_READ_ERROR}}){Fixture x;x.planes[0]={};x.planes[0].plane_id=35;x.planes[0].query_state=state;x.planes[0].query_errno=state==PR_FORMAT_READ_ERROR?5:0;
  refuses(x,state==PR_FORMAT_READ_ERROR?PR_FORMAT_REASON_QUERY_ERROR:PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_PLANE,35);}
 pass("recorded_cap0_and_absent_unknown_error_are_distinct");
}
void routes_and_pairs(){
 {Fixture x;x.facts.route_state=PR_FORMAT_READ_UNKNOWN;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.route_state=PR_FORMAT_READ_ERROR;x.facts.route_errno=5;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_UNKNOWN;x.facts.current_crtc_id=0;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_ERROR;x.facts.current_encoder_errno=5;x.facts.current_crtc_id=0;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_encoder_state=PR_FORMAT_PROPERTY_ABSENT;x.facts.current_encoder_id=x.facts.current_crtc_id=0;check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"known absent encoder is not a failed query; raw possible route remains");}
 {Fixture x;x.encoders[0]=2;refuses(x,PR_FORMAT_REASON_NO_ROUTE,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.facts.current_crtc_id=999;refuses(x,PR_FORMAT_REASON_NO_ROUTE,PR_FORMAT_SOURCE_ROUTE);}
 {Fixture x;x.planes[0].possible_crtcs=2;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.planes[0].type.value=2;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.legacy[0]=AR24;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.pairs[0].modifier=1;refuses(x,PR_FORMAT_REASON_NO_EXPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 {Fixture x;x.facts.plane_count=2;x.planes[1]=x.planes[0];x.planes[0].plane_id=34;x.planes[0].type.value=2;check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"overlay cannot corroborate but later actual primary can");}
 pass("raw_route_primary_legacy_and_exact_pair_controls");
}
void atomic_invalid(){
 constexpr auto MAX=std::numeric_limits<uint32_t>::max();
 {Fixture x;x.facts.abi_version=1;malformed(x,PR_FORMAT_INVALID);}
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
 {Fixture x;auto out=sentinel();const auto ob=bytes(out);const auto ib=x.copied();check(pr_format_select_v2(nullptr,&out)==PR_FORMAT_INVALID&&bytes(out)==ob,"null facts preserves initialized result");check(pr_format_select_v2(&x.facts,nullptr)==PR_FORMAT_INVALID&&x.copied()==ib,"null output preserves input");}
 {Fixture x;alignas(PrFormatFactsV2) std::array<unsigned char,sizeof(PrFormatFactsV2)+8> storage{};std::memcpy(storage.data()+1,&x.facts,sizeof(x.facts));const auto original=storage;auto out=sentinel();const auto ob=bytes(out);const auto* bad=reinterpret_cast<const PrFormatFactsV2*>(storage.data()+1);check(pr_format_select_v2(bad,&out)==PR_FORMAT_INVALID&&bytes(out)==ob&&storage==original,"misaligned allocated input refused before typed access");}
 {Fixture x;alignas(PrFormatSelectionV2) std::array<unsigned char,sizeof(PrFormatSelectionV2)+8> storage{};storage.fill(0xA5);const auto original=storage;const auto ib=x.copied();auto* bad=reinterpret_cast<PrFormatSelectionV2*>(storage.data()+1);check(pr_format_select_v2(&x.facts,bad)==PR_FORMAT_INVALID&&storage==original&&x.copied()==ib,"misaligned allocated output refused before write");}
 {Fixture x;alignas(uint64_t) std::array<unsigned char,32> storage{};const auto original=storage;x.facts.modifiers=reinterpret_cast<const uint64_t*>(storage.data()+1);malformed(x,PR_FORMAT_INVALID);check(storage==original,"misaligned allocated slice unchanged");}
 {Fixture x;const auto ib=x.copied();auto* overlap=reinterpret_cast<PrFormatSelectionV2*>(&x.facts);check(pr_format_select_v2(&x.facts,overlap)==PR_FORMAT_INVALID&&x.copied()==ib,"overlap rejected without accessing output");}
 // Validate every span before deciding, including an unused later native group.
 {Fixture x;x.facts.native_format_count=2;x.native[1]={AR24,MAX,2,0};malformed(x,PR_FORMAT_INVALID);}
 // A prior successful owned value also survives malformed-input refusal.
 {Fixture x;auto out=decision(x);const auto ob=bytes(out);x.native[0].modifier_first=MAX;const auto ib=x.copied();check(pr_format_select_v2(&x.facts,&out)==PR_FORMAT_INVALID&&bytes(out)==ob&&x.copied()==ib,"prior owned decision atomically preserved on malformed input");}

 {Fixture x;x.facts.ordinary_reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.allocator_device_query.reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.kms_device_query.reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.devices_equal_count=2;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.devices_equal_count=0;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.allocator_device_query.state=PR_FORMAT_READ_UNKNOWN;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.allocator_device_query={PR_FORMAT_READ_OK,1,0,0};malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.kms_device_query={PR_FORMAT_READ_ERROR,0,5,0};x.facts.devices_equal=x.facts.devices_equal_count=0;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.kms_device_query={PR_FORMAT_READ_UNKNOWN,0,5,0};x.facts.devices_equal=x.facts.devices_equal_count=0;malformed(x,PR_FORMAT_INVALID);}
 for(unsigned field=0;field<3;++field){Fixture x;if(field==0)x.facts.scanout=2;if(field==1)x.facts.cursor=2;if(field==2)x.facts.multigpu=2;malformed(x,PR_FORMAT_INVALID);}
 for(unsigned field=0;field<4;++field){Fixture x;if(field==0)x.facts.width=0;if(field==1)x.facts.height=0;if(field==2)x.facts.width=16385;if(field==3)x.facts.height=16385;malformed(x,PR_FORMAT_INVALID);}
 // Implicit may ignore modifier choices, but never ignores malformed raw input.
 {Fixture x;x.facts.cap.value=0;x.facts.native_format_count=2;x.native[1]={AR24,MAX,2,0};malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.cap.value=0;x.pairs[0].reserved=1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.cap.value=0;x.planes[0].pair_first=MAX;x.planes[0].pair_count=2;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.allocator_device_query.state=9;x.facts.devices_equal=x.facts.devices_equal_count=0;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.allocator_device_query.error_number=-1;malformed(x,PR_FORMAT_INVALID);}
 {Fixture x;x.facts.kms_device_query={PR_FORMAT_READ_ERROR,-1,-1,0};x.facts.devices_equal=x.facts.devices_equal_count=0;malformed(x,PR_FORMAT_INVALID);}
 pass("whole_input_bounds_alignment_and_atomic_refusal");
}
void implicit_ordinary(){
 for(uint64_t raw:{uint64_t{0},uint64_t{17},uint64_t{PR_FORMAT_MOD_INVALID},std::numeric_limits<uint64_t>::max()}){
  Fixture x;x.facts.cap.value=0;x.modifiers[0]=raw;const auto out=decision(x);
  check(bytes(out)==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"implicit ignores raw modifier value without rewriting input");
  check(out.buffer_mode==PR_FORMAT_BUFFER_IMPLICIT&&out.modifier==0,"implicit mode carries request, modifier0 is reserved not forced LINEAR");
 }
 {Fixture x;x.facts.cap.value=0;x.native[0].modifier_count=0;x.facts.modifier_count=0;x.facts.modifiers=nullptr;
  x.planes[0].in_formats={PR_FORMAT_PROPERTY_ABSENT,0,0};x.no_blob();
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"empty first modifier list and known absent property permit exact implicit request");}
 {Fixture x;x.facts.cap.value=0;x.pairs[0].modifier=17;x.modifiers[0]=99;
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"present complete blob does not require explicit pair for implicit");}
 {Fixture x;x.facts.cap.value=0;x.facts.explicit_pair_count=0;x.facts.explicit_pairs=nullptr;x.planes[0].pair_count=0;
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"present completely decoded empty pair set is not implicit rejection");}
 {Fixture x;x.facts.cap.value=0;x.facts.native_format_count=2;x.native[0].modifier_count=0;x.native[1]={AR24,0,1,0};x.legacy[0]=AR24;
  refuses(x,PR_FORMAT_REASON_NO_IMPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
 for(unsigned which=0;which<5;++which){Fixture x;x.facts.cap.value=0;
  if(which==0){x.planes[0].in_formats={PR_FORMAT_PROPERTY_UNKNOWN,0,0};x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_IN_FORMATS,35);}
  if(which==1){x.planes[0].in_formats={PR_FORMAT_PROPERTY_ERROR,5,0};x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_IN_FORMATS,35);}
  if(which==2){x.no_blob();refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_BLOB,35);}
  if(which==3){x.no_blob();x.planes[0].blob_state=PR_FORMAT_READ_ERROR;refuses(x,PR_FORMAT_REASON_QUERY_ERROR,PR_FORMAT_SOURCE_BLOB,35);}
  if(which==4){x.planes[0].in_formats.value=0;x.no_blob();refuses(x,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_IN_FORMATS,35);}
 }
 for(unsigned which=0;which<4;++which){Fixture x;x.facts.cap.value=0;
  if(which==0){x.planes[0].possible_crtcs=2;refuses(x,PR_FORMAT_REASON_NO_IMPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
  if(which==1){x.planes[0].type.value=2;refuses(x,PR_FORMAT_REASON_NO_IMPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
  if(which==2){x.legacy[0]=AR24;refuses(x,PR_FORMAT_REASON_NO_IMPLICIT_MATCH,PR_FORMAT_SOURCE_PLANE);}
  if(which==3){x.planes[0].type={PR_FORMAT_PROPERTY_UNKNOWN,0,0};refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_TYPE,35);}
 }
 for(uint32_t dimension:{uint32_t{1},uint32_t{16384}}){Fixture x;x.facts.cap.value=0;x.facts.width=x.facts.height=dimension;
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"dimension endpoints copied exactly");}
 for(uint32_t state:{uint32_t{PR_FORMAT_READ_UNKNOWN},uint32_t{PR_FORMAT_READ_ERROR}}){
  {Fixture x;x.facts.cap.value=0;x.facts.route_state=state;x.facts.route_errno=state==PR_FORMAT_READ_ERROR?5:0;
   refuses(x,state==PR_FORMAT_READ_ERROR?PR_FORMAT_REASON_QUERY_ERROR:PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_ROUTE);}
  {Fixture x;x.facts.cap.value=0;x.planes[0]={};x.planes[0].plane_id=35;x.planes[0].query_state=state;x.planes[0].query_errno=state==PR_FORMAT_READ_ERROR?5:0;
   refuses(x,state==PR_FORMAT_READ_ERROR?PR_FORMAT_REASON_QUERY_ERROR:PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_PLANE,35);}
 }
 pass("implicit_ordinary_plan_raw_modifier_and_property_controls");
}
void backend_device_options(){
 for(unsigned field=0;field<2;++field){Fixture x;if(field==0)x.facts.allocator_kind=0;else x.facts.output_backend_kind=0;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_BACKEND);}
 for(uint32_t kind:{uint32_t{2},std::numeric_limits<uint32_t>::max()}){
  {Fixture x;x.facts.allocator_kind=kind;refuses(x,PR_FORMAT_REASON_BACKEND_UNSUPPORTED,PR_FORMAT_SOURCE_BACKEND);}
  {Fixture x;x.facts.output_backend_kind=kind;refuses(x,PR_FORMAT_REASON_BACKEND_UNSUPPORTED,PR_FORMAT_SOURCE_BACKEND);}
 }
 for(unsigned field=0;field<2;++field){
  {Fixture x;auto& query=field==0?x.facts.allocator_device_query:x.facts.kms_device_query;query={PR_FORMAT_READ_UNKNOWN,0,0,0};x.facts.devices_equal=x.facts.devices_equal_count=0;
   refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,field==0?PR_FORMAT_SOURCE_ALLOCATOR_DEVICE:PR_FORMAT_SOURCE_KMS_DEVICE);}
  {Fixture x;auto& query=field==0?x.facts.allocator_device_query:x.facts.kms_device_query;query={PR_FORMAT_READ_ERROR,-1,5,0};x.facts.devices_equal=x.facts.devices_equal_count=0;
   refuses(x,PR_FORMAT_REASON_QUERY_ERROR,field==0?PR_FORMAT_SOURCE_ALLOCATOR_DEVICE:PR_FORMAT_SOURCE_KMS_DEVICE);}
 }
 {Fixture x;x.facts.devices_equal=x.facts.devices_equal_count=0;refuses(x,PR_FORMAT_REASON_QUERY_UNKNOWN,PR_FORMAT_SOURCE_DEVICE);}
 {Fixture x;x.facts.devices_equal=0;refuses(x,PR_FORMAT_REASON_DEVICE_MISMATCH,PR_FORMAT_SOURCE_DEVICE);}
 for(int32_t raw:{int32_t{-1},int32_t{2},std::numeric_limits<int32_t>::max()}){Fixture x;x.facts.devices_equal=raw;refuses(x,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_DEVICE);}
 {Fixture x;++x.facts.kms_rdev;refuses(x,PR_FORMAT_REASON_INCONSISTENT,PR_FORMAT_SOURCE_CONTEXT);}
 for(unsigned which=0;which<3;++which){Fixture x;if(which==0)x.facts.scanout=0;if(which==1)x.facts.cursor=1;if(which==2)x.facts.multigpu=1;refuses(x,PR_FORMAT_REASON_OPTIONS_UNSUPPORTED,PR_FORMAT_SOURCE_OPTIONS);}
 for(unsigned which=0;which<3;++which){Fixture x;if(which==0)x.facts.allocator_rdev=0;if(which==1)x.facts.kms_rdev=x.facts.context.rdev=0;if(which==2)x.facts.allocator_rdev=x.facts.kms_rdev=x.facts.context.rdev=0;
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"raw rdev zero is not inferred missing device after actual successful comparison");}
 {Fixture x;x.facts.context={901,44,931,33,57856,111,77,140,0};
  check(bytes(decision(x))==bytes(expected(x,PR_FORMAT_PREPARE_ATTEMPT,PR_FORMAT_REASON_NONE,PR_FORMAT_SOURCE_NONE,35)),"context copied not authenticated or rewritten by stateless policy");}
 pass("backend_raw_device_comparison_context_and_option_refusals");
}

}
int main(){try{const auto started=now();work_until=started+5000000000ULL;close_until=started+8000000000ULL;explicit_and_order();missing_facts();routes_and_pairs();atomic_invalid();implicit_ordinary();backend_device_options();const auto completed=now();check(completed<work_until&&work_until<close_until,"original work/close cutoffs");std::printf("FORMAT_SELECT_OBSERVATION pid=%ld original_work_until_ns=%llu original_close_until_ns=%llu completed_boottime_ns=%llu authored_CPU_facts=true\n",static_cast<long>(getpid()),static_cast<unsigned long long>(work_until),static_cast<unsigned long long>(close_until),static_cast<unsigned long long>(completed));std::puts("FORMAT_SELECT_ALL_PASS groups=6");return 0;}catch(const std::exception& e){std::fprintf(stderr,"FORMAT_SELECT_FAIL %s\n",e.what());return 1;}}
