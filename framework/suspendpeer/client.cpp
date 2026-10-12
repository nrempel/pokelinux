// Original generated NDK proxy into exact frozen Rust dispatcher, actual Binder.
#include "bootstrap.h"
#include <aidl/android/system/suspend/ISystemSuspend.h>
#include <aidl/android/system/suspend/IWakeLock.h>
#include <aidl/org/pokelinux/suspendtest/ISuspendControl.h>
#include <hardware_legacy/power.h>
#include <iostream>
#include <vector>
#include <cstring>
using namespace aidl::android::system::suspend;
using aidl::org::pokelinux::suspendtest::ISuspendControl;
static std::shared_ptr<ISystemSuspend> discovery;
static bool declared=true;
static int declared_calls=0, wait_calls=0;
extern "C" bool __wrap_AServiceManager_isDeclared(const char* name){ require(std::strcmp(name,"android.system.suspend.ISystemSuspend/default")==0,"original discovery name");declared_calls++;return declared; }
extern "C" AIBinder* __wrap_AServiceManager_waitForService(const char* name){require(std::strcmp(name,"android.system.suspend.ISystemSuspend/default")==0,"original wait name");wait_calls++;auto* b=discovery->asBinder().get();AIBinder_incStrong(b);return b;}
static std::vector<int64_t> snapshot(const std::shared_ptr<ISuspendControl>& c){std::vector<int64_t> v;ok(c->snapshot(&v));require(v.size()==11,"snapshot size");return v;}
static void count(const std::shared_ptr<ISuspendControl>& c,int expected){auto end=std::chrono::steady_clock::now()+std::chrono::seconds(2);while(snapshot(c)[0]!=expected){require(std::chrono::steady_clock::now()<end,"wake count mismatch");usleep(1000);}}
static void command(){std::string value;require(bool(std::getline(std::cin,value))&&value=="continue","owned command missing");}
int main(int argc,char** argv){
    require(argc==2,"scenario required");std::string mode=argv[1];
    auto b=bootstrap();auto peer=ISuspendBoundaryProbe::fromBinder(b);require(peer&&peer->isRemote(),"remote probe");
    std::shared_ptr<ISystemSuspend> service;std::shared_ptr<ISuspendControl> control;ok(peer->service(&service));ok(peer->control(&control));discovery=service;
    int32_t version;std::string hash;ok(service->getInterfaceVersion(&version));ok(service->getInterfaceHash(&hash));require(version==1&&hash=="082a58c2505b703702f34ab8abcb1d1b5e890be4","frozen V1 metadata");
    if(mode=="basic"){
        std::shared_ptr<IWakeLock> w;ok(service->acquireWakeLock(WakeLockType::FULL,"synthetic",&w));require(w&&w->isRemote(),"returned remote wake object");count(control,1);
        ok(w->getInterfaceVersion(&version));ok(w->getInterfaceHash(&hash));require(version==1&&hash=="082a58c2505b703702f34ab8abcb1d1b5e890be4","wake V1 metadata");
        auto s=snapshot(control);require(s[4]==2000&&s[5]==getpid(),"kernel native caller identity");std::string sid;ok(control->callerSid(&sid));require(sid=="u:r:shell:s0","kernel caller SID");puts("frozen-v1-native-identity\tPASS");
        ok(control->holdRelease(true));auto start=std::chrono::steady_clock::now();ok(w->release());require(std::chrono::steady_clock::now()-start<std::chrono::milliseconds(300),"release not one-way");require(snapshot(control)[0]==1,"one-way receipt became release confirmation");puts("oneway-release-receipt\tPASS");
        ok(control->holdRelease(false));count(control,0);ok(w->release());w.reset();count(control,0);require(snapshot(control)[3]==1,"release accounting not once");puts("explicit-release-once\tPASS");
        ok(service->acquireWakeLock(WakeLockType::PARTIAL,"destroy",&w));count(control,1);w.reset();count(control,0);puts("last-binder-reference\tPASS");
        int32_t rejected;ok(peer->foreignAcquire(&rejected));require(rejected==EX_SECURITY,"app caller was not rejected");s=snapshot(control);require(s[4]>=10000&&s[5]>0&&s[0]==0,"foreign identity not observed");puts("foreign-native-caller-rejected\tPASS");
    }else if(mode=="default-gate"){
        auto real_declared=reinterpret_cast<bool(*)(const char*)>(dlsym(RTLD_DEFAULT,"AServiceManager_isDeclared"));
        auto real_check=reinterpret_cast<AIBinder*(*)(const char*)>(dlsym(RTLD_DEFAULT,"AServiceManager_checkService"));
        require(real_declared&&real_check,"platform discovery functions");
        const char* name="android.system.suspend.ISystemSuspend/default";
        ndk::SpAIBinder stock(real_check(name));printf("default-observation\tdeclared=%d\tfound=%d\n",real_declared(name),stock.get()!=nullptr);
    }else if(mode=="snapshot"){
        auto s=snapshot(control);for(auto v:s)printf("%lld\t",static_cast<long long>(v));puts("");
    }else if(mode=="transfer-owner"){
        std::shared_ptr<IWakeLock> w;ok(service->acquireWakeLock(WakeLockType::PARTIAL,"transfer",&w));ok(peer->retain(w));w.reset();count(control,1);puts("transfer-owner\tPASS");
    }else if(mode=="holder"){
        std::shared_ptr<IWakeLock> w;ok(peer->borrow(&w));ok(peer->dropRetained());count(control,1);printf("holder\t%d\tREADY\n",getpid());fflush(stdout);command();w.reset();count(control,0);puts("holder-release\tPASS");
    }else if(mode=="authority"){
        auto* klass=AIBinder_Class_define("org.pokelinux.suspendtest.Authority",create,destroy,unused_transaction);ndk::SpAIBinder token(AIBinder_new(klass,nullptr));ok(control->arm(token));require(snapshot(control)[6]==1,"authority not armed");printf("authority\t%d\tREADY\n",getpid());fflush(stdout);command();
    }else if(mode=="fenced"){
        auto s=snapshot(control);require(s[1]==3&&s[6]==0,"authority death not terminal");int32_t attempt;ok(control->attempt(&attempt));require(attempt==0,"dead authority attempted suspend");auto* klass=AIBinder_Class_define("org.pokelinux.suspendtest.Authority",create,destroy,unused_transaction);ndk::SpAIBinder token(AIBinder_new(klass,nullptr));require(!control->arm(token).isOk(),"implicit rearm");puts("authority-death-fenced\tPASS");
    }else if(mode=="null-cache"){
        declared=false;require(acquire_wake_lock(0,"missing")==-1,"initial missing service");declared=true;require(acquire_wake_lock(0,"later")==-1&&declared_calls==1&&wait_calls==0,"cached null was rediscovered");puts("libpower-cached-null\tPASS");
    }else if(mode=="power-cache"){
        auto baseline=snapshot(control)[2];require(acquire_wake_lock(1,"held")==0&&acquire_wake_lock(1,"held")==0,"original acquire/dedup");count(control,1);require(snapshot(control)[2]==baseline+1,"duplicate libpower name invoked RPC");puts("libpower-name-dedup\tPASS");fflush(stdout);command();
        require(control->die().getStatus()==STATUS_DEAD_OBJECT,"owned server death missing");
        require(acquire_wake_lock(0,"held")==0,"cached held name after death");require(acquire_wake_lock(0,"new")==-1,"dead cached proxy accepted fresh name");require(release_wake_lock("held")==0&&release_wake_lock("held")==-1,"original release after death");puts("libpower-dead-cache\tPASS");fflush(stdout);command();
        require(acquire_wake_lock(0,"restart")==-1&&declared_calls==1&&wait_calls==1,"service restart caused implicit rediscovery");puts("libpower-restart-stays-dead\tPASS");
    }else require(false,"unknown scenario");
    return 0;
}
