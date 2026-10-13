// Synthetic native peer using genuine, retained ARM64 Android libbinder_ndk/Bionic.
// No production factory service or vendor implementation is executed.
#include <android/binder_ibinder.h>
#include <android/binder_parcel.h>
#include <android/binder_status.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <atomic>
#include <initializer_list>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dlfcn.h>
#include <unistd.h>
static const char* RADIO="vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";
static std::atomic<int32_t> calls{0},pid{0},uid{0};
static AIBinder_Class* response_class;
static AIBinder_Class* indication_class;
static void require(bool value,const char* message){if(!value){fprintf(stderr,"NDK FAIL: %s\n",message);exit(1);}}
static void ok(binder_status_t status){if(status!=STATUS_OK){fprintf(stderr,"NDK Binder status: %d\n",status);exit(1);}}
static void* create(void* state){return state;}
static void destroy(void*){}
static binder_status_t unused(AIBinder*,transaction_code_t,const AParcel*,AParcel*){return STATUS_UNKNOWN_TRANSACTION;}
static bool string_buffer(void* buffer,int32_t length,char** out){if(length<1||length>64)return false;*out=static_cast<char*>(buffer);return true;}
static void write_ok(AParcel* out){AStatus* status=AStatus_newOk();ok(AParcel_writeStatusHeader(out,status));AStatus_delete(status);}
static void callback(AIBinder* target,bool indication){
    AParcel *in=nullptr,*out=nullptr;ok(AIBinder_prepareTransaction(target,&in));
    if(indication){ok(AParcel_writeInt32(in,33));ok(AParcel_writeInt32(in,0));const int8_t bytes[]={1,2,3};ok(AParcel_writeByteArray(in,bytes,3));}
    else {for(int32_t value:{1,24,0,7,137,1,0})ok(AParcel_writeInt32(in,value));const int8_t bytes[]={7};ok(AParcel_writeByteArray(in,bytes,1));}
    ok(AIBinder_transact(target,1,&in,&out,FLAG_ONEWAY));if(out)AParcel_delete(out);
}
static binder_status_t transact(AIBinder*,transaction_code_t code,const AParcel* in,AParcel* out){
    if(code==5){
        AIBinder *response=nullptr,*indication=nullptr;char client[64]={};
        ok(AParcel_readStrongBinder(in,&response));ok(AParcel_readStrongBinder(in,&indication));ok(AParcel_readString(in,client,string_buffer));
        require(response&&indication&&AIBinder_isRemote(response)&&AIBinder_isRemote(indication),"remote callback objects");
        require(AIBinder_associateClass(response,response_class)&&AIBinder_associateClass(indication,indication_class),"callback descriptors");
        calls++;pid=AIBinder_getCallingPid();uid=AIBinder_getCallingUid();
        printf("NDK REGISTER pid=%d uid=%d response_remote=1 indication_remote=1\n",pid.load(),uid.load());fflush(stdout);
        if(strcmp(client,"die-register")==0)_exit(0);
        if(strcmp(client,"callbacks")==0){
            AParcel *request=nullptr,*reply=nullptr;ok(AIBinder_prepareTransaction(response,&request));
            ok(AIBinder_transact(response,16777215,&request,&reply,0));
            AStatus* status=nullptr;ok(AParcel_readStatusHeader(reply,&status));require(AStatus_isOk(status),"nested metadata status");AStatus_delete(status);
            int32_t version=0;ok(AParcel_readInt32(reply,&version));require(version==1,"nested metadata version");AParcel_delete(reply);
            callback(response,false);callback(indication,true);
        }else{require(strcmp(client,"active")==0,"known synthetic client");callback(response,false);}
        AIBinder_decStrong(response);AIBinder_decStrong(indication);write_ok(out);ok(AParcel_writeInt32(out,42));return STATUS_OK;
    }
    if(code==100){write_ok(out);for(int32_t value:{calls.load(),0,0,pid.load(),uid.load(),0})ok(AParcel_writeInt32(out,value));return STATUS_OK;}
    if(code==107)_exit(0);
    return STATUS_UNKNOWN_TRANSACTION;
}
int main(){
    Dl_info info={};require(dladdr(reinterpret_cast<void*>(&AIBinder_Class_define),&info)!=0,"original library identity");
    printf("GENUINE-NDK-LOADED %s\n",info.dli_fname);fflush(stdout);
    auto set_threads=reinterpret_cast<decltype(&ABinderProcess_setThreadPoolMaxThreadCount)>(dlsym(RTLD_DEFAULT,"ABinderProcess_setThreadPoolMaxThreadCount"));
    auto start_pool=reinterpret_cast<decltype(&ABinderProcess_startThreadPool)>(dlsym(RTLD_DEFAULT,"ABinderProcess_startThreadPool"));
    auto join_pool=reinterpret_cast<decltype(&ABinderProcess_joinThreadPool)>(dlsym(RTLD_DEFAULT,"ABinderProcess_joinThreadPool"));
    auto add_service=reinterpret_cast<decltype(&AServiceManager_addService)>(dlsym(RTLD_DEFAULT,"AServiceManager_addService"));
    require(set_threads&&start_pool&&join_pool&&add_service,"genuine platform symbols");require(set_threads(3),"bounded binder thread pool");start_pool();
    response_class=AIBinder_Class_define("vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalRes",create,destroy,unused);
    indication_class=AIBinder_Class_define("vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalInd",create,destroy,unused);
    AIBinder_Class* radio_class=AIBinder_Class_define(RADIO,create,destroy,transact);require(radio_class&&response_class&&indication_class,"class creation");
    AIBinder* radio=AIBinder_new(radio_class,nullptr);require(radio!=nullptr,"native endpoint");
    binder_exception_t status=add_service(radio,"org.pokelinux.fixture.arm64-radio");printf("NDK-ADD-SERVICE %d\n",status);fflush(stdout);require(status==EX_NONE,"native service-manager registration");
    printf("READY-NDK pid=%d\n",getpid());fflush(stdout);join_pool();return 0;
}
