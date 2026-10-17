// Source-frozen independent actual-QThread regression; no injected wait result.
#include "MessagesClient.h"
#include <QCoreApplication>
#include <QDeadlineTimer>
#include <QThread>
#include <QTemporaryDir>
#include <atomic>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>
namespace {
std::atomic<bool> enteredWrapper{false},releaseEntry{false},realEntered{false};
std::atomic<bool> stopped{false},lastWait{false},joined{false},exited{false};
std::atomic<unsigned> falseWaits{0},trueWaits{0},firstFalseJoins{0},socketCalls{0},exitPlans{0},frees{0};
std::atomic<PrMessagesClient*> actualCore{nullptr};
thread_local bool workerRole=false;
uint64_t originalUntil=0;
uint64_t boot(){timespec t{};assert(clock_gettime(CLOCK_BOOTTIME,&t)==0);assert(t.tv_sec>=0&&t.tv_nsec>=0&&t.tv_nsec<1000000000);return uint64_t(t.tv_sec)*1000000000ULL+uint64_t(t.tv_nsec);}
void bound(){assert(boot()<originalUntil);}
void same(PrMessagesClient* core){assert(core&&core==actualCore.load());}
}
extern "C" int32_t __real_pr_messages_client_worker_enter_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_worker_enter_v1(PrMessagesClient* core){
 workerRole=true;PrMessagesClient* empty=nullptr;assert(actualCore.compare_exchange_strong(empty,core));
 assert(!enteredWrapper.exchange(true));
 std::printf("PRE_ENTRY_HELD before_real_entry=1 original_until_ns=%llu\n",(unsigned long long)originalUntil);
 while(!releaseEntry.load()){bound();usleep(1000);}
 bound();assert(stopped.load()&&firstFalseJoins.load()==1);
 const auto code=__real_pr_messages_client_worker_enter_v1(core);
 std::printf("PRE_ENTRY_REAL_ENTER code=%d\n",code);assert(code==PR_MESSAGES_OK);realEntered=true;return code;
}
extern "C" int32_t __real_pr_messages_client_stop_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_stop_v1(PrMessagesClient* core){
 same(core);assert(enteredWrapper.load()&&!releaseEntry.load());
 const auto code=__real_pr_messages_client_stop_v1(core);assert(code==PR_MESSAGES_OK);stopped=true;
 std::printf("PRE_ENTRY_ACTUAL_STOP code=%d\n",code);return code;
}
extern "C" bool __real__ZN7QThread4waitE14QDeadlineTimer(QThread*,QDeadlineTimer);
extern "C" bool __wrap__ZN7QThread4waitE14QDeadlineTimer(QThread* thread,QDeadlineTimer deadline){
 bound();const bool completed=__real__ZN7QThread4waitE14QDeadlineTimer(thread,deadline);
 lastWait=completed;bound();
 if(completed){++trueWaits;assert(exited.load());}else{++falseWaits;if(!releaseEntry.load())assert(!realEntered.load());}
 std::printf("PRE_ENTRY_ACTUAL_QT_WAIT completed=%u before_release=%u observed_ns=%llu\n",completed?1U:0U,releaseEntry.load()?0U:1U,(unsigned long long)boot());
 return completed; // Never substitute/inject the native return.
}
extern "C" int32_t __real_pr_messages_client_join_result_v1(PrMessagesClient*,uint32_t);
extern "C" int32_t __wrap_pr_messages_client_join_result_v1(PrMessagesClient* core,uint32_t completed){
 same(core);assert(completed<=1&&completed==uint32_t(lastWait.load()));bound();
 const auto code=__real_pr_messages_client_join_result_v1(core,completed);
 if(!releaseEntry.load()){
  assert(completed==0&&falseWaits.load()>=1&&!realEntered.load()&&stopped.load());
  std::printf("PRE_ENTRY_FIRST_FALSE_JOIN completed=0 actual_code=%d expected_WAIT=%d before_release=1\n",code,PR_MESSAGES_WAIT);
  assert(code==PR_MESSAGES_WAIT); // Original buggy engine fails here; entry stays held.
  assert(firstFalseJoins.fetch_add(1)==0);releaseEntry=true;
 }else if(completed){assert(code==PR_MESSAGES_OK&&exited.load());joined=true;}
 else assert(code==PR_MESSAGES_WAIT);
 return code;
}
extern "C" int32_t __real_pr_messages_client_step_v1(PrMessagesClient*,PrMessagesStepV1*);
extern "C" int32_t __wrap_pr_messages_client_step_v1(PrMessagesClient* core,PrMessagesStepV1* plan){
 same(core);assert(workerRole&&realEntered.load()&&releaseEntry.load());bound();
 const auto code=__real_pr_messages_client_step_v1(core,plan);
 assert(code==PR_MESSAGES_OK&&plan&&plan->action==PR_MESSAGES_EXIT);++exitPlans;
 std::puts("PRE_ENTRY_ACTUAL_EXIT_PLAN no_CONNECT=1");return code;
}
extern "C" int32_t __real_pr_messages_client_worker_exited_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_worker_exited_v1(PrMessagesClient* core){
 same(core);assert(workerRole&&exitPlans.load()==1&&socketCalls.load()==0);bound();
 const auto code=__real_pr_messages_client_worker_exited_v1(core);assert(code==PR_MESSAGES_OK);exited=true;
 std::printf("PRE_ENTRY_REAL_WORKER_EXITED code=%d\n",code);return code;
}
extern "C" int __real_socket(int,int,int);
extern "C" int __wrap_socket(int domain,int type,int protocol){
 if(workerRole)++socketCalls;
 return __real_socket(domain,type,protocol); // Observe actual production calls; never fabricate a socket.
}
extern "C" int32_t __real_pr_messages_client_free_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_free_v1(PrMessagesClient* core){
 same(core);assert(joined.load()&&trueWaits.load()>=1&&exited.load()&&socketCalls.load()==0);bound();
 const auto code=__real_pr_messages_client_free_v1(core);assert(code==PR_MESSAGES_OK&&frees.fetch_add(1)==0);
 std::printf("PRE_ENTRY_REAL_FREE code=%d\n",code);return code;
}
int main(int argc,char** argv){
 std::setvbuf(stdout,nullptr,_IONBF,0);QCoreApplication app(argc,argv);QTemporaryDir dir;assert(dir.isValid());
 originalUntil=boot()+2000000000ULL;
 auto* client=new MessagesClient(dir.filePath("unused.sock"));
 while(!enteredWrapper.load()){bound();usleep(1000);}
 assert(!realEntered.load()&&!releaseEntry.load());delete client;bound();
 assert(firstFalseJoins.load()==1&&falseWaits.load()>=1&&trueWaits.load()>=1);
 assert(realEntered.load()&&exited.load()&&joined.load()&&frees.load()==1&&exitPlans.load()==1&&socketCalls.load()==0);
 std::printf("PASS pre_entry_actual_QThread_false_WAIT_then_true_OK_free no_worker_socket=1 original_until_ns=%llu completed_ns=%llu\n",(unsigned long long)originalUntil,(unsigned long long)boot());
 return 0;
}
