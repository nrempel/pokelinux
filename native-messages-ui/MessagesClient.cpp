#include "MessagesClient.h"
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QRegularExpression>
#include <QMetaObject>
#include <sys/socket.h>
#include <sys/un.h>
#include <poll.h>
#include <unistd.h>
#include <time.h>
#include <cstring>
#include <chrono>
#include <thread>
#include <optional>
#include <limits>
#include <cstdlib>
#include <QThread>
namespace {
std::optional<quint64> now(){
 timespec t{};
 if(clock_gettime(CLOCK_BOOTTIME,&t)!=0||t.tv_sec<0||t.tv_nsec<0||t.tv_nsec>=1000000000)return {};
 const auto seconds=quint64(t.tv_sec), nanos=quint64(t.tv_nsec);
 if(seconds>(std::numeric_limits<quint64>::max()-nanos)/1000000000)return {};
 return seconds*1000000000+nanos;
}
bool current(const std::atomic<bool>&stopped,quint64 until){const auto sample=now();return !stopped&&sample&&*sample<until;}
bool id(const QString&s){return QRegularExpression("^[0-9a-f]{32}$").match(s).hasMatch();}
QByteArray exchange(int fd,const QByteArray &request,pid_t server,const std::atomic<bool>&stopped){
 const auto began=now();
 if(stopped||!began||*began>std::numeric_limits<quint64>::max()-1000000000)return {};
 const auto until=*began+1000000000;
 bool sent=false;
 while(current(stopped,until)){const auto n=send(fd,request.constData(),size_t(request.size()),MSG_NOSIGNAL|MSG_DONTWAIT);if(n==request.size()){sent=true;break;}if(n<0&&(errno==EAGAIN||errno==EINTR)){pollfd p{fd,POLLOUT,0};poll(&p,1,20);continue;}return {};}
 if(!sent)return {};
 while(current(stopped,until)){char bytes[4096];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(sizeof(int))];iovec v{bytes,sizeof(bytes)};msghdr msg{};msg.msg_iov=&v;msg.msg_iovlen=1;msg.msg_control=control;msg.msg_controllen=sizeof(control);
  const auto n=recvmsg(fd,&msg,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);if(n<0&&(errno==EAGAIN||errno==EINTR)){pollfd p{fd,POLLIN,0};poll(&p,1,20);continue;}if(n<=0)return {};
  bool trusted=false,seenCredential=false,invalid=msg.msg_flags&(MSG_TRUNC|MSG_CTRUNC);for(auto*c=CMSG_FIRSTHDR(&msg);c;c=CMSG_NXTHDR(&msg,c)){if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS&&c->cmsg_len==CMSG_LEN(sizeof(ucred))&&!seenCredential){seenCredential=true;ucred u{};memcpy(&u,CMSG_DATA(c),sizeof(u));trusted=u.uid==0&&u.pid==server;}else{invalid=true;if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_RIGHTS){const auto count=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);auto*f=reinterpret_cast<int*>(CMSG_DATA(c));for(size_t i=0;i<count;i++)close(f[i]);}}}
  if(!trusted||invalid||!current(stopped,until)) { return {}; }
  return QByteArray(bytes,int(n));
 }return {};
}
}
MessagesClient::MessagesClient(QString socket,QObject*parent):QObject(parent){thread=QThread::create([this,socket]{loop(socket);});thread->start();}
MessagesClient::~MessagesClient(){
 stopped=true;
 const auto began=now();
 // A successful wait synchronizes complete thread exit, including TLS cleanup.
 // Never enter QThread::create's implicitly waiting destructor with uncertain ownership.
 if(!began||*began>std::numeric_limits<quint64>::max()-500000000){
  if(!thread->wait(20))std::abort();
 }else{
  const auto until=*began+500000000;
  while(!thread->wait(20)){
   const auto sample=now();
   if(!sample||*sample>=until)std::abort();
  }
 }
 delete thread;
}
void MessagesClient::select(const QString&v){if(!id(v))return;{std::lock_guard<std::mutex>g(requests);selected=v;after=0;requestGeneration++;}texts.clear();emit changed();}
void MessagesClient::nextPage(){if(texts.isEmpty())return;{std::lock_guard<std::mutex>g(requests);after=texts.last().toMap()["ordinal"].toULongLong();requestGeneration++;}texts.clear();emit changed();}
void MessagesClient::firstPage(){{std::lock_guard<std::mutex>g(requests);after=0;requestGeneration++;}texts.clear();emit changed();}
void MessagesClient::nextConversations(){if(convo.isEmpty())return;{std::lock_guard<std::mutex>g(requests);conversationAfter=convo.last().toMap()["id"].toString();selected.clear();after=0;requestGeneration++;}convo.clear();texts.clear();emit changed();}
void MessagesClient::firstConversations(){{std::lock_guard<std::mutex>g(requests);conversationAfter.clear();selected.clear();after=0;requestGeneration++;}convo.clear();texts.clear();emit changed();}
void MessagesClient::loop(const QString &path){
 int fd=-1;pid_t server=0;bool conversation=true;
 while(!stopped){
  if(fd<0){fd=socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);sockaddr_un addr{};addr.sun_family=AF_UNIX;const auto p=path.toLocal8Bit();if(fd<0||p.size()>=int(sizeof(addr.sun_path))){if(fd>=0)close(fd);return;}memcpy(addr.sun_path,p.constData(),size_t(p.size())+1);
   int one=1;setsockopt(fd,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one));ucred peer{};socklen_t length=sizeof(peer);if(::connect(fd,reinterpret_cast<sockaddr*>(&addr),sizeof(addr))||getsockopt(fd,SOL_SOCKET,SO_PEERCRED,&peer,&length)||peer.uid!=0){close(fd);fd=-1;std::this_thread::sleep_for(std::chrono::milliseconds(100));continue;}server=peer.pid;
  }
  QString chosen,cursor;quint64 ordinal,generation;{std::lock_guard<std::mutex>g(requests);chosen=selected;cursor=conversationAfter;ordinal=after;generation=requestGeneration;}
  QJsonObject query;if(conversation||chosen.isEmpty())query={{"query","conversations"},{"after",cursor.isEmpty()?QJsonValue(QJsonValue::Null):QJsonValue(cursor)},{"limit",8}};else query={{"query","messages"},{"conversation",chosen},{"after_ordinal",qint64(ordinal)},{"limit",2}};
  const auto data=exchange(fd,QJsonDocument(QJsonObject{{"op","messages_read"},{"request",QJsonObject{{"query",query},{"ttl_ms",1000}}}}).toJson(QJsonDocument::Compact),server,stopped);
  if(data.isEmpty()){close(fd);fd=-1;}
  if(stopped)break;
  QMetaObject::invokeMethod(this,[this,data,generation]{std::lock_guard<std::mutex>g(requests);if(generation==requestGeneration)apply(data);},Qt::QueuedConnection);
  conversation=!conversation;for(int i=0;i<25&&!stopped;i++)std::this_thread::sleep_for(std::chrono::milliseconds(10));
 }
 if(fd>=0)close(fd);
}
void MessagesClient::apply(const QByteArray&wire){
 const auto value=QJsonDocument::fromJson(wire).object();const auto kind=value["page"].toString();const auto items=value["items"].toArray();bool valid=wire.size()<=4096&&value["items"].isArray()&&(kind=="conversations"||kind=="messages");
 valid=valid&&items.size()<=(kind=="conversations"?8:2);QVariantList out;
 for(const auto&v:items){const auto m=v.toObject();const auto sender=m["sender"].toString();valid=valid&&v.isObject()&&!sender.isEmpty()&&sender.toUtf8().size()<=64;
  if(kind=="conversations"){valid=valid&&id(m["id"].toString())&&m["slot"].toInt(-1)>=0&&m["slot"].toInt()>-1&&m["slot"].toInt()<8;}
  else{valid=valid&&id(m["receipt"].toString())&&m["text"].isString()&&m["text"].toString().toUtf8().size()<=640&&m["ordinal"].toInteger(-1)>0&&m["ordinal"].toInteger()>0&&m["ordinal"].toInteger()<=4096;}
  out.append(m.toVariantMap());
 }
 if(!valid){convo.clear();texts.clear();currentStatus=value["result"].toString()=="denied"?"Messages access denied":"Messages unavailable";emit changed();return;}
 currentStatus="Local receipt history · modem receive and send unavailable";
 if(kind=="conversations"){convo=out;if(selected.isEmpty()&&!out.isEmpty()){selected=out.first().toMap()["id"].toString();after=0;}}
 else texts=out;
 emit changed();
}
