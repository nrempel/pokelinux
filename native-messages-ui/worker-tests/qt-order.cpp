// Source-only independent actual Qt/native ordering consumer; no production hooks.
// The root parent owns the actual peer, Qt-child wait and stdout PIPE EOF.
#include "MessagesClient.h"
#include "pokelinux_messages_client.h"
#include <QCoreApplication>
#include <QDeadlineTimer>
#include <QEventLoop>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryDir>
#include <QThread>
#include <algorithm>
#include <atomic>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>
#include <functional>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
namespace {
constexpr uint64_t Second=1000000000ULL;
constexpr uint64_t Cooldown=250000000ULL;
constexpr size_t OutputCap=4096;
enum class Case { Notify, Stale, Connect, Send, StopNotify };
Case mode=Case::Notify;
uint64_t cutoff=0;
int commands=-1, gates=-1;
char expectedPath[sizeof(sockaddr_un::sun_path)]{};
QThread *guiThread=nullptr;
std::mutex stateMutex;
PrMessagesStepV1 notifyPlan{};
struct stat socketIdentity{};
std::atomic<int> ownedSocket{-1}, nativeConnects{0}, nativeSends{0}, nativeReceives{0};
std::atomic<int> notifyPlans{0}, notifyAcks{0}, notifyResult{-99};
std::atomic<int> deliverCalls{0}, deliverResult{-99}, lateConnected{-99}, lateSent{-99};
std::atomic<int> nativeCloses{0}, closeReports{0}, workerExits{0}, nativeWaits{0};
std::atomic<int> trueWaits{0}, frees{0}, lastWait{-1};
std::atomic<bool> waitConsumed{true}, stopped{false}, holdActive{false}, ackFinished{false};
std::atomic<bool> ebadfSeen{false}, waitReleaseSent{false};
std::atomic<uint64_t> sequence{0}, closeSequence{0}, closedSequence{0};
std::atomic<uint64_t> exitSequence{0}, waitSequence{0}, freeSequence{0};
std::atomic<uint64_t> receivedBefore{0}, receivedAfter{0};
std::atomic<int> stopCalls{0};
[[noreturn]] void edgeFailure(const char *label) {
 std::fprintf(stderr,"ORDER_EDGE_FAILURE %s errno=%d\n",label,errno);std::fflush(stderr);_exit(101);
}
void edge(bool ok,const char *label){if(!ok){edgeFailure(label);}}
void need(bool ok,const char *label){if(!ok){throw std::runtime_error(label);}}
uint64_t boot(){timespec t{};edge(clock_gettime(CLOCK_BOOTTIME,&t)==0&&t.tv_sec>=0&&t.tv_nsec>=0&&t.tv_nsec<1000000000,"real BOOTTIME");return uint64_t(t.tv_sec)*Second+uint64_t(t.tv_nsec);}
uint64_t tick(){return ++sequence;}
int remainingMs(uint64_t until){const auto n=boot();if(n>=until){return 0;}return int(std::min<uint64_t>((until-n+999999)/1000000,20));}
void sendRecord(int fd,const QByteArray &bytes){
 edge(!bytes.isEmpty()&&bytes.size()<=8192,"bounded control record");
 while(boot()<cutoff){const auto n=::send(fd,bytes.constData(),size_t(bytes.size()),MSG_NOSIGNAL|MSG_DONTWAIT);if(n==bytes.size()){return;}if(n<0&&(errno==EINTR||errno==EAGAIN||errno==EWOULDBLOCK)){pollfd p{fd,POLLOUT,0};::poll(&p,1,remainingMs(cutoff));continue;}edgeFailure("whole control send");}edgeFailure("original control cutoff");
}
QByteArray readRecord(int fd){
 char bytes[8192];iovec vector{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&vector;message.msg_iovlen=1;
 const auto n=recvmsg(fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);
 if(n<0&&(errno==EAGAIN||errno==EWOULDBLOCK||errno==EINTR)){return {};}
 edge(n>0&&!(message.msg_flags&(MSG_TRUNC|MSG_CTRUNC)),"whole owned control record");return {bytes,int(n)};
}
void workerOnly(){edge(QThread::currentThread()!=guiThread,"actual worker caller");}
void guiOnly(){edge(QThread::currentThread()==guiThread,"actual GUI caller");}
void hold(const char *name){
 workerOnly();waitReleaseSent=false;holdActive=true;sendRecord(gates,QByteArray("HOLD ")+name);
 while(boot()<cutoff){pollfd p{gates,POLLIN,0};const auto r=poll(&p,1,remainingMs(cutoff));if(r<0&&errno==EINTR){continue;}edge(r>=0,"hold poll");if(r>0&&(p.revents&(POLLIN|POLLHUP))){const auto b=readRecord(gates);if(b.isEmpty()){continue;}edge(b==QByteArray("RELEASE ")+name,"exact actual-call release");holdActive=false;return;}}edgeFailure("held call original cutoff");
}
void trace(const char *kind,int result){std::fprintf(stderr,"ACTUAL_ORDER %s result=%d seq=%llu\n",kind,result,static_cast<unsigned long long>(tick()));std::fflush(stderr);}
}
extern "C" int __real_connect(int,const sockaddr*,socklen_t);
extern "C" int __wrap_connect(int fd,const sockaddr *address,socklen_t length){
 const bool selected=address&&length>=sizeof(sockaddr_un)&&address->sa_family==AF_UNIX&&std::strncmp(reinterpret_cast<const sockaddr_un*>(address)->sun_path,expectedPath,sizeof(expectedPath))==0;
 const int result=__real_connect(fd,address,length);const int saved=errno;
 if(selected&&result==0){workerOnly();edge(++nativeConnects==1,"one actual service connection");struct stat identity{};edge(fstat(fd,&identity)==0&&S_ISSOCK(identity.st_mode),"actual connected socket identity");{std::lock_guard<std::mutex> lock(stateMutex);socketIdentity=identity;}ownedSocket=fd;trace("connect-success",result);if(mode==Case::Connect){hold("connect");}}
 errno=saved;return result;
}
extern "C" ssize_t __real_send(int,const void*,size_t,int);
extern "C" ssize_t __wrap_send(int fd,const void *bytes,size_t size,int flags){
 const bool selected=fd==ownedSocket.load();const auto result=__real_send(fd,bytes,size,flags);const int saved=errno;
 if(selected&&result>0){workerOnly();edge(size<=4096&&result==ssize_t(size),"whole actual application send");edge(!stopped.load(),"no successor native send after Stop");const auto n=++nativeSends;trace("send-success",int(result));if(mode==Case::Send&&n==1){hold("send");}}
 errno=saved;return result;
}
extern "C" ssize_t __real_recvmsg(int,msghdr*,int);
extern "C" ssize_t __wrap_recvmsg(int fd,msghdr *message,int flags){
 const bool selected=fd==ownedSocket.load();if(selected){workerOnly();edge(!stopped.load(),"no new native receive after Stop");}
 const auto result=__real_recvmsg(fd,message,flags);if(selected&&result>0){++nativeReceives;trace("recvmsg-success",int(result));}return result;
}
extern "C" int __real_close(int);
extern "C" int __wrap_close(int fd){
 const bool selected=fd==ownedSocket.load();
 if(!selected){return __real_close(fd);}
 workerOnly();struct stat before{};edge(fstat(fd,&before)==0,"socket identity before close");{std::lock_guard<std::mutex> lock(stateMutex);edge(before.st_dev==socketIdentity.st_dev&&before.st_ino==socketIdentity.st_ino,"close exact acquired socket");}
 const int result=__real_close(fd);const int saved=errno;edge(result==0,"real owned socket close");errno=0;const int check=fcntl(fd,F_GETFD);edge(check<0&&errno==EBADF,"immediate actual fd EBADF");ebadfSeen=true;ownedSocket=-1;edge(++nativeCloses==1,"one actual owned close");closeSequence=tick();trace("native-close-and-EBADF",result);errno=saved;return result;
}
extern "C" int32_t __real_pr_messages_client_step_v1(PrMessagesClient*,PrMessagesStepV1*);
extern "C" int32_t __wrap_pr_messages_client_step_v1(PrMessagesClient *client,PrMessagesStepV1 *plan){
 workerOnly();const auto result=__real_pr_messages_client_step_v1(client,plan);
 if(result==PR_MESSAGES_OK){if(plan->action==PR_MESSAGES_NOTIFY){++notifyPlans;}
  if(ackFinished.load()&&notifyResult.load()==PR_MESSAGES_OK&&nativeSends.load()<2&&plan->action==PR_MESSAGES_SLEEP){std::lock_guard<std::mutex> lock(stateMutex);edge(plan->wake_boottime_ns==notifyPlan.wake_boottime_ns,"no ACK cooldown renewal");}}
 return result;
}
extern "C" int32_t __real_pr_messages_client_received_v1(PrMessagesClient*,const PrMessagesStepV1*,const uint8_t*,uint32_t);
extern "C" int32_t __wrap_pr_messages_client_received_v1(PrMessagesClient *client,const PrMessagesStepV1 *plan,const uint8_t *bytes,uint32_t length){
 workerOnly();const auto before=boot();const auto result=__real_pr_messages_client_received_v1(client,plan,bytes,length);const auto after=boot();
 if(length>0&&result==PR_MESSAGES_OK){edge(receivedAfter.load()==0,"one completed real reply");receivedBefore=before;receivedAfter=after;}return result;
}
extern "C" int32_t __real_pr_messages_client_report_v1(PrMessagesClient*,const PrMessagesStepV1*,uint32_t,uint32_t);
extern "C" int32_t __wrap_pr_messages_client_report_v1(PrMessagesClient *client,const PrMessagesStepV1 *plan,uint32_t outcome,uint32_t sent){
 workerOnly();if(outcome==PR_MESSAGES_NOTIFY_QUEUED){edge(++notifyAcks==1,"one real queued notification acknowledgement");{std::lock_guard<std::mutex> lock(stateMutex);notifyPlan=*plan;}edge(receivedAfter.load()!=0,"notify follows actual received boundary");edge(plan->wake_boottime_ns>=receivedBefore.load()+Cooldown&&plan->wake_boottime_ns<=receivedAfter.load()+Cooldown,"original completion anchored wake");hold("notify");}
 if(outcome==PR_MESSAGES_CLOSED){edge(nativeCloses.load()==1&&ownedSocket.load()==-1&&ebadfSeen.load(),"CLOSED follows actual close");}
 const auto result=__real_pr_messages_client_report_v1(client,plan,outcome,sent);
 if(outcome==PR_MESSAGES_NOTIFY_QUEUED){notifyResult=result;ackFinished=true;trace("actual-notify-ack",result);}
 if(outcome==PR_MESSAGES_CONNECTED&&stopped.load()){lateConnected=result;trace("late-CONNECTED",result);}
 if(outcome==PR_MESSAGES_SENT&&stopped.load()){lateSent=result;trace("late-SENT",result);}
 if(outcome==PR_MESSAGES_CLOSED){edge(result==PR_MESSAGES_OK,"real CLOSED accepted");++closeReports;closedSequence=tick();}
 return result;
}
extern "C" int32_t __real_pr_messages_client_deliver_v1(PrMessagesClient*,uint64_t);
extern "C" int32_t __wrap_pr_messages_client_deliver_v1(PrMessagesClient *client,uint64_t token){
 guiOnly();const auto result=__real_pr_messages_client_deliver_v1(client,token);++deliverCalls;deliverResult=result;trace("actual-GUI-deliver",result);return result;
}
extern "C" int32_t __real_pr_messages_client_stop_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_stop_v1(PrMessagesClient *client){
 guiOnly();const auto result=__real_pr_messages_client_stop_v1(client);edge(result==PR_MESSAGES_OK,"real UI Stop accepted");stopped=true;++stopCalls;trace("actual-Stop",result);
 // Test-only order injection: deliver the already-real queued lambda after real
 // Stop but before native destruction continues. Production never pumps here.
 if(mode==Case::StopNotify){QCoreApplication::processEvents(QEventLoop::AllEvents,2);edge(deliverCalls.load()==1&&deliverResult.load()==PR_MESSAGES_REFUSED,"actual queued delivery refused after Stop");}
 return result;
}
extern "C" int32_t __real_pr_messages_client_worker_exited_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_worker_exited_v1(PrMessagesClient *client){
 workerOnly();edge(nativeCloses.load()==1&&closeReports.load()==1&&ownedSocket.load()==-1,"worker exit after native close and CLOSED");const auto result=__real_pr_messages_client_worker_exited_v1(client);edge(result==PR_MESSAGES_OK,"actual worker exit accepted");edge(++workerExits==1,"one actual worker exit");exitSequence=tick();trace("actual-worker-exited",result);return result;
}
extern "C" bool __real__ZN7QThread4waitE14QDeadlineTimer(QThread*,QDeadlineTimer);
extern "C" bool __wrap__ZN7QThread4waitE14QDeadlineTimer(QThread *thread,QDeadlineTimer deadline){
 guiOnly();edge(stopped.load(),"actual Qt wait after Stop");++nativeWaits;if(holdActive.load()&&!waitReleaseSent.exchange(true)){sendRecord(gates,"WAIT_ENTER");}
 const bool completed=__real__ZN7QThread4waitE14QDeadlineTimer(thread,deadline);lastWait=completed?1:0;waitConsumed=false;
 if(completed){edge(thread->isFinished(),"actual Qt wait includes thread completion");++trueWaits;waitSequence=tick();}trace("actual-QThread-wait",completed?1:0);return completed;
}
extern "C" int32_t __real_pr_messages_client_join_result_v1(PrMessagesClient*,uint32_t);
extern "C" int32_t __wrap_pr_messages_client_join_result_v1(PrMessagesClient *client,uint32_t actual){
 guiOnly();edge(actual<=1&&!waitConsumed.exchange(true)&&int(actual)==lastWait.load(),"join verdict uses one exact actual Qt wait return");return __real_pr_messages_client_join_result_v1(client,actual);
}
extern "C" int32_t __real_pr_messages_client_free_v1(PrMessagesClient*);
extern "C" int32_t __wrap_pr_messages_client_free_v1(PrMessagesClient *client){
 guiOnly();edge(trueWaits.load()==1&&workerExits.load()==1,"free requires actual native wait and exit");const auto result=__real_pr_messages_client_free_v1(client);edge(result==PR_MESSAGES_OK,"actual joined core free");++frees;freeSequence=tick();return result;
}
namespace {
std::vector<QByteArray> inbox;
QByteArray waitFor(const QByteArray &prefix,bool pump=false){
 while(boot()<cutoff){for(auto i=inbox.begin();i!=inbox.end();++i){if(i->startsWith(prefix)){const auto value=*i;inbox.erase(i);return value;}}
  if(pump){QCoreApplication::processEvents(QEventLoop::AllEvents,2);}
  pollfd p{commands,POLLIN,0};const auto result=poll(&p,1,remainingMs(cutoff));need(result>=0||errno==EINTR,"GUI control poll");if(result>0&&(p.revents&(POLLIN|POLLHUP))){const auto b=readRecord(commands);if(!b.isEmpty()){inbox.push_back(b);}}}
 throw std::runtime_error("original Qt work cutoff");
}
void pumpUntil(const std::function<bool()> &predicate){while(!predicate()&&boot()<cutoff){QCoreApplication::processEvents(QEventLoop::AllEvents,2);QThread::msleep(1);}need(predicate(),"bounded actual Qt event completion");}
QJsonObject query(const QByteArray &record){
 const auto space=record.indexOf(' ',6);need(record.startsWith("QUERY ")&&space>6,"actual peer query record");const auto root=QJsonDocument::fromJson(record.mid(space+1)).object();need(root["op"]=="messages_read","actual messages_read operation");const auto request=root["request"].toObject();need(request["ttl_ms"].toInt()==1000,"original service request ttl");return request["query"].toObject();
}
int runChild(int argc,char **argv,const QString &path,Case chosen,int cmd,int gate){
 QCoreApplication application(argc,argv);guiThread=QThread::currentThread();mode=chosen;commands=cmd;gates=gate;cutoff=boot()+8*Second;
 const auto raw=path.toLocal8Bit();edge(raw.size()<int(sizeof(expectedPath)),"bounded fixture path");std::memcpy(expectedPath,raw.constData(),size_t(raw.size())+1);
 int changes=0;auto client=std::make_unique<MessagesClient>(path);
 QObject::connect(client.get(),&MessagesClient::changed,&application,[&]{guiOnly();++changes;},Qt::DirectConnection);
 if(mode==Case::Connect){waitFor("HELD connect");}
 else if(mode==Case::Send){waitFor("HELD send");const auto first=query(waitFor("QUERY "));need(first["query"]=="conversations"&&first["after"].isNull()&&first["limit"].toInt()==8,"initial actual conversation request");}
 else {
  const auto first=query(waitFor("QUERY "));need(first["query"]=="conversations"&&first["after"].isNull()&&first["limit"].toInt()==8,"initial actual conversation request");
  sendRecord(commands,R"(REPLY {"page":"conversations","items":[{"id":"11111111111111111111111111111111","sender":"Alice","slot":0}]})");waitFor("HELD notify");
  if(mode==Case::Notify||mode==Case::Stale){
   if(mode==Case::Stale){client->select(QString(32,'2'));need(changes==1,"one actual public action notification");}
   pumpUntil([]{return deliverCalls.load()==1;});
   need(notifyResult.load()==-99&&holdActive.load(),"actual delivery precedes ACK call forwarding");
   if(mode==Case::Notify){need(deliverResult.load()==PR_MESSAGES_OK&&changes==1&&client->conversations().size()==1,"one actual valid queued publication");}
   else {need(deliverResult.load()==PR_MESSAGES_STALE&&changes==1&&client->conversations().isEmpty(),"actual stale queued event cannot publish");}
   uint64_t wake=0;{std::lock_guard<std::mutex> lock(stateMutex);wake=notifyPlan.wake_boottime_ns;}
   while(boot()<=wake&&boot()<cutoff){QCoreApplication::processEvents(QEventLoop::AllEvents,2);QThread::msleep(1);}need(boot()>wake,"original cooldown passed while ACK held");
   sendRecord(commands,"RELEASE");const auto second=query(waitFor("QUERY ",true));need(second["query"]=="messages"&&second["conversation"]==QString(32,mode==Case::Stale?'2':'1')&&second["after_ordinal"].toInteger()==0&&second["limit"].toInt()==2,"next actual request uses selected generation");
   need(notifyResult.load()==PR_MESSAGES_OK&&notifyPlans.load()==1&&notifyAcks.load()==1&&deliverCalls.load()==1,"one queue/ACK/delivery after FIFO removal");
  }
 }
 const int changesBeforeStop=changes;const auto beginStop=boot();client.reset();const auto endStop=boot();
 // Real QObject context cancellation is observed after destruction as well.
 QCoreApplication::processEvents(QEventLoop::AllEvents,2);need(changes==changesBeforeStop,"no queued publication after Stop/destruction");
 waitFor("EOF");need(nativeCloses.load()==1&&ebadfSeen.load()&&closeReports.load()==1&&workerExits.load()==1&&trueWaits.load()==1&&frees.load()==1,"actual descriptor/thread/core ownership closure");
 need(closeSequence.load()<closedSequence.load()&&closedSequence.load()<exitSequence.load()&&exitSequence.load()<waitSequence.load()&&waitSequence.load()<freeSequence.load(),"actual close/CLOSED/exit/nativewait/free order");
 if(mode==Case::Connect){need(lateConnected.load()==PR_MESSAGES_REFUSED&&nativeSends.load()==0&&nativeReceives.load()==0&&deliverCalls.load()==0,"late real CONNECTED refuses without request");}
 if(mode==Case::Send){need(lateSent.load()==PR_MESSAGES_REFUSED&&nativeSends.load()==1&&nativeReceives.load()==0&&deliverCalls.load()==0,"late real SENT refuses without successor receive/replay");}
 if(mode==Case::StopNotify){need(notifyResult.load()==PR_MESSAGES_REFUSED&&deliverCalls.load()==1&&deliverResult.load()==PR_MESSAGES_REFUSED&&changes==0,"unacknowledged notification cannot block actual Stop or publish");}
 QJsonObject result{{"case",QString::fromLocal8Bit(argv[1])},{"actual_qt_wait",true},{"actual_finished",true},{"actual_socket_close",true},{"actual_ebadf",true},{"actual_peer_eof",true},{"actual_core_free",true},{"stop_elapsed_ns",qint64(endStop-beginStop)},{"native_wait_calls",nativeWaits.load()},{"actual_sends",nativeSends.load()},{"actual_receives",nativeReceives.load()},{"gui_deliveries",deliverCalls.load()},{"gui_changes",changes}};
 const auto bytes=QJsonDocument(result).toJson(QJsonDocument::Compact);std::printf("%s\n",bytes.constData());std::fflush(stdout);sendRecord(commands,"FINISHED");edge(::close(commands)==0&&::close(gates)==0,"actual child control fd close");return 0;
}
QByteArray applicationRecord(int fd,pid_t child){
 char bytes[4096];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(sizeof(int)*4)]{};iovec vector{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&vector;message.msg_iovlen=1;message.msg_control=control;message.msg_controllen=sizeof(control);
 const auto n=recvmsg(fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);if(n<0&&(errno==EAGAIN||errno==EINTR||errno==EWOULDBLOCK)){return {};}
 if(n==0){return QByteArray("#EOF");}need(n>0,"actual root peer receive");bool credential=false,invalid=(message.msg_flags&(MSG_TRUNC|MSG_CTRUNC))!=0;
 for(auto *c=CMSG_FIRSTHDR(&message);c;c=CMSG_NXTHDR(&message,c)){if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS&&c->cmsg_len==CMSG_LEN(sizeof(ucred))&&!credential){ucred actual{};std::memcpy(&actual,CMSG_DATA(c),sizeof(actual));credential=actual.pid==child&&actual.uid==0;invalid=invalid||!credential;}else{invalid=true;if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_RIGHTS&&c->cmsg_len>=CMSG_LEN(0)){const auto count=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);for(size_t i=0;i<count;++i){int delivered=-1;std::memcpy(&delivered,static_cast<const char*>(static_cast<const void*>(CMSG_DATA(c)))+i*sizeof(int),sizeof(int));::close(delivered);}}}}
 need(credential&&!invalid,"root peer exact actual client SCM credential");return {bytes,int(n)};
}
struct ParentDescriptors {
 std::vector<int> values;
 void add(int fd){need(fd>=0,"owned parent fd");values.push_back(fd);}
 void forget(int fd){const auto i=std::find(values.begin(),values.end(),fd);need(i!=values.end(),"exact owned fd transfer");values.erase(i);}
 void close(int fd){forget(fd);need(::close(fd)==0,"actual owned parent fd close");}
 void closeAll(){while(!values.empty()){close(values.back());}}
 ~ParentDescriptors(){for(const int fd:values){::close(fd);}}
};
struct ChildOwner {
 pid_t pid=-1;int output=-1;bool eof=false,waited=false,withinCap=true;int status=0;std::string bytes;uint64_t outer=0;
 void drain(){
  if(output<0||eof){return;}
  char part[1024];
  for(;;){
   const auto n=read(output,part,sizeof(part));
   if(n>0){withinCap=bytes.size()+size_t(n)<=OutputCap;need(withinCap,"bounded child stdout cap");bytes.append(part,size_t(n));continue;}
   if(n==0){eof=true;::close(output);output=-1;return;}
   if(errno==EINTR){continue;}
   need(errno==EAGAIN||errno==EWOULDBLOCK,"nonblocking stdout drain");return;
  }
 }
 bool finish(){
  const auto until=std::min(outer,boot()+2*Second);
  while(!eof&&boot()<until){drain();if(!eof){pollfd p{output,POLLIN,0};poll(&p,1,remainingMs(until));}}
  if(!eof){return false;}
  while(boot()<until){const auto r=waitpid(pid,&status,WNOHANG);if(r==pid){waited=true;pid=-1;return true;}need(r==0||(r<0&&errno==EINTR),"exact Qt child wait");poll(nullptr,0,1);}
  return false;
 }
 ~ChildOwner(){
  if(pid>0){
   const int signalled=kill(pid,SIGKILL);const auto until=std::min(outer,boot()+2*Second);bool cap=withinCap;
   // Failure closure has the same actual EOF-before-wait rule; never assert
   // a wait fact merely because kill/wait was attempted. No blocking wait.
   while(!eof&&output>=0&&boot()<until){
    char part[1024];const auto n=read(output,part,sizeof(part));
    if(n>0){const auto available=OutputCap-bytes.size();const auto keep=std::min(available,size_t(n));bytes.append(part,keep);cap=cap&&keep==size_t(n);continue;}
    if(n==0){eof=true;::close(output);output=-1;break;}
    if(errno==EINTR){continue;}
    if(errno!=EAGAIN&&errno!=EWOULDBLOCK){break;}
    pollfd p{output,POLLIN,0};poll(&p,1,remainingMs(until));
   }
   while(eof&&boot()<until){const auto r=waitpid(pid,&status,WNOHANG);if(r==pid){waited=true;pid=-1;break;}if(r<0&&errno!=EINTR){break;}poll(nullptr,0,1);}
   std::fprintf(stderr,"ORDER_FAILED_CLEANUP kill_result=%d stdout_actual_eof=%d stdout_within_cap=%d actual_child_wait=%d raw_wait_status=%d\n",signalled,eof,cap,waited,status);
  }
  if(output>=0){::close(output);}
 }
};
int parentRun(int argc,char **argv,Case chosen){
 ParentDescriptors descriptors;QTemporaryDir directory;need(directory.isValid(),"private root peer directory");const QString path=directory.path()+"/service.sock";const auto raw=path.toLocal8Bit();const auto start=boot();cutoff=start+20*Second;const auto workUntil=start+8*Second;
 const int listener=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);need(listener>=0,"actual root listener");descriptors.add(listener);int listenerPasscred=1;need(setsockopt(listener,SOL_SOCKET,SO_PASSCRED,&listenerPasscred,sizeof(listenerPasscred))==0,"PASSCRED before requests can queue");sockaddr_un address{};address.sun_family=AF_UNIX;need(raw.size()<int(sizeof(address.sun_path)),"actual socket path bound");std::memcpy(address.sun_path,raw.constData(),size_t(raw.size())+1);need(bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(listener,2)==0,"actual root bind/listen");
 int cmd[2],gate[2],output[2];
 need(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0,cmd)==0,"owned GUI control pair");descriptors.add(cmd[0]);descriptors.add(cmd[1]);
 need(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0,gate)==0,"owned worker gate pair");descriptors.add(gate[0]);descriptors.add(gate[1]);
 need(pipe2(output,O_NONBLOCK|O_CLOEXEC)==0,"owned stdout PIPE");descriptors.add(output[0]);descriptors.add(output[1]);
 const pid_t serverPid=getpid();
 const pid_t child=fork();need(child>=0,"owned Qt child fork");
 if(child==0){::close(listener);::close(cmd[0]);::close(gate[0]);::close(output[0]);edge(prctl(PR_SET_PDEATHSIG,SIGKILL)==0&&getppid()==serverPid,"owned child parent liveness");rlimit noCore{0,0};edge(setrlimit(RLIMIT_CORE,&noCore)==0,"disable disposable core dump");edge(dup2(output[1],STDOUT_FILENO)==STDOUT_FILENO,"actual child stdout PIPE");::close(output[1]);int result=101;try{result=runChild(argc,argv,path,chosen,cmd[1],gate[1]);}catch(const std::exception &error){std::fprintf(stderr,"ORDER_CHILD_FAILURE %s\n",error.what());}std::fflush(nullptr);_exit(result);}
 descriptors.close(cmd[1]);descriptors.close(gate[1]);descriptors.close(output[1]);descriptors.forget(output[0]);ChildOwner owner;owner.pid=child;owner.output=output[0];owner.outer=cutoff;
 int peer=-1,accepts=0,queries=0,eofs=0;bool cmdEof=false,gateEof=false,finished=false;QByteArray held;bool heldAnnounced=false;bool normal=false;
 auto release=[&]{need(!held.isEmpty()&&heldAnnounced,"release exact held actual call");sendRecord(gate[0],QByteArray("RELEASE ")+held);held.clear();heldAnnounced=false;};
 try{
  while(boot()<workUntil){
   pollfd f[5]{{listener,POLLIN,0},{peer,POLLIN,0},{cmdEof?-1:cmd[0],POLLIN,0},{gateEof?-1:gate[0],POLLIN,0},{owner.output,POLLIN,0}};
   const auto polled=poll(f,5,remainingMs(cutoff));need(polled>=0||errno==EINTR,"root peer original-bound poll");
   if(f[0].revents&POLLIN){need(peer<0&&++accepts==1,"one actual root peer accept");peer=accept4(listener,nullptr,nullptr,SOCK_NONBLOCK|SOCK_CLOEXEC);need(peer>=0,"actual root accept");descriptors.add(peer);int one=1;need(setsockopt(peer,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one))==0,"root peer PASSCRED");ucred actual{};socklen_t size=sizeof(actual);need(getsockopt(peer,SOL_SOCKET,SO_PEERCRED,&actual,&size)==0&&size==sizeof(actual)&&actual.pid==child&&actual.uid==0,"actual root/Qt child identity");std::fprintf(stderr,"ACTUAL_ROOT_PEER server_pid=%d client_pid=%d uid=%u\n",getpid(),actual.pid,actual.uid);}
   if(peer>=0&&(f[1].revents&(POLLIN|POLLHUP))){const auto record=applicationRecord(peer,child);if(record=="#EOF"){++eofs;descriptors.close(peer);peer=-1;if(!cmdEof){sendRecord(cmd[0],"EOF");}}else if(!record.isEmpty()){++queries;need(queries<=2,"no successor request/replay");sendRecord(cmd[0],QByteArray("QUERY ")+QByteArray::number(qulonglong(boot()))+' '+record);}}
   if(!gateEof&&(f[3].revents&(POLLIN|POLLHUP))){char bytes[128];const auto n=recv(gate[0],bytes,sizeof(bytes),MSG_DONTWAIT);if(n==0){gateEof=true;}else if(n>0){const QByteArray record(bytes,int(n));if(record.startsWith("HOLD ")){need(held.isEmpty(),"one held actual call at a time");held=record.mid(5);}else if(record=="WAIT_ENTER"){need(chosen==Case::Connect||chosen==Case::Send||chosen==Case::StopNotify,"actual wait releases only Stop case");release();}else{throw std::runtime_error("unexpected gate record");}}else{need(errno==EAGAIN||errno==EWOULDBLOCK||errno==EINTR,"actual gate receive");}}
   if(!held.isEmpty()&&!heldAnnounced&&accepts==1&&(held!="send"||queries>=1)){sendRecord(cmd[0],QByteArray("HELD ")+held);heldAnnounced=true;}
   if(!cmdEof&&(f[2].revents&(POLLIN|POLLHUP))){char bytes[8192];const auto n=recv(cmd[0],bytes,sizeof(bytes),MSG_DONTWAIT);if(n==0){cmdEof=true;}else if(n>0){const QByteArray record(bytes,int(n));if(record.startsWith("REPLY ")){need(peer>=0&&queries==1,"reply only to real first request");sendRecord(peer,record.mid(6));}else if(record=="RELEASE"){need(chosen==Case::Notify||chosen==Case::Stale,"manual release only notification order");release();}else if(record=="FINISHED"){finished=true;}else{throw std::runtime_error("unexpected GUI control record");}}else{need(errno==EAGAIN||errno==EWOULDBLOCK||errno==EINTR,"actual GUI control receive");}}
   owner.drain();if(owner.eof&&cmdEof&&gateEof){normal=finished&&accepts==1&&eofs==1;break;}
  }
  need(normal,"actual peer and control EOF closure");need(owner.finish(),"stdout actual EOF before bounded actual child wait");need(WIFEXITED(owner.status)&&WEXITSTATUS(owner.status)==0,"actual Qt child wait0");
  const int expectedQueries=chosen==Case::Connect?0:(chosen==Case::Send||chosen==Case::StopNotify?1:2);need(queries==expectedQueries,"exact actual peer request count");descriptors.closeAll();
  std::fwrite(owner.bytes.data(),1,owner.bytes.size(),stdout);std::printf("{\"case\":\"%s\",\"actual_root_peer\":true,\"actual_child_wait\":0,\"stdout_actual_eof\":true,\"stdout_bytes\":%zu,\"parent_fds_closed\":true,\"peer_eof_count\":%d,\"actual_queries\":%d}\n",argv[1],owner.bytes.size(),eofs,queries);
 }catch(...){throw;}
 return 0;
}
}
int main(int argc,char **argv){
 if(argc!=2||getuid()!=0||geteuid()!=0){return 64;}const QByteArray name(argv[1]);Case chosen;
 if(name=="notify-deliver-before-ack"){chosen=Case::Notify;}else if(name=="stale-deliver-before-ack"){chosen=Case::Stale;}else if(name=="stop-connect-entered"){chosen=Case::Connect;}else if(name=="stop-send-entered"){chosen=Case::Send;}else if(name=="stop-notify-unacked"){chosen=Case::StopNotify;}else{return 64;}
 try{return parentRun(argc,argv,chosen);}catch(const std::exception &error){std::fprintf(stderr,"ORDER_PARENT_FAILURE %s\n",error.what());return 101;}
}
