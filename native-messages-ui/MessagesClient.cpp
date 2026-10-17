#include "MessagesClient.h"
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonParseError>
#include <QMetaObject>
#include <QThread>
#include <algorithm>
#include <cerrno>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <utility>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
namespace {
void result(int32_t code){
 // Refusal/expiry can themselves produce a terminal plan. Always drive that
 // plan; a refused report never releases a native descriptor or thread.
 if(code<PR_MESSAGES_OK||code>PR_MESSAGES_ABORT||code==PR_MESSAGES_INVALID||code==PR_MESSAGES_ABORT)std::abort();
}
struct NativeSocket {
 int fd=-1;
 pid_t server=0;
 void close(){
  if(fd<0)return;
  const int owned=fd;fd=-1;server=0;
  // Linux releases a close(EINTR) descriptor; retry could close a reused fd.
  if(::close(owned)<0&&errno!=EINTR)std::abort();
 }
 ~NativeSocket(){close();}
};
void pollNative(int fd,short events,const PrMessagesStepV1 &plan){
 if(plan.wait_slice_ms==0||plan.wait_slice_ms>20)std::abort();
 pollfd descriptor{fd,events,0};
 ::poll(&descriptor,1,int(plan.wait_slice_ms));
}
QVariantList project(const uint8_t *bytes,uint32_t length){
 if(length>4096)std::abort();
 QJsonParseError error{};
 const auto document=QJsonDocument::fromJson(QByteArray(reinterpret_cast<const char*>(bytes),int(length)),&error);
 if(error.error!=QJsonParseError::NoError||!document.isArray())std::abort();
 return document.array().toVariantList();
}
bool authenticated(const msghdr &message,pid_t server){
 bool valid=(message.msg_flags&(MSG_TRUNC|MSG_CTRUNC))==0;
 bool seen=false,trusted=false;
 // The kernel supplies ancillary framing; reject every unexpected record and
 // close every delivered rights descriptor, including truncated refusals.
 auto copy=message;
 for(auto *c=CMSG_FIRSTHDR(&copy);c;c=CMSG_NXTHDR(&copy,c)){
  if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS&&c->cmsg_len==CMSG_LEN(sizeof(ucred))&&!seen){
   seen=true;ucred actual{};std::memcpy(&actual,CMSG_DATA(c),sizeof(actual));trusted=actual.uid==0&&actual.pid==server;
  }else{
   valid=false;
   if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_RIGHTS&&c->cmsg_len>=CMSG_LEN(0)){
    const auto count=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);
    for(size_t i=0;i<count;++i){int delivered=-1;std::memcpy(&delivered,CMSG_DATA(c)+i*sizeof(int),sizeof(delivered));if(delivered>=0)::close(delivered);}
   }
  }
 }
 return valid&&seen&&trusted;
}
}
MessagesClient::MessagesClient(QString socket,QObject *parent):QObject(parent){
 if(pr_messages_client_new_v1(&core)!=PR_MESSAGES_OK||!core)std::abort();
 thread=QThread::create([this,socket]{loop(socket);});
 if(!thread)std::abort();
 thread->start();
}
MessagesClient::~MessagesClient(){
 if(pr_messages_client_stop_v1(core)!=PR_MESSAGES_OK)std::abort();
 for(;;){
  const bool completed=thread->wait(20);
  const auto verdict=pr_messages_client_join_result_v1(core,completed?1:0);
  if(verdict==PR_MESSAGES_OK)break;
  if(verdict!=PR_MESSAGES_WAIT)std::abort();
 }
 delete thread;thread=nullptr;
 if(pr_messages_client_free_v1(core)!=PR_MESSAGES_OK)std::abort();
 core=nullptr;
}
void MessagesClient::sync(){
 PrMessagesViewV1 view{};
 if(pr_messages_client_view_v1(core,&view)!=PR_MESSAGES_OK)std::abort();
 if(view.model_revision==projectedRevision)return;
 auto conversations=project(view.conversations_json,view.conversations_length);
 auto messages=project(view.messages_json,view.messages_length);
 QString status;
 switch(view.status){
  case PR_MESSAGES_UNAVAILABLE:status="Messages unavailable";break;
  case PR_MESSAGES_HISTORY:status="Local receipt history · modem receive and send unavailable";break;
  case PR_MESSAGES_DENIED:status="Messages access denied";break;
  default:std::abort();
 }
 convo=std::move(conversations);texts=std::move(messages);currentStatus=std::move(status);
 projectedRevision=view.model_revision; // Set before reentrant changed handlers.
 emit changed();
}
void MessagesClient::action(uint32_t kind,const PrMessagesIdV1 *id){
 result(pr_messages_client_action_v1(core,kind,id));sync();
}
void MessagesClient::select(const QString &value){
 const auto bytes=value.toUtf8();PrMessagesIdV1 id{};
 id.length=uint32_t(std::min<quint64>(quint64(bytes.size()),std::numeric_limits<uint32_t>::max()));
 std::memcpy(id.bytes,bytes.constData(),std::min<size_t>(sizeof(id.bytes),size_t(bytes.size())));
 // Rust decides exact length/hex syntax; this only bounds the copied ABI input.
 const auto code=pr_messages_client_action_v1(core,PR_MESSAGES_SELECT,&id);
 if(code==PR_MESSAGES_INVALID)return;
 result(code);sync();
}
void MessagesClient::nextPage(){action(PR_MESSAGES_NEXT_MESSAGES);}
void MessagesClient::firstPage(){action(PR_MESSAGES_FIRST_MESSAGES);}
void MessagesClient::nextConversations(){action(PR_MESSAGES_NEXT_CONVERSATIONS);}
void MessagesClient::firstConversations(){action(PR_MESSAGES_FIRST_CONVERSATIONS);}
void MessagesClient::loop(const QString &path){
 NativeSocket native;
 if(pr_messages_client_worker_enter_v1(core)!=PR_MESSAGES_OK)std::abort();
 const auto encoded=path.toLocal8Bit();
 for(;;){
  PrMessagesStepV1 plan{};
  const auto status=pr_messages_client_step_v1(core,&plan);
  if(status==PR_MESSAGES_WAIT){QThread::yieldCurrentThread();continue;}
  if(status!=PR_MESSAGES_OK||plan.reserved!=0)std::abort();
  switch(plan.action){
   case PR_MESSAGES_CONNECT:{
    if(native.fd>=0)std::abort();
    sockaddr_un address{};address.sun_family=AF_UNIX;
    if(encoded.size()>=int(sizeof(address.sun_path))){result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_PATH_REFUSED,0));break;}
    native.fd=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);
    int one=1;ucred peer{};socklen_t size=sizeof(peer);
    std::memcpy(address.sun_path,encoded.constData(),size_t(encoded.size())+1);
    if(native.fd<0||setsockopt(native.fd,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one))||::connect(native.fd,reinterpret_cast<sockaddr*>(&address),sizeof(address))||getsockopt(native.fd,SOL_SOCKET,SO_PEERCRED,&peer,&size)||size!=sizeof(peer)||peer.uid!=0||peer.pid<=0){
     native.close();result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_CONNECT_FAILED,0));break;
    }
    native.server=peer.pid;
    result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_CONNECTED,0));break;
   }
   case PR_MESSAGES_SEND:{
    if(native.fd<0||plan.packet_length==0||plan.packet_length>sizeof(plan.packet))std::abort();
    const auto sent=::send(native.fd,plan.packet,plan.packet_length,MSG_NOSIGNAL|MSG_DONTWAIT);
    if(sent==ssize_t(plan.packet_length)){result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_SENT,uint32_t(sent)));break;}
    if(sent<0&&(errno==EAGAIN||errno==EWOULDBLOCK||errno==EINTR)){
     const auto outcome=errno==EINTR?PR_MESSAGES_INTERRUPTED:PR_MESSAGES_WOULD_BLOCK;
     result(pr_messages_client_report_v1(core,&plan,outcome,0));pollNative(native.fd,POLLOUT,plan);break;
    }
    result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_IO_FAILED,0));break;
   }
   case PR_MESSAGES_RECEIVE:{
    if(native.fd<0)std::abort();
    uint8_t bytes[4096];alignas(cmsghdr) char ancillary[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(sizeof(int))]{};
    iovec vector{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&vector;message.msg_iovlen=1;message.msg_control=ancillary;message.msg_controllen=sizeof(ancillary);
    const auto received=::recvmsg(native.fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);
    if(received<0&&(errno==EAGAIN||errno==EWOULDBLOCK||errno==EINTR)){
     const auto outcome=errno==EINTR?PR_MESSAGES_INTERRUPTED:PR_MESSAGES_WOULD_BLOCK;
     result(pr_messages_client_report_v1(core,&plan,outcome,0));pollNative(native.fd,POLLIN,plan);break;
    }
    const bool trusted=received>=0&&authenticated(message,native.server);
    if(received<=0||!trusted){result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_IO_FAILED,0));break;}
    result(pr_messages_client_received_v1(core,&plan,bytes,uint32_t(received)));break;
   }
   case PR_MESSAGES_NOTIFY:{
    const auto token=plan.event_token;
    if(QMetaObject::invokeMethod(this,[this,token]{if(core){result(pr_messages_client_deliver_v1(core,token));sync();}},Qt::QueuedConnection)){
     result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_NOTIFY_QUEUED,0));
    }else if(pr_messages_client_stop_v1(core)!=PR_MESSAGES_OK){std::abort();}
    break;
   }
   case PR_MESSAGES_SLEEP:
    if(plan.wait_slice_ms==0||plan.wait_slice_ms>10)std::abort();
    QThread::msleep(plan.wait_slice_ms);break;
   case PR_MESSAGES_CLOSE:
    native.close();result(pr_messages_client_report_v1(core,&plan,PR_MESSAGES_CLOSED,0));break;
   case PR_MESSAGES_EXIT:{
    const bool unexpectedDescriptor=native.fd>=0;
    native.close();
    if(unexpectedDescriptor)std::abort();
    if(pr_messages_client_worker_exited_v1(core)!=PR_MESSAGES_OK)std::abort();
    return;
   }
   default:std::abort();
  }
 }
}
