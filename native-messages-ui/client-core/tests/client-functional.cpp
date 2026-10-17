// Independent frozen full-engine C ABI consumer. No Qt and no implementation includes.
// Linux root socketpair supplies actual packets and kernel credentials. Core calls
// model the adapter schedule; this does not claim QThread, actor, GUI or hardware.
#include "pokelinux_messages_client.h"
#include <cassert>
#include <cerrno>
#include <chrono>
#include <cstring>
#include <iostream>
#include <limits>
#include <string>
#include <thread>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <signal.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
static uint64_t boot(){timespec t{};assert(clock_gettime(CLOCK_BOOTTIME,&t)==0);assert(t.tv_sec>=0);return uint64_t(t.tv_sec)*1000000000+uint64_t(t.tv_nsec);}
static void nap(unsigned ms){std::this_thread::sleep_for(std::chrono::milliseconds(ms));}
static const std::string A(32,'1'),B(32,'2'),C(32,'3');
static PrMessagesIdV1 id(const std::string&s){PrMessagesIdV1 r{};assert(s.size()<=32);r.length=uint32_t(s.size());std::memcpy(r.bytes,s.data(),s.size());return r;}
static std::string conv(const std::string& a=A,const std::string& b=B){return "{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\"Alice\",\"slot\":0},{\"id\":\""+b+"\",\"sender\":\"Bob\",\"slot\":7}]}";}
static std::string msg(){return "{\"page\":\"messages\",\"items\":[{\"receipt\":\""+C+"\",\"sender\":\"Alice\",\"text\":\"<b>literal</b>\",\"ordinal\":1},{\"receipt\":\""+B+"\",\"sender\":\"Bob\",\"text\":\"two\",\"ordinal\":4}]}";}
static std::string cv(const PrMessagesViewV1&v){assert(v.conversations_length<=4096);return {reinterpret_cast<const char*>(v.conversations_json),v.conversations_length};}
static std::string mv(const PrMessagesViewV1&v){assert(v.messages_length<=4096);return {reinterpret_cast<const char*>(v.messages_json),v.messages_length};}
static std::string ids(const PrMessagesIdV1& v){assert(v.length<=32);return {reinterpret_cast<const char*>(v.bytes),v.length};}
struct Core {
 PrMessagesClient* h=nullptr;int fd[2]{-1,-1};bool entered=false,stopped=false;unsigned actualCloseCount=0;
 Core(){assert(geteuid()==0);assert(pr_messages_client_new_v1(&h)==PR_MESSAGES_OK&&h);}
 void connectNow(){assert(fd[0]<0);if(fd[1]>=0){assert(close(fd[1])==0);fd[1]=-1;}assert(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0,fd)==0);int one=1;assert(setsockopt(fd[0],SOL_SOCKET,SO_PASSCRED,&one,sizeof(one))==0);ucred cred{};socklen_t n=sizeof(cred);assert(getsockopt(fd[0],SOL_SOCKET,SO_PEERCRED,&cred,&n)==0&&cred.uid==0&&cred.pid==getpid());}
 void closeNative(){if(fd[0]>=0){const int actual=fd[0];assert(close(actual)==0);fd[0]=-1;++actualCloseCount;errno=0;assert(fcntl(actual,F_GETFD)==-1&&errno==EBADF);}}
 void peerEOF(){assert(fd[0]<0&&fd[1]>=0);char byte{};assert(recv(fd[1],&byte,1,MSG_DONTWAIT)==0);}
 PrMessagesViewV1 view(){PrMessagesViewV1 v{};assert(pr_messages_client_view_v1(h,&v)==PR_MESSAGES_OK);return v;}
 PrMessagesStepV1 step(){PrMessagesStepV1 p{};assert(pr_messages_client_step_v1(h,&p)==PR_MESSAGES_OK);assert(p.reserved==0);return p;}
 void noRepeatedNotify(){PrMessagesStepV1 sentinel;std::memset(&sentinel,0xa5,sizeof(sentinel));const auto unchanged=sentinel;assert(pr_messages_client_step_v1(h,&sentinel)==PR_MESSAGES_WAIT);assert(std::memcmp(&sentinel,&unchanged,sizeof(sentinel))==0);}
 void enter(){assert(pr_messages_client_worker_enter_v1(h)==PR_MESSAGES_OK);entered=true;}
 int report(const PrMessagesStepV1&p,uint32_t result,uint32_t sent=0){return pr_messages_client_report_v1(h,&p,result,sent);}
 PrMessagesStepV1 sendPlan(){if(!entered)enter();const auto until=boot()+2000000000;for(;;){auto p=step();if(p.action==PR_MESSAGES_CONNECT){connectNow();assert(report(p,PR_MESSAGES_CONNECTED)==PR_MESSAGES_OK);continue;}if(p.action==PR_MESSAGES_SEND)return p;assert(p.action==PR_MESSAGES_SLEEP&&p.wait_slice_ms<=10&&p.wait_slice_ms>0);assert(boot()<until);nap(p.wait_slice_ms);}}
 std::string nativeSend(const PrMessagesStepV1&p){assert(p.action==PR_MESSAGES_SEND&&p.packet_length>0&&p.packet_length<=4096&&fd[0]>=0);assert(send(fd[0],p.packet,p.packet_length,MSG_NOSIGNAL|MSG_DONTWAIT)==ssize_t(p.packet_length));char b[4096];const auto n=recv(fd[1],b,sizeof(b),MSG_DONTWAIT);assert(n==ssize_t(p.packet_length));return {b,size_t(n)};}
 std::string transmit(const PrMessagesStepV1&p){auto wire=nativeSend(p);assert(report(p,PR_MESSAGES_SENT,uint32_t(wire.size()))==PR_MESSAGES_OK);return wire;}
 PrMessagesStepV1 nativeReceiveUnacknowledged(const PrMessagesStepV1& p,const std::string& wire,int expected=PR_MESSAGES_OK){assert(p.action==PR_MESSAGES_RECEIVE&&wire.size()<=4096);assert(send(fd[1],wire.data(),wire.size(),MSG_NOSIGNAL|MSG_DONTWAIT)==ssize_t(wire.size()));char bytes[4096];alignas(cmsghdr) char ancillary[CMSG_SPACE(sizeof(ucred))]{};iovec v{bytes,sizeof(bytes)};msghdr m{};m.msg_iov=&v;m.msg_iovlen=1;m.msg_control=ancillary;m.msg_controllen=sizeof(ancillary);const auto n=recvmsg(fd[0],&m,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);assert(n==ssize_t(wire.size())&&(m.msg_flags&(MSG_TRUNC|MSG_CTRUNC))==0);auto*c=CMSG_FIRSTHDR(&m);assert(c&&c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS&&c->cmsg_len==CMSG_LEN(sizeof(ucred))&&!CMSG_NXTHDR(&m,c));ucred u{};std::memcpy(&u,CMSG_DATA(c),sizeof(u));assert(u.pid==getpid()&&u.uid==0);assert(pr_messages_client_received_v1(h,&p,reinterpret_cast<uint8_t*>(bytes),uint32_t(n))==expected);std::memset(bytes,'X',size_t(n));auto notify=step();assert(notify.action==PR_MESSAGES_NOTIFY&&notify.event_token&&notify.wake_boottime_ns>0);return notify;}
 PrMessagesStepV1 pendingReply(const std::string&wire){return nativeReceiveUnacknowledged(step(),wire);}
 uint64_t receivePlan(const PrMessagesStepV1& p,const std::string& wire,int expected=PR_MESSAGES_OK){const auto notify=nativeReceiveUnacknowledged(p,wire,expected);assert(report(notify,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);return notify.event_token;}
 uint64_t receive(const std::string& wire){return receivePlan(step(),wire);}
 uint64_t queued(){auto p=step();assert(p.action==PR_MESSAGES_NOTIFY&&p.event_token&&p.wake_boottime_ns);assert(report(p,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);return p.event_token;}
 void deliver(uint64_t token,int expected=PR_MESSAGES_OK){assert(pr_messages_client_deliver_v1(h,token)==expected);}
 std::string exchange(const std::string& wire){const auto p=sendPlan();auto request=transmit(p);deliver(receive(wire));return request;}
 void closeAndAcknowledge(const PrMessagesStepV1&p){assert(p.action==PR_MESSAGES_CLOSE);closeNative();assert(fd[0]<0);assert(report(p,PR_MESSAGES_CLOSED)==PR_MESSAGES_OK);}
 void closePending(){auto p=step();if(p.action==PR_MESSAGES_CLOSE)closeAndAcknowledge(p);}
 void shutdown(){if(stopped)return;assert(pr_messages_client_stop_v1(h)==PR_MESSAGES_OK);if(entered){for(int i=0;i<3;++i){const auto p=step();if(p.action==PR_MESSAGES_EXIT)break;assert(p.action==PR_MESSAGES_CLOSE);closeAndAcknowledge(p);}assert(step().action==PR_MESSAGES_EXIT);closeNative();assert(fd[0]<0);assert(pr_messages_client_worker_exited_v1(h)==PR_MESSAGES_OK);assert(pr_messages_client_join_result_v1(h,1)==PR_MESSAGES_OK);}closeNative();assert(pr_messages_client_free_v1(h)==PR_MESSAGES_OK);h=nullptr;stopped=true;}
 ~Core(){shutdown();for(int f:fd)if(f>=0)close(f);}
};
static void initial_and_packets(){
 Core c;auto v=c.view();assert(v.generation==0&&v.model_revision==0&&v.status==PR_MESSAGES_UNAVAILABLE&&v.worker_active==0&&cv(v)=="[]"&&mv(v)=="[]");
 c.enter();assert(pr_messages_client_worker_enter_v1(c.h)==PR_MESSAGES_REFUSED);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_REFUSED);
 auto p=c.step();assert(p.action==PR_MESSAGES_CONNECT&&p.connection_epoch>0&&p.request_token==0&&p.original_until_boottime_ns==0);const auto old=p;c.connectNow();assert(c.report(p,PR_MESSAGES_CONNECTED)==PR_MESSAGES_OK);assert(c.report(old,PR_MESSAGES_CONNECTED)==PR_MESSAGES_STALE);
 p=c.step();assert(p.action==PR_MESSAGES_SEND&&p.request_token&&p.original_until_boottime_ns>boot());auto copied=p;copied.packet[0]='X';assert(copied.packet[0]=='X');const auto same=c.step();assert(same.packet[0]=='{'&&same.request_token==p.request_token&&same.original_until_boottime_ns==p.original_until_boottime_ns);
 assert(c.report(p,PR_MESSAGES_WOULD_BLOCK)==PR_MESSAGES_OK);assert(c.report(p,PR_MESSAGES_INTERRUPTED)==PR_MESSAGES_OK);assert(c.step().original_until_boottime_ns==p.original_until_boottime_ns);
 assert(c.transmit(p)=="{\"op\":\"messages_read\",\"request\":{\"query\":{\"after\":null,\"limit\":8,\"query\":\"conversations\"},\"ttl_ms\":1000}}");assert(c.report(p,PR_MESSAGES_SENT,p.packet_length)==PR_MESSAGES_STALE);
 c.deliver(c.receive(conv()));v=c.view();assert(v.status==PR_MESSAGES_HISTORY&&ids(v.selected)==A&&v.after_ordinal==0&&cv(v).find("Alice")!=std::string::npos&&mv(v)=="[]");
 const auto query=c.exchange(msg());assert(query=="{\"op\":\"messages_read\",\"request\":{\"query\":{\"after_ordinal\":0,\"conversation\":\""+A+"\",\"limit\":2,\"query\":\"messages\"},\"ttl_ms\":1000}}");assert(mv(c.view()).find("<b>literal</b>")!=std::string::npos);
 auto out=c.view();out.conversations_json[0]='X';assert(out.conversations_json[0]=='X');assert(cv(c.view())[0]=='[');std::cout<<"PASS initial_packets_and_copied_model\n";
}
static void navigation(){
 Core c;c.exchange(conv());c.exchange(msg());auto before=c.view();assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_NEXT_MESSAGES,nullptr)==PR_MESSAGES_OK);auto v=c.view();assert(v.generation==before.generation+1&&v.after_ordinal==4&&mv(v)=="[]"&&cv(v)==cv(before)&&v.status==before.status);
 c.exchange(conv());const auto q=c.exchange(msg());assert(q.find("\"after_ordinal\":4")!=std::string::npos);
 assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_FIRST_MESSAGES,nullptr)==PR_MESSAGES_OK&&c.view().after_ordinal==0);
 const auto b=id(B);assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_SELECT,&b)==PR_MESSAGES_OK&&ids(c.view().selected)==B&&mv(c.view())=="[]");
 auto invalid=id(std::string(32,'A'));before=c.view();assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_SELECT,&invalid)==PR_MESSAGES_INVALID&&c.view().generation==before.generation);
 assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_NEXT_CONVERSATIONS,nullptr)==PR_MESSAGES_OK);v=c.view();assert(ids(v.conversations_after)==B&&ids(v.selected).empty()&&v.after_ordinal==0&&cv(v)=="[]"&&mv(v)=="[]");
 assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_NEXT_CONVERSATIONS,nullptr)==PR_MESSAGES_EMPTY);assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_NEXT_MESSAGES,nullptr)==PR_MESSAGES_EMPTY);
 const auto page=c.exchange(conv(C,A));assert(page.find("\"after\":\""+B+"\"")!=std::string::npos&&ids(c.view().selected)==C);
 assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_FIRST_CONVERSATIONS,nullptr)==PR_MESSAGES_OK);v=c.view();assert(ids(v.selected).empty()&&ids(v.conversations_after).empty()&&cv(v)=="[]"&&mv(v)=="[]");
 std::cout<<"PASS navigation_and_generation\n";
}
static void queued_delivery(){
 Core c;auto p=c.sendPlan();c.transmit(p);const auto token=c.receive(conv());assert(c.view().queued_events==1&&cv(c.view())=="[]");
 auto chosen=id(B);assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_SELECT,&chosen)==PR_MESSAGES_OK);c.deliver(token,PR_MESSAGES_STALE);auto v=c.view();assert(ids(v.selected)==B&&cv(v)=="[]"&&v.status==PR_MESSAGES_UNAVAILABLE&&v.queued_events==0);c.deliver(token,PR_MESSAGES_STALE);
 c.exchange(msg());assert(!mv(c.view()).empty());p=c.sendPlan();c.transmit(p);const auto delayed=c.receive(conv());const auto revision=c.view().model_revision;nap(1100);c.deliver(delayed);assert(c.view().model_revision==revision+1&&c.view().status==PR_MESSAGES_HISTORY);
 p=c.sendPlan();c.transmit(p);const auto first=c.receive(msg());p=c.sendPlan();c.transmit(p);const auto second=c.receive(conv());assert(c.view().queued_events==2);c.deliver(second,PR_MESSAGES_STALE);assert(c.view().queued_events==2);c.deliver(first);c.deliver(second);assert(c.view().queued_events==0);
 std::cout<<"PASS_queued_fifo_stale_and_timely_receive_delayed_ui\n";
}
static void parser_and_fail_closed(){
 struct Case{std::string bytes;uint32_t status;};
 const std::string good="{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"Alice\",\"slot\":0,\"extra\":{\"kept\":true}}]}";
 const Case cases[]={
  {good,PR_MESSAGES_HISTORY},
  {"{\"page\":\"conversations\",\"items\":[]}",PR_MESSAGES_HISTORY},
  {"{\"page\":\"messages\",\"items\":[]}",PR_MESSAGES_HISTORY},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"A\",\"slot\":7.0}]}",PR_MESSAGES_HISTORY},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"B\",\"text\":\"\",\"ordinal\":4e0}]}",PR_MESSAGES_HISTORY},
  {"{\"result\":\"denied\"}",PR_MESSAGES_DENIED},
  {"{\"result\":\"unavailable\"}",PR_MESSAGES_UNAVAILABLE},
  {"[]",PR_MESSAGES_UNAVAILABLE},{"{",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":null}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"\",\"slot\":0}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"A\",\"slot\":true}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"B\",\"text\":null,\"ordinal\":1}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"B\",\"text\":\"x\",\"ordinal\":4097}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"B\",\"text\":\"x\",\"ordinal\":1.5}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"A\",\"slot\":0},{\"id\":\""+B+"\",\"sender\":\"B\",\"slot\":8}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"B\",\"text\":\""+std::string(641,'x')+"\",\"ordinal\":1}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\""+std::string(65,'x')+"\",\"slot\":0}]}",PR_MESSAGES_UNAVAILABLE},
  {"{\"page\":\"conversations\",\"items\":[{\"id\":\""+A+"\",\"sender\":\"A\",\"slot\":0,\"slot\":7}]}",PR_MESSAGES_HISTORY},
  {"{\"page\":\"messages\",\"items\":[{\"receipt\":\""+B+"\",\"sender\":\"\\ud800\",\"text\":\"x\",\"ordinal\":1}]}",PR_MESSAGES_UNAVAILABLE}
 };
 for(const auto& test:cases){Core c;c.exchange(test.bytes);const auto v=c.view();assert(v.status==test.status);if(test.status!=PR_MESSAGES_HISTORY)assert(cv(v)=="[]"&&mv(v)=="[]");if(test.bytes==good)assert(cv(v).find("extra")!=std::string::npos);}
 Core c;c.exchange(conv());c.exchange(msg());auto selected=c.view().selected;c.exchange("{\"result\":\"denied\"}");auto v=c.view();assert(v.status==PR_MESSAGES_DENIED&&cv(v)=="[]"&&mv(v)=="[]"&&ids(v.selected)==ids(selected));
 std::cout<<"PASS bounded_parser_atomic_failure_and_denial\n";
}
static void real_expiry(){
 {Core c;auto p=c.sendPlan();const auto original=p.original_until_boottime_ns;nap(1100);assert(c.view().original_until_boottime_ns==original);auto next=c.step();assert(next.action==PR_MESSAGES_NOTIFY);assert(c.report(p,PR_MESSAGES_SENT,p.packet_length)==PR_MESSAGES_STALE);const auto t=next.event_token;assert(c.report(next,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);c.deliver(t);assert(c.view().status==PR_MESSAGES_UNAVAILABLE);c.closePending();}
 {Core c;auto p=c.sendPlan();c.transmit(p);auto recv=c.step();assert(recv.action==PR_MESSAGES_RECEIVE);nap(1100);c.deliver(c.receivePlan(recv,conv(),PR_MESSAGES_EXPIRED));assert(cv(c.view())=="[]");c.closePending();}
 std::cout<<"PASS real_boottime_expiry_without_timer_delivery\n";
}
static void reconnect_and_stale_plans(){
 Core c;c.enter();auto connect=c.step();assert(c.report(connect,PR_MESSAGES_CONNECT_FAILED)==PR_MESSAGES_OK);auto wait=c.step();assert(wait.action==PR_MESSAGES_SLEEP&&wait.wake_boottime_ns>boot());auto same=c.step();assert(same.wake_boottime_ns==wait.wake_boottime_ns);
 auto p=c.sendPlan();assert(p.connection_epoch>connect.connection_epoch);assert(c.report(connect,PR_MESSAGES_CONNECTED)==PR_MESSAGES_STALE);c.transmit(p);auto recv=c.step();assert(c.report(recv,PR_MESSAGES_IO_FAILED)==PR_MESSAGES_OK);const auto token=c.queued();c.deliver(token);c.closePending();const auto q=c.sendPlan();assert(q.connection_epoch>p.connection_epoch&&q.request_token!=p.request_token);assert(c.report(p,PR_MESSAGES_SENT,p.packet_length)==PR_MESSAGES_STALE&&c.step().request_token==q.request_token);
 std::cout<<"PASS retry_wake_connection_epoch_and_request_tokens\n";
}
static void shutdown_ownership(){
 {Core c;c.enter();auto p=c.step();assert(p.action==PR_MESSAGES_CONNECT);assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_FIRST_MESSAGES,nullptr)==PR_MESSAGES_REFUSED);auto mustClose=c.step();assert(mustClose.action==PR_MESSAGES_CLOSE);assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_REFUSED);c.closeAndAcknowledge(mustClose);assert(c.step().action==PR_MESSAGES_EXIT);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_REFUSED);assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_OK);assert(pr_messages_client_join_result_v1(c.h,0)==PR_MESSAGES_WAIT);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_REFUSED);assert(pr_messages_client_join_result_v1(c.h,1)==PR_MESSAGES_OK);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_OK);c.h=nullptr;c.stopped=true;}
 {const auto until=boot()+2000000000;const auto child=fork();assert(child>=0);if(child==0){Core c;c.sendPlan();assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);auto p=c.step();assert(p.action==PR_MESSAGES_CLOSE);c.closeAndAcknowledge(p);assert(c.step().action==PR_MESSAGES_EXIT);assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_OK);nap(550);assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);assert(pr_messages_client_join_result_v1(c.h,0)==PR_MESSAGES_ABORT);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_REFUSED);_exit(0);}int status=0;bool waited=false;while(boot()<until){auto result=waitpid(child,&status,WNOHANG);if(result==child){waited=true;break;}assert(result==0);nap(1);}if(!waited){kill(child,SIGKILL);assert(waitpid(child,&status,0)==child);}assert(waited&&WIFEXITED(status)&&WEXITSTATUS(status)==0);std::cout<<"ACTUAL_COMPONENT_CHILD_WAIT pid="<<child<<" exit=0 original_until_ns="<<until<<" observed_ns="<<boot()<<"\n";}
 {Core c;c.sendPlan();assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);c.closeAndAcknowledge(c.step());assert(c.step().action==PR_MESSAGES_EXIT&&c.fd[0]<0);assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_OK);nap(550);assert(pr_messages_client_join_result_v1(c.h,1)==PR_MESSAGES_OK);assert(pr_messages_client_free_v1(c.h)==PR_MESSAGES_OK);c.h=nullptr;c.stopped=true;}
 std::cout<<"PASS stop_budget_and_actual_join_required\n";
}
static void delivery_before_notification_ack(){
 // Component scheduling is explicit; packets/credentials/EOF are actual Linux
 // socketpair evidence. These calls do not claim an actual Qt callback/thread.
 {Core c;c.transmit(c.sendPlan());const auto n=c.pendingReply(conv());const auto originalWake=n.wake_boottime_ns;const auto revision=c.view().model_revision;assert(c.view().queued_events==1);c.deliver(n.event_token);assert(c.view().queued_events==0&&c.view().model_revision==revision+1&&ids(c.view().selected)==A);c.noRepeatedNotify();nap(20);assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_STALE);const auto next=c.step();assert((next.action==PR_MESSAGES_SLEEP&&next.wake_boottime_ns==originalWake)||(next.action==PR_MESSAGES_SEND&&boot()>=originalWake));c.deliver(n.event_token,PR_MESSAGES_STALE);}
 {Core c;c.transmit(c.sendPlan());const auto n=c.pendingReply(conv());const auto originalWake=n.wake_boottime_ns;auto chosen=id(B);assert(pr_messages_client_action_v1(c.h,PR_MESSAGES_SELECT,&chosen)==PR_MESSAGES_OK);const auto generation=c.view().generation;const auto revision=c.view().model_revision;c.deliver(n.event_token,PR_MESSAGES_STALE);assert(c.view().queued_events==0&&cv(c.view())=="[]"&&c.view().generation==generation&&c.view().model_revision==revision);c.noRepeatedNotify();const auto until=boot()+1000000000;while(boot()<=originalWake){assert(boot()<until);nap(1);}assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_STALE);const auto next=c.step();assert(next.action==PR_MESSAGES_SEND&&next.generation==generation);const auto query=c.transmit(next);assert(query.find("\"conversation\":\""+B+"\"")!=std::string::npos);}
 {Core c;c.transmit(c.sendPlan());const auto receive=c.step();assert(c.report(receive,PR_MESSAGES_IO_FAILED)==PR_MESSAGES_OK);const auto n=c.step();assert(n.action==PR_MESSAGES_NOTIFY);const auto originalWake=n.wake_boottime_ns;c.deliver(n.event_token);assert(c.view().queued_events==0&&c.view().status==PR_MESSAGES_UNAVAILABLE);c.noRepeatedNotify();assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_OK);assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_STALE);const auto close=c.step();assert(close.action==PR_MESSAGES_CLOSE);c.closeAndAcknowledge(close);c.peerEOF();assert(c.actualCloseCount==1);assert(c.report(close,PR_MESSAGES_CLOSED)==PR_MESSAGES_STALE);const auto next=c.step();assert((next.action==PR_MESSAGES_SLEEP&&next.wake_boottime_ns==originalWake)||(next.action==PR_MESSAGES_CONNECT&&boot()>=originalWake));}
 {Core c;c.transmit(c.sendPlan());const auto n=c.pendingReply(conv());const auto revision=c.view().model_revision;assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);c.deliver(n.event_token,PR_MESSAGES_REFUSED);assert(c.report(n,PR_MESSAGES_NOTIFY_QUEUED)==PR_MESSAGES_REFUSED);assert(c.view().queued_events==0&&c.view().model_revision==revision&&cv(c.view())=="[]");const auto close=c.step();assert(close.action==PR_MESSAGES_CLOSE);c.closeAndAcknowledge(close);c.peerEOF();assert(c.step().action==PR_MESSAGES_EXIT);}
 std::cout<<"PASS delivery_before_ack_stale_generation_fixed_wake_and_stop_notification_cancellation\n";
}
static void stopped_native_operation_reports(){
 // A copied operation has entered the mechanical edge; UI Stop wins while the
 // native completion/report is delayed. Successful actual native effect is not
 // claimed undone. No fresh core request/receive or identity is manufactured.
 {Core c;c.enter();const auto connect=c.step();assert(connect.action==PR_MESSAGES_CONNECT);assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_REFUSED);c.connectNow();assert(c.report(connect,PR_MESSAGES_CONNECTED)==PR_MESSAGES_REFUSED);assert(c.view().stopping==1&&c.view().request_token==0&&cv(c.view())=="[]");char b{};assert(recv(c.fd[1],&b,1,MSG_DONTWAIT)<0&&errno==EAGAIN);const auto close=c.step();assert(close.action==PR_MESSAGES_CLOSE);c.closeAndAcknowledge(close);c.peerEOF();assert(c.actualCloseCount==1&&c.fd[0]<0&&c.step().action==PR_MESSAGES_EXIT);assert(c.report(connect,PR_MESSAGES_CONNECTED)==PR_MESSAGES_REFUSED);assert(c.step().action==PR_MESSAGES_EXIT);}
 {Core c;const auto send=c.sendPlan();assert(send.request_token>0);assert(pr_messages_client_stop_v1(c.h)==PR_MESSAGES_OK);const auto actual=c.nativeSend(send);assert(!actual.empty());assert(c.report(send,PR_MESSAGES_SENT,uint32_t(actual.size()))==PR_MESSAGES_REFUSED);assert(c.view().stopping==1&&c.view().queued_events==0&&cv(c.view())=="[]");assert(pr_messages_client_worker_exited_v1(c.h)==PR_MESSAGES_REFUSED);const auto close=c.step();assert(close.action==PR_MESSAGES_CLOSE);c.closeAndAcknowledge(close);c.peerEOF();assert(c.actualCloseCount==1&&c.fd[0]<0&&c.step().action==PR_MESSAGES_EXIT);assert(c.report(send,PR_MESSAGES_SENT,uint32_t(actual.size()))==PR_MESSAGES_REFUSED);assert(c.step().action==PR_MESSAGES_EXIT);}
 std::cout<<"PASS stopped_late_connect_send_native_socket_EOF_EBADF_and_no_new_request\n";
}
int main(){initial_and_packets();navigation();queued_delivery();parser_and_fail_closed();real_expiry();reconnect_and_stale_plans();shutdown_ownership();delivery_before_notification_ack();stopped_native_operation_reports();std::cout<<"PASS all_9_full_engine_groups; native_socketpair_credentials_EOF_EBADF; Qt_integration_unclaimed\n";}
