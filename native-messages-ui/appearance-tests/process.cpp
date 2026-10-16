// Real UID11010 production clients, installed QML and root seqpacket peers.
// No window, screenshot, compositor, production broker or lifecycle authority.
#include "MessagesClient.h"
#include "PhoneAppearanceClient.h"
#include <QGuiApplication>
#include <QQmlEngine>
#include <QQmlComponent>
#include <QQuickItem>
#include <QQuickStyle>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QScopedPointer>
#include <QColor>
#include <QThread>
#include <QEventLoop>
#include <atomic>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cerrno>
#include <vector>
#include <string>
#include <stdexcept>
#include <new>
#include <poll.h>
#include <grp.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/prctl.h>
#include <time.h>
#include <unistd.h>
static quint64 boot(){timespec t{};if(clock_gettime(CLOCK_BOOTTIME,&t))std::abort();return quint64(t.tv_sec)*1000000000+quint64(t.tv_nsec);}
static void require(bool value,const char* label){if(!value){std::fprintf(stderr,"BOUNDARY_FAILURE %s\n",label);throw std::runtime_error(label);}}
struct Shared {std::atomic<bool> stop{false};std::atomic<int> mode{0},revision{1},requests{0},clientUid{-1};};
static QJsonObject appearance(int mode,int revision){
 QJsonObject palette;for(const char* key:{"background","foreground","surface","border","accent","muted","urgent","positive","ink"})palette.insert(key,QJsonObject{{"r",mode==1?17476:4369},{"g",mode==1?21845:8738},{"b",mode==1?26214:13107},{"a",65535}});
 QJsonObject fonts;int n=10;for(const char* key:{"caption","bodySmall","body","subtitle","title","heading","display","displayLarge"})fonts.insert(key,n++);
 QJsonObject spacing;for(const char* key:{"controlGap","panelPadding","panelGap","rowGap","controlPaddingX","controlPaddingY"})spacing.insert(key,8);
 return {{"schema",3},{"owner","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},{"revision",revision},{"until_ns",QString::number(boot()+400000000)},{"available",mode!=2},{"appearance",mode==2?QJsonValue(QJsonValue::Null):QJsonValue(QJsonObject{{"palette",palette},{"style",QJsonObject{{"base_size",12},{"font_tokens",fonts},{"spacing_tokens",spacing}}}})}};
}
struct Peer {
 Shared* shared;pid_t pid;quint64 until;QByteArray path;bool waited=false;
 Peer(const QString& pathname,bool isAppearance):until(boot()+7000000000),path(pathname.toLocal8Bit()){
  void* memory=mmap(nullptr,sizeof(Shared),PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);require(memory!=MAP_FAILED,"mmap");shared=new(memory)Shared;
  int ready[2];require(pipe2(ready,O_CLOEXEC)==0,"ready pipe");pid=fork();require(pid>=0,"fork");
  if(pid==0){
   close(ready[0]);prctl(PR_SET_PDEATHSIG,SIGKILL);
   int listener=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);if(listener<0)_exit(71);
   sockaddr_un address{};address.sun_family=AF_UNIX;if(path.size()>=int(sizeof(address.sun_path)))_exit(71);std::memcpy(address.sun_path,path.constData(),size_t(path.size())+1);
   if(bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))||chmod(path.constData(),0666)||listen(listener,4))_exit(71);
   if(write(ready[1],"R",1)!=1){_exit(71);}
   close(ready[1]);std::vector<int> clients;
   while(!shared->stop.load()&&boot()<until){
    int fd=accept4(listener,nullptr,nullptr,SOCK_NONBLOCK|SOCK_CLOEXEC);if(fd>=0){ucred credentials{};socklen_t size=sizeof(credentials);if(getsockopt(fd,SOL_SOCKET,SO_PEERCRED,&credentials,&size))_exit(71);shared->clientUid=credentials.uid;clients.push_back(fd);}
    for(auto it=clients.begin();it!=clients.end();){char data[4096];const auto count=recv(*it,data,sizeof(data),MSG_DONTWAIT);if(count==0){close(*it);it=clients.erase(it);continue;}if(count>0){
      ++shared->requests;const auto query=QJsonDocument::fromJson(QByteArray(data,int(count))).object();QJsonObject response;
      if(isAppearance){if(query["op"]!="snapshot")_exit(72);response=appearance(shared->mode.load(),shared->revision.load());}
      else if(query["op"]!="messages_read")_exit(72);
      else if(shared->mode==1)response={{"result","denied"}};
      else if(shared->mode==2)response={{"result","unavailable"}};
      else if(query["request"].toObject()["query"].toObject()["query"]=="conversations")response={{"page","conversations"},{"items",QJsonArray{QJsonObject{{"id","11111111111111111111111111111111"},{"sender","Synthetic sender"},{"slot",0}}}}};
      else response={{"page","messages"},{"items",QJsonArray{QJsonObject{{"receipt","22222222222222222222222222222222"},{"ordinal",1},{"sender","Synthetic sender"},{"text","<b>synthetic literal</b>"}}}}};
      const auto bytes=QJsonDocument(response).toJson(QJsonDocument::Compact);if(send(*it,bytes.constData(),size_t(bytes.size()),MSG_DONTWAIT|MSG_NOSIGNAL)!=bytes.size())_exit(73);
     }else if(errno!=EAGAIN&&errno!=EINTR)_exit(74);++it;}
    usleep(1000);
   }
   for(int fd:clients){close(fd);}
   close(listener);_exit(shared->stop.load()?0:75);
  }
  close(ready[1]);pollfd p{ready[0],POLLIN,0};const bool okay=poll(&p,1,1000)>0;char byte=0;const bool received=okay&&read(ready[0],&byte,1)==1;close(ready[0]);if(!received||byte!='R'){shared->stop=true;finish();munmap(shared,sizeof(Shared));throw std::runtime_error("root peer ready");}
 }
 void finish(){if(waited)return;shared->stop=true;int status=0;while(boot()<until){const auto r=waitpid(pid,&status,WNOHANG);if(r==pid){waited=true;std::printf("ROOT_PEER_ACTUAL_WAIT pid=%d code=%d waited_at_ns=%llu original_until_ns=%llu stdout=INHERITED stderr=INHERITED\n",pid,WIFEXITED(status)?WEXITSTATUS(status):-1,static_cast<unsigned long long>(boot()),static_cast<unsigned long long>(until));require(WIFEXITED(status)&&WEXITSTATUS(status)==0,"root peer status");return;}require(r==0,"waitpid");usleep(1000);}std::abort();}
 ~Peer(){finish();munmap(shared,sizeof(Shared));}
};
template<class F>static bool wait(F test,quint64 until){while(boot()<until){QCoreApplication::processEvents(QEventLoop::AllEvents,2);if(test())return true;QThread::msleep(1);}return false;}
static int bodies(QQuickItem* item,QString* text=nullptr){int found=0;if(item->objectName()=="messageBody"){++found;if(text)*text=item->property("text").toString();require(item->property("textFormat").toInt()==0,"plain text body");}for(auto* child:item->childItems())found+=bodies(child,text);return found;}
static int arguments(const char* binary){
 for(int mode=0;mode<4;++mode){const auto until=boot()+2000000000;const auto pid=fork();require(pid>=0,"normal fork");if(pid==0){if(mode!=2){const auto uid=mode==3?1000:11010;if(setgroups(0,nullptr)||setgid(uid)||setuid(uid))_exit(71);}std::vector<std::string> args{binary};if(mode!=0){for(int i=0;i<6;++i)args.emplace_back("/nonexistent-boundary-path");if(mode==1)args.emplace_back("extra");}std::vector<char*> raw;for(auto& a:args)raw.push_back(a.data());raw.push_back(nullptr);execv(binary,raw.data());_exit(72);}int status=0;bool reaped=false;while(boot()<until){if(waitpid(pid,&status,WNOHANG)==pid){reaped=true;break;}usleep(1000);}if(!reaped){kill(pid,SIGKILL);require(waitpid(pid,&status,0)==pid,"normal kill wait");}const int expected=mode<2?64:65;std::printf("NORMAL_REJECTION_ACTUAL_WAIT pid=%d code=%d expected=%d waited_at_ns=%llu original_until_ns=%llu stdout=INHERITED stderr=INHERITED\n",pid,WIFEXITED(status)?WEXITSTATUS(status):-1,expected,static_cast<unsigned long long>(boot()),static_cast<unsigned long long>(until));require(reaped&&WIFEXITED(status)&&WEXITSTATUS(status)==expected,"normal argc UID rejection");}
 return 0;
}
int main(int argc,char** argv){
 if(argc!=4||getuid()!=0)return 64;
 try{
  const QString mode=QString::fromLocal8Bit(argv[1]);if(mode=="arguments")return arguments(argv[3]);
  char rootTemplate[]="/run/messages-qml-XXXXXX";char* temporary=mkdtemp(rootTemplate);require(temporary,"temp root");QString root=QString::fromLocal8Bit(temporary);require(chmod(temporary,0755)==0,"public parent");
  const auto runtime=(root+"/runtime").toLocal8Bit();require(mkdir(runtime.constData(),0700)==0&&chown(runtime.constData(),11010,11010)==0,"app runtime");
  Peer broker(root+"/appearance.sock",true);Peer actor(root+"/app.sock",false);
  require(setgroups(0,nullptr)==0&&setgid(11010)==0&&setuid(11010)==0,"app principal");qputenv("QT_QPA_PLATFORM","offscreen");qputenv("QT_QUICK_BACKEND","software");qputenv("XDG_RUNTIME_DIR",runtime);qputenv("HOME",runtime);QGuiApplication app(argc,argv);QQuickStyle::setStyle("Basic");
  const auto work=boot()+5000000000;
  PhoneAppearanceClient appearanceClient(root+"/appearance.sock");MessagesClient messages(root+"/app.sock");QQmlEngine engine;
  QQmlComponent component(&engine,QUrl::fromLocalFile(QString::fromLocal8Bit(argv[2])+"/PhoneAppearance.qml"));require(component.isReady(),"installed PhoneAppearance.qml must be Ready");
  QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source",QVariant::fromValue(&appearanceClient)}}));require(!theme.isNull(),"installed typed component create");
  QQmlComponent viewComponent(&engine,QUrl("qrc:/messages/MessagesView.qml"));require(viewComponent.isReady(),"normal resource component Ready");
  QScopedPointer<QObject> view(viewComponent.createWithInitialProperties({{"theme",QVariant::fromValue(theme.data())},{"messagesClient",QVariant::fromValue(&messages)}}));require(!view.isNull(),"normal Messages QML create");auto* item=qobject_cast<QQuickItem*>(view.data());require(item,"normal view root");
  require(wait([&]{return appearanceClient.available()&&theme->property("available").toBool();},work),"actual root appearance publication");require(item->property("color").value<QColor>()==QColor("#112233"),"normal view binds typed palette");
  require(wait([&]{return !messages.messages().isEmpty();},work),"actual root Messages response");
  if(mode=="live-appearance"){
   broker.shared->mode=1;broker.shared->revision=2;require(wait([&]{return item->property("color").value<QColor>()==QColor("#445566");},work),"live typed palette update");broker.shared->mode=2;broker.shared->revision=3;require(wait([&]{return !appearanceClient.available()&&!theme->property("available").toBool();},work),"appearance unavailable propagation");require(!messages.messages().isEmpty(),"appearance is not Messages authority");
  }else if(mode=="messages-policy"){
   QString text;require(wait([&]{return bodies(item,&text)==1;},work)&&text=="<b>synthetic literal</b>","literal normal message delegate");actor.shared->mode=1;require(wait([&]{return messages.status()=="Messages access denied"&&messages.messages().isEmpty()&&messages.conversations().isEmpty()&&bodies(item)==0;},work),"denial clears normal view");actor.shared->mode=2;require(wait([&]{return messages.status()=="Messages unavailable";},work),"unavailability remains distinct");require(appearanceClient.available(),"Messages denial is not appearance authority");
  }else require(mode=="installed","known mode");
  require(broker.shared->clientUid==11010&&actor.shared->clientUid==11010,"actual per-app socket principal");
  std::printf("BOUNDARY_PASS mode=%s app_uid=%u broker_pid=%d actor_pid=%d actual_root_seqpacket=true installed_QML=true normal_resource=true Window_created=false\n",qPrintable(mode),getuid(),broker.pid,actor.pid);
  return 0;
 }catch(const std::exception& error){std::fprintf(stderr,"BOUNDARY_RESULT %s\n",error.what());return 101;}
}
