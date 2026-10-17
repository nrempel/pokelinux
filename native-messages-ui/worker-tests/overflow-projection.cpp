// Authored preimplementation controls against unchanged public MessagesClient.
// Actual root child SEQPACKET peer; copied synthetic local-history payloads.
// No private-member access, parser reimplementation, actor/GUI or hardware claim.
#include "MessagesClient.h"
#include <QCoreApplication>
#include <QEventLoop>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryDir>
#include <QThread>
#include <atomic>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
#include <poll.h>
#include <signal.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
static quint64 boot(){timespec t{};if(clock_gettime(CLOCK_BOOTTIME,&t))std::abort();return quint64(t.tv_sec)*1000000000+quint64(t.tv_nsec);}
static void need(bool ok,const char* label){if(!ok){std::fprintf(stderr,"CONTROL_FAILURE %s\n",label);throw std::runtime_error(label);}}
static bool readable(int fd,int ms){pollfd p{fd,POLLIN,0};return poll(&p,1,ms)>0&&(p.revents&POLLIN);}
static QByteArray receive(int fd,int ms){need(readable(fd,ms),"bounded record ready");char bytes[8192];const auto n=recv(fd,bytes,sizeof(bytes),MSG_DONTWAIT);need(n>0,"record received");return {bytes,int(n)};}
static void sendRecord(int fd,const QByteArray& data){need(send(fd,data.constData(),size_t(data.size()),MSG_NOSIGNAL)==data.size(),"whole record sent");}
static void events(unsigned ms){const auto until=boot()+quint64(ms)*1000000;while(boot()<until){QCoreApplication::processEvents(QEventLoop::AllEvents,2);QThread::msleep(1);}}
static const QString A(32,'1'),B(32,'2'),C(32,'3');
static const QString History="Local receipt history · modem receive and send unavailable";
static QByteArray conversations(){return QJsonDocument(QJsonObject{{"page","conversations"},{"items",QJsonArray{QJsonObject{{"id",A},{"sender","Alice"},{"slot",0}},QJsonObject{{"id",B},{"sender","Bob"},{"slot",7}}}}}).toJson(QJsonDocument::Compact);}
static QByteArray messages(){return QJsonDocument(QJsonObject{{"page","messages"},{"items",QJsonArray{QJsonObject{{"receipt",C},{"sender","Alice"},{"text","<b>literal</b>"},{"ordinal",1}},QJsonObject{{"receipt",B},{"sender","Bob"},{"text","two"},{"ordinal",4}}}}}).toJson(QJsonDocument::Compact);}
struct Fixture {
 QTemporaryDir dir;int control=-1;pid_t pid=-1;std::unique_ptr<MessagesClient> client;int changes=0;quint64 originalUntil=0;
 Fixture(){
  need(dir.isValid(),"private directory");const auto path=dir.path()+"/service.sock";const auto raw=path.toLocal8Bit();int c[2];need(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC,0,c)==0,"control pair");originalUntil=boot()+20000000000ULL;
  pid=fork();need(pid>=0,"root peer fork");
  if(pid==0){
   close(c[0]);if(prctl(PR_SET_PDEATHSIG,SIGKILL)||geteuid()!=0)_exit(71);
   const int listenFd=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);if(listenFd<0)_exit(71);sockaddr_un a{};a.sun_family=AF_UNIX;if(raw.size()>=int(sizeof(a.sun_path)))_exit(71);std::memcpy(a.sun_path,raw.constData(),size_t(raw.size())+1);if(bind(listenFd,reinterpret_cast<sockaddr*>(&a),sizeof(a))||listen(listenFd,2))_exit(71);
   if(send(c[1],"READY",5,MSG_NOSIGNAL)!=5)_exit(71);
   int peer=-1;bool waiting=false;bool stop=false;
   while(!stop&&boot()<originalUntil){
    pollfd f[3]{{c[1],POLLIN,0},{listenFd,POLLIN,0},{peer,POLLIN,0}};if(poll(f,3,10)<0&&errno!=EINTR)_exit(72);
    if(f[0].revents&POLLIN){char b[8192];auto n=recv(c[1],b,sizeof(b),0);if(n<=0)_exit(72);QByteArray value(b,int(n));if(value=="#STOP"){stop=true;continue;}if(!waiting||peer<0)_exit(73);if(send(peer,value.constData(),size_t(value.size()),MSG_NOSIGNAL|MSG_DONTWAIT)!=value.size())_exit(74);waiting=false;}
    if(peer<0&&(f[1].revents&POLLIN)){peer=accept4(listenFd,nullptr,nullptr,SOCK_CLOEXEC|SOCK_NONBLOCK);if(peer<0)_exit(74);ucred actual{};socklen_t size=sizeof(actual);if(getsockopt(peer,SOL_SOCKET,SO_PEERCRED,&actual,&size)||actual.uid!=0||actual.pid!=getppid())_exit(74);std::fprintf(stderr,"ACTUAL_ROOT_PEER pid=%d client_pid=%d client_uid=%u\n",getpid(),actual.pid,actual.uid);}
    if(!waiting&&peer>=0&&(f[2].revents&POLLIN)){char bytes[4096];const auto n=recv(peer,bytes,sizeof(bytes),MSG_DONTWAIT);if(n==0){close(peer);peer=-1;continue;}if(n<0){if(errno==EAGAIN||errno==EINTR)continue;_exit(74);}if(send(c[1],bytes,size_t(n),MSG_NOSIGNAL)!=n)_exit(74);waiting=true;}
   }
   if(peer>=0){close(peer);}
   close(listenFd);close(c[1]);_exit(stop?0:75);
  }
  close(c[1]);control=c[0];need(receive(control,1000)=="READY","actual peer ready");client=std::make_unique<MessagesClient>(path);QObject::connect(client.get(),&MessagesClient::changed,[this]{++changes;});
 }
 QJsonObject request(const char* kind=nullptr){const auto bytes=receive(control,2000);std::printf("ACTUAL_QUERY_JSON %s\n",bytes.constData());const auto root=QJsonDocument::fromJson(bytes).object();need(root["op"]=="messages_read","real messages_read request");const auto req=root["request"].toObject();need(req["ttl_ms"].toInt()==1000,"original ttl1000");auto q=req["query"].toObject();if(kind)need(q["query"]==QString::fromLatin1(kind),"selected query kind");need(q["limit"].toInt()==(q["query"]=="conversations"?8:2),"bounded page limit");return q;}
 void reply(const QByteArray& bytes,bool pump=true){const auto n=changes;sendRecord(control,bytes);if(pump){const auto until=boot()+180000000;while(changes==n&&boot()<until)events(1);need(changes>n,"actual queued publication observed");}}
 void history(){auto q=request("conversations");need(q["after"].isNull(),"initial null cursor");reply(conversations());need(client->conversations().size()==2,"conversations published");q=request("messages");need(q["conversation"]==A&&q["after_ordinal"].toInteger()==0,"automatic first selection");reply(messages());need(client->messages().size()==2,"messages published");}
 void finish(){if(!client)return;client.reset();sendRecord(control,"#STOP");const auto until=boot()+2000000000;int status=0;bool waited=false;while(boot()<until){auto r=waitpid(pid,&status,WNOHANG);if(r==pid){waited=true;break;}need(r==0,"waitpid root peer");usleep(1000);}if(!waited){kill(pid,SIGKILL);need(waitpid(pid,&status,0)==pid,"emergency root peer wait");}std::printf("ACTUAL_ROOT_PEER_WAIT pid=%d wait=%d exit=%d original_until_ns=%llu observed_ns=%llu\n",pid,waited,WIFEXITED(status)?WEXITSTATUS(status):-1,static_cast<unsigned long long>(originalUntil),static_cast<unsigned long long>(boot()));close(control);control=-1;need(waited&&WIFEXITED(status)&&WEXITSTATUS(status)==0,"normal bounded peer exit");}
 ~Fixture(){if(client){try{finish();}catch(...){std::abort();}}}
};

