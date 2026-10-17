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
struct ParserCase{const char* name;QByteArray bytes;int expected;};
static std::vector<ParserCase> parserCases(){
 const auto a=A.toUtf8(),b=B.toUtf8();const QByteArray cprefix="{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\"A\",\"slot\":";
 const QByteArray mprefix="{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\"B\",\"text\":\"x\",\"ordinal\":";
 QJsonArray nine,three;const auto cr=QJsonDocument::fromJson(conversations()).object()["items"].toArray().first();const auto mr=QJsonDocument::fromJson(messages()).object()["items"].toArray().first();for(int i=0;i<9;++i)nine.append(cr);for(int i=0;i<3;++i)three.append(mr);
 return {
  {"conversations9",QJsonDocument(QJsonObject{{"page","conversations"},{"items",nine}}).toJson(QJsonDocument::Compact),0},
  {"messages3",QJsonDocument(QJsonObject{{"page","messages"},{"items",three}}).toJson(QJsonDocument::Compact),0},
  {"empty-sender","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\"\",\"slot\":0}]}",0},
  {"uppercase-id","{\"page\":\"conversations\",\"items\":[{\"id\":\""+QByteArray(32,'A')+"\",\"sender\":\"A\",\"slot\":0}]}",0},
  {"text-null","{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\"B\",\"text\":null,\"ordinal\":1}]}",0},
  {"valid-page-wins-denied",R"({"result":"denied","page":"conversations","items":[]})",1},
  {"valid-conversations",conversations(),1},{"valid-messages-response-to-conversation-request",messages(),1},
  {"empty-conversations",R"({"page":"conversations","items":[]})",1},
  {"empty-messages",R"({"page":"messages","items":[]})",1},
  {"denied",R"({"result":"denied"})",2},{"unavailable",R"({"result":"unavailable"})",0},
  {"array-root","[]",0},{"truncated-json","{",0},{"null-items",R"({"page":"conversations","items":null})",0},
  {"nonobject-row",R"({"page":"conversations","items":[true]})",0},
  {"slot7",cprefix+"7}]}",1},{"slot8",cprefix+"8}]}",0},{"slot-bool",cprefix+"true}]}",0},
  {"slot-integral-decimal",cprefix+"7.0}]}",1},{"slot-fraction",cprefix+"0.5}]}",0},
  {"ordinal-positive",mprefix+"1}]}",1},{"ordinal4096",mprefix+"4096}]}",1},{"ordinal4097",mprefix+"4097}]}",0},
  {"ordinal-zero",mprefix+"0}]}",0},{"ordinal-bool",mprefix+"true}]}",0},
  {"ordinal-integral-exponent",mprefix+"4e0}]}",1},{"ordinal-fraction",mprefix+"1.5}]}",0},
  {"last-duplicate-slot-valid",cprefix+"8,\"slot\":0}]}",1},{"last-duplicate-slot-invalid",cprefix+"0,\"slot\":8}]}",0},
  {"unknown-keys-preserved",cprefix+"0,\"extra\":{\"kept\":true}}]}",1},
  {"sender-byte64","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\""+QByteArray(64,'x')+"\",\"slot\":0}]}",1},
  {"sender-byte65","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\""+QByteArray(65,'x')+"\",\"slot\":0}]}",0},
  {"sender-multibyte64","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\""+QString(32,QChar(0xe9)).toUtf8()+"\",\"slot\":0}]}",1},
  {"sender-multibyte66","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\",\"sender\":\""+QString(33,QChar(0xe9)).toUtf8()+"\",\"slot\":0}]}",0},
  {"text-byte640","{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\"B\",\"text\":\""+QByteArray(640,'x')+"\",\"ordinal\":1}]}",1},
  {"text-byte641","{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\"B\",\"text\":\""+QByteArray(641,'x')+"\",\"ordinal\":1}]}",0},
  // Observation-only: this finite control must determine current Qt normalization.
  {"observe-unpaired-surrogate","{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\"\\ud800\",\"text\":\"x\",\"ordinal\":1}]}",-1},
  {"observe-id-terminal-newline","{\"page\":\"conversations\",\"items\":[{\"id\":\""+a+"\\n\",\"sender\":\"A\",\"slot\":0}]}",-1},
  {"observe-invalid-utf8","{\"page\":\"messages\",\"items\":[{\"receipt\":\""+b+"\",\"sender\":\""+QByteArray(1,char(0xff))+"\",\"text\":\"x\",\"ordinal\":1}]}",-1}
 };
}
static void parser(){for(const auto&t:parserCases()){Fixture f;f.request("conversations");f.reply(t.bytes);const auto s=f.client->status();const int actual=s==History?1:s=="Messages access denied"?2:0;if(t.expected>=0)need(actual==t.expected,t.name);if(actual!=1)need(f.client->conversations().isEmpty()&&f.client->messages().isEmpty(),"failure clears both lists");if(QByteArray(t.name)=="unknown-keys-preserved")need(f.client->conversations().first().toMap()["extra"].toMap()["kept"].toBool(),"unknown model field retained");QJsonObject result{{"case",t.name},{"expected",t.expected},{"observed",actual},{"status",s},{"conversations",QJsonArray::fromVariantList(f.client->conversations())},{"messages",QJsonArray::fromVariantList(f.client->messages())}};const auto bytes=QJsonDocument(result).toJson(QJsonDocument::Compact);std::printf("%s\n",bytes.constData());f.finish();}}
static void navigation(){Fixture f;f.history();auto status=f.client->status();f.client->nextPage();need(f.client->messages().isEmpty()&&f.client->conversations().size()==2&&f.client->status()==status,"next messages clear/status");f.request("conversations");f.reply(conversations());auto q=f.request("messages");need(q["after_ordinal"].toInteger()==4,"last ordinal cursor");f.reply(messages());f.client->firstPage();f.request("conversations");f.reply(conversations());q=f.request("messages");need(q["after_ordinal"].toInteger()==0,"first message page reset");f.reply(messages());const auto n=f.changes;f.client->select(QString(32,'A'));need(f.changes==n&&f.client->messages().size()==2,"invalid select no event");f.client->nextConversations();need(f.client->messages().isEmpty()&&f.client->conversations().isEmpty(),"next conversations clears both");q=f.request("conversations");need(q["after"]==B,"last conversation cursor");f.reply(conversations());f.client->firstConversations();q=f.request("conversations");need(q["after"].isNull(),"first conversation null cursor");f.reply(conversations());f.finish();std::puts("PASS current_qt_navigation");}
static void stale(bool queued){Fixture f;f.request("conversations");if(queued){f.reply(conversations(),false);usleep(60000);}f.client->select(B);if(!queued)f.reply(conversations(),false);events(130);need(f.client->conversations().isEmpty()&&f.client->messages().isEmpty()&&f.client->status()=="Messages unavailable","old generation never publishes");auto q=f.request("messages");need(q["conversation"]==B,"fresh selected query");f.reply(messages());need(f.client->messages().size()==2,"fresh generation publishes");f.finish();std::puts(queued?"PASS current_qt_queued_stale":"PASS current_qt_inflight_stale");}
static void delayed(){Fixture f;f.request("conversations");const auto first=boot();f.reply(conversations(),false);usleep(1050000);need(boot()-first>1000000000,"actual GUI delivery hold exceeds request ttl");events(40);need(f.client->conversations().size()==2&&f.client->status()==History,"timely receive survives delayed GUI delivery");f.finish();std::puts("PASS current_qt_timely_receive_delayed_ui");}
static void denial(){Fixture f;f.history();f.request("conversations");f.reply(R"({"result":"denied"})");need(f.client->conversations().isEmpty()&&f.client->messages().isEmpty()&&f.client->status()=="Messages access denied","denial clears both published lists");f.finish();std::puts("PASS current_qt_denial");}
int main(int argc,char**argv){QCoreApplication app(argc,argv);if(argc!=2||geteuid()!=0)return 64;try{const QByteArray mode=argv[1];if(mode=="parser")parser();else if(mode=="navigation")navigation();else if(mode=="inflight-stale")stale(false);else if(mode=="queued-stale")stale(true);else if(mode=="delayed-ui")delayed();else if(mode=="denial")denial();else return 64;return 0;}catch(const std::exception&e){std::fprintf(stderr,"FAILED %s\n",e.what());return 101;}}
