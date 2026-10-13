// Actual production Qt client thread + Linux SOCK_SEQPACKET peer credentials.
// Linker injections: caller clock failure, actual Qt wait report, and post-real-recv delay.
#include "MessagesClient.h"
#include <QCoreApplication>
#include <QDeadlineTimer>
#include <QTemporaryDir>
#include <QThread>
#include <QJsonDocument>
#include <QJsonObject>
#include <QEventLoop>
#include <atomic>
#include <memory>
#include <cstdio>
#include <cstring>
#include <cerrno>
#include <chrono>
#include <thread>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>
std::atomic<bool> failClock{false}, rejectWait{false}, clockAfterReceive{false}, delayAfterReceive{false};
std::atomic<int> actualReceives{0};
extern "C" int __real_clock_gettime(clockid_t,timespec*);
extern "C" int __wrap_clock_gettime(clockid_t id,timespec* t) {
 if(id==CLOCK_BOOTTIME && failClock.load()){errno=EIO;return -1;}
 return __real_clock_gettime(id,t);
}
extern "C" bool __real__ZN7QThread4waitE14QDeadlineTimer(QThread*,QDeadlineTimer);
extern "C" bool __wrap__ZN7QThread4waitE14QDeadlineTimer(QThread* thread,QDeadlineTimer deadline) {
 const bool completed=__real__ZN7QThread4waitE14QDeadlineTimer(thread,deadline);
 if(completed && rejectWait.load()){
  static bool reported=false;
  if(!reported){std::fprintf(stderr,"INJECTED_WAIT_REPORT: actual_wait=1 actual_finished=%d; return false\n",thread->isFinished());std::fflush(stderr);reported=true;}
  return false;
 }
 return completed;
}
extern "C" ssize_t __real_recvmsg(int,msghdr*,int);
extern "C" ssize_t __wrap_recvmsg(int fd,msghdr* msg,int flags){
 const auto n=__real_recvmsg(fd,msg,flags);
 if(n>0){
  const auto count=++actualReceives;
  std::fprintf(stderr,"ACTUAL_RECEIVE: bytes=%zd ordinal=%d\n",n,count);std::fflush(stderr);
  if(clockAfterReceive.load())failClock=true;
  if(delayAfterReceive.load()&&count==1)QThread::msleep(1100);
 }
 return n;
}
static qint64 millis(){timespec t{};if(__real_clock_gettime(CLOCK_BOOTTIME,&t))return -1;return qint64(t.tv_sec)*1000+t.tv_nsec/1000000;}
static int report(QJsonObject value,int code){const auto b=QJsonDocument(value).toJson(QJsonDocument::Compact);std::printf("%s\n",b.constData());std::fflush(stdout);return code;}
static bool ready(int fd,int events,int wait){pollfd p{fd,short(events),0};return ::poll(&p,1,wait)>0&&(p.revents&events);}
static void events(int duration){const auto until=millis()+duration;while(millis()<until){QCoreApplication::processEvents(QEventLoop::AllEvents,2);QThread::msleep(1);}}
int main(int argc,char**argv){
 QCoreApplication app(argc,argv);if(argc!=2||getuid()!=0)return 64;
 const QString mode=QString::fromLocal8Bit(argv[1]);QTemporaryDir dir;if(!dir.isValid())return 70;
 const auto path=dir.path()+"/service.sock";const auto encoded=path.toLocal8Bit();
 const int listener=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);if(listener<0)return 70;
 sockaddr_un address{};address.sun_family=AF_UNIX;if(encoded.size()>=int(sizeof(address.sun_path)))return 70;
 std::memcpy(address.sun_path,encoded.constData(),size_t(encoded.size())+1);
 if(::bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))||::listen(listener,2))return 70;
 const bool admission=mode=="clock-admission";failClock=admission;
 auto client=std::make_unique<MessagesClient>(path);
 if(!ready(listener,POLLIN,1000))return 70;
 const int peer=::accept4(listener,nullptr,nullptr,SOCK_NONBLOCK|SOCK_CLOEXEC);if(peer<0)return 70;
 ucred actual{};socklen_t length=sizeof(actual);if(getsockopt(peer,SOL_SOCKET,SO_PEERCRED,&actual,&length)||actual.pid!=getpid()||actual.uid!=0)return 70;
 char request[4096]{};ssize_t n=0;
 if(ready(peer,POLLIN,admission?100:1000))n=::recv(peer,request,sizeof(request),MSG_DONTWAIT);
 const bool queried=n>0&&QJsonDocument::fromJson(QByteArray(request,int(n))).object()["op"].toString()=="messages_read";
 std::fprintf(stderr,"ACTUAL_SOCKET: client_pid=%d peer_uid=%d request_seen=%d mode=%s\n",actual.pid,actual.uid,queried,qPrintable(mode));std::fflush(stderr);
 if(!admission&&!queried)return 70;
 if(admission){failClock=false;const auto start=millis();client.reset();const auto elapsed=millis()-start;return report({{"mode",mode},{"query_seen",queried},{"owner_destroyed",true},{"elapsed_boottime_ms",elapsed}},!queried&&elapsed<500?0:101);}
 if(mode=="held-close"||mode=="held-clock-close"||mode=="wait-report-failure"){
  failClock=mode=="held-clock-close";rejectWait=mode=="wait-report-failure";
  const auto start=millis();client.reset();const auto elapsed=millis()-start;
  return report({{"mode",mode},{"owner_destroyed",true},{"elapsed_boottime_ms",elapsed}},elapsed<200&&mode!="wait-report-failure"?0:101);
 }
 const QByteArray reply=R"({"page":"conversations","items":[{"id":"11111111111111111111111111111111","sender":"synthetic sender","slot":0}]})";
 if(mode=="clock-completion")clockAfterReceive=true;
 else if(mode=="late-completion")delayAfterReceive=true;
 else if(mode!="within-budget")return 64;
 const auto sent=::send(peer,reply.constData(),size_t(reply.size()),MSG_NOSIGNAL|MSG_DONTWAIT);
 events(mode=="late-completion"?1300:100);
 const bool published=!client->conversations().isEmpty();failClock=false;client.reset();
 const bool expected=mode=="within-budget";
 return report({{"mode",mode},{"reply_sent",sent==reply.size()},{"actual_receive_count",actualReceives.load()},{"history_published",published},{"owner_destroyed",true}},published==expected&&sent==reply.size()&&actualReceives.load()==1?0:101);
}