static const QByteArray FinitePacket=R"({"page":"conversations","items":[{"id":"11111111111111111111111111111111","sender":"A","slot":0,"unknown":{"kept":1e2}}]})";
static const QByteArray OverflowPacket=R"({"page":"conversations","items":[{"id":"11111111111111111111111111111111","sender":"A","slot":0,"unknown":{"kept":1e400}}]})";
static quint64 workUntil=0;
static void probe(const char* name,const QByteArray& packet,bool positive){
 need(boot()<workUntil,"original whole probe work cutoff");
 QJsonParseError parseError{};const auto directDocument=QJsonDocument::fromJson(packet,&parseError);
 Fixture f;f.request("conversations");f.reply(packet);
 const auto status=f.client->status();const auto cs=f.client->conversations();const auto ms=f.client->messages();
 const bool expected=positive?(status==History&&cs.size()==1&&cs.first().toMap()["unknown"].toMap()["kept"].toDouble()==100.0):(status=="Messages unavailable"&&cs.isEmpty()&&ms.isEmpty());
 const auto observed=boot();
 QJsonObject result{{"case",name},{"packet_utf8_hex",QString::fromLatin1(packet.toHex())},{"actual_status",status},{"actual_conversations",QJsonArray::fromVariantList(cs)},{"actual_messages",QJsonArray::fromVariantList(ms)},{"actual_Qt_direct_parse_error",int(parseError.error)},{"actual_Qt_direct_document_null",directDocument.isNull()},{"expected_satisfied",expected},{"original_work_until_ns",QString::number(workUntil)},{"observed_boottime_ns",QString::number(observed)}};
 const auto bytes=QJsonDocument(result).toJson(QJsonDocument::Compact);std::printf("%s\n",bytes.constData());std::fflush(stdout);
 need(expected,"actual whole-client packet projection disposition");need(observed<workUntil,"actual disposition before original work cutoff");
 f.finish();
}
int main(int argc,char** argv){
 std::setvbuf(stdout,nullptr,_IONBF,0);QCoreApplication app(argc,argv);if(argc!=1||geteuid()!=0)return 64;
 workUntil=boot()+5000000000ULL;
 try{
  probe("unknown-finite-control",FinitePacket,true);
  probe("unknown-overflow-unavailable",OverflowPacket,false);
  need(boot()<workUntil,"original completion cutoff");std::puts("PASS actual_Qt_client_unknown_numeric_overflow_is_unavailable finite_control_passed=1 groups=2");return 0;
 }catch(const std::exception& error){std::fprintf(stderr,"OVERFLOW_PROJECTION_FAILURE %s\n",error.what());return 101;}
}
