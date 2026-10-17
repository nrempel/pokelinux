#include "AudioClient.h"
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QRegularExpression>
#include <QSocketNotifier>
#include <cerrno>
#include <cmath>
#include <cstring>
#include <limits>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

namespace {
constexpr qint64 requestNs=1000000000;
constexpr int maxPacket=4096,pageSize=8,maxRoutes=32;
bool selector(const QString &s) {
 static const QRegularExpression pattern(QStringLiteral("^[A-Za-z0-9_.-]{1,64}$"));
 return pattern.match(s).hasMatch();
}
bool label(const QString &s) {
 if(s.trimmed().isEmpty() || s.toUtf8().size()>96) return false;
 for(const auto c:s) if(c.isNull() || c.category()==QChar::Other_Control) return false;
 return true;
}
bool encodeRoute(const QString &id,PrSoundsRouteV1 &out) {
 const auto bytes=id.toUtf8();
 if(bytes.isEmpty() || bytes.size()>64)return false;
 out={};out.length=uint32_t(bytes.size());std::memcpy(out.bytes,bytes.constData(),size_t(bytes.size()));return true;
}
bool nullableSelector(const QJsonValue &v) { return v.isNull() || (v.isString() && selector(v.toString())); }
bool state(const QJsonValue &v) {
 if(!v.isObject()) return false;
 const auto s=v.toObject();
 return s.size()==5 && s.value("connected").isBool() && s.value("available").isBool()
  && s.value("pending").isBool() && nullableSelector(s.value("requested")) && nullableSelector(s.value("applied"));
}
}
qint64 AudioClient::boottime() {
 timespec ts{};
 if(::clock_gettime(CLOCK_BOOTTIME,&ts) || ts.tv_sec<0 || ts.tv_nsec<0 || ts.tv_nsec>=1000000000
    || ts.tv_sec>(std::numeric_limits<qint64>::max()-999999999)/1000000000) return -1;
 return qint64(ts.tv_sec)*1000000000+ts.tv_nsec;
}
bool AudioClient::expired() const { const auto now=boottime();return now<0 || now>=m_deadline; }
bool AudioClient::setDeadline() {
 const auto now=boottime();
 if(now<0 || now>std::numeric_limits<qint64>::max()-requestNs) return false;
 m_deadline=now+requestNs;return true;
}
AudioClient::AudioClient(QString path,uid_t ownerUid,QObject *parent)
 :QObject(parent),m_path(std::move(path)),m_ownerUid(ownerUid) {
 if(pr_sounds_queue_new_v1(&m_intents)!=PR_SOUNDS_OK){m_status="Audio command state unavailable";return;}
 m_poll.setInterval(250);m_watch.setInterval(25);
 connect(&m_poll,&QTimer::timeout,this,[this]{if(m_online && m_pending.isEmpty())statusRequest();});
 connect(&m_watch,&QTimer::timeout,this,[this]{if(!m_pending.isEmpty() && expired())fail("Audio request expired; reconnect to inspect");});
 QTimer::singleShot(0,this,[this]{if(m_fd<0 && m_pending.isEmpty())connectActor();});
}
AudioClient::~AudioClient(){m_closing=true;closeSocket();pr_sounds_queue_free_v1(m_intents);}
void AudioClient::closeSocket() {
 ++m_generation;m_poll.stop();m_watch.stop();
 for(auto **notifier:{&m_read,&m_write}) {
  if(*notifier){(*notifier)->setEnabled(false);(*notifier)->deleteLater();*notifier=nullptr;}
 }
 if(m_fd>=0)::close(m_fd);
 pr_sounds_queue_cancel_v1(m_intents);m_sendToken=0;m_available=false;
 m_fd=-1;m_peerPid=-1;m_stage=Stage::Idle;m_online=false;m_pending.clear();m_output.clear();m_pageRoutes.clear();m_offset=0;
}
void AudioClient::fail(const QString &message) {
 if(m_closing)return;
 m_closing=true;closeSocket();m_actorCanChoose=false;m_actorCanReconnect=false;
 m_routes.clear();m_requested.clear();m_applied.clear();m_status=message;m_closing=false;emit changed();
}
bool AudioClient::peer() {
 ucred who{};socklen_t size=sizeof(who);
 if(::getsockopt(m_fd,SOL_SOCKET,SO_PEERCRED,&who,&size) || size!=sizeof(who)
    || who.uid!=m_ownerUid || who.gid!=0 || who.pid<=0)return false;
 m_peerPid=who.pid;return true;
}
void AudioClient::connectActor() {
 if(m_fd>=0 || !m_pending.isEmpty())return;
 const auto path=m_path.toLocal8Bit();sockaddr_un address{};address.sun_family=AF_UNIX;
 if(!m_path.startsWith('/') || path.isEmpty() || path.contains('\0') || path.size()>=int(sizeof(address.sun_path)) || !setDeadline()) {
  fail("Audio actor path or clock refused");return;
 }
 std::memcpy(address.sun_path,path.constData(),size_t(path.size())+1);
 m_pending="connect";m_stage=Stage::Connecting;m_status="Connecting to audio";m_watch.start();
 m_fd=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);int enabled=1;
 if(m_fd<0 || ::setsockopt(m_fd,SOL_SOCKET,SO_PASSCRED,&enabled,sizeof(enabled))) {fail("Audio actor connection refused");return;}
 const auto generation=m_generation;
 m_read=new QSocketNotifier(m_fd,QSocketNotifier::Read,this);m_write=new QSocketNotifier(m_fd,QSocketNotifier::Write,this);
 m_read->setEnabled(false);m_write->setEnabled(false);
 connect(m_read,&QSocketNotifier::activated,this,[this,generation]{if(generation==m_generation)receive();});
 connect(m_write,&QSocketNotifier::activated,this,[this,generation]{if(generation==m_generation)writable();});
 const auto connected=::connect(m_fd,reinterpret_cast<sockaddr*>(&address),sizeof(address));const auto connectError=errno;
 if(expired()){fail("Audio actor connection expired");return;}
 if(connected<0) {
  if(connectError!=EINPROGRESS){fail("Audio actor unavailable; reconnect to inspect");return;}
  m_write->setEnabled(true);
 }else writable();
 emit changed();
}
void AudioClient::reconnect() {
 if(!canReconnect())return;
 if(m_fd<0)connectActor(); // A new app transport inspects only, never replays.
 else if(m_pending=="status") {
  if(pr_sounds_queue_offer_v1(m_intents,PR_SOUNDS_RECONNECT,nullptr)!=PR_SOUNDS_OK){fail("Audio clock unavailable");return;}
  emit changed();
 }else request({{"op","media_reconnect"}},"reconnect");
}
void AudioClient::statusRequest() {
 if(!m_online || !m_pending.isEmpty())return;
 m_pageRoutes.clear();m_offset=0;request({{"op","media_status"},{"offset",0}},"status");
}
void AudioClient::request(const QJsonObject &message,const QString &kind,qint64 originalUntil) {
 if(!m_online || !m_pending.isEmpty())return;
 if(originalUntil>0) { m_deadline=originalUntil;if(expired()){fail("Audio request expired");return;} }
 else if(!setDeadline()){fail("Audio clock unavailable");return;}
 m_pending=kind;m_watch.start();queue(message);emit changed();
}
bool AudioClient::queue(const QJsonObject &message) {
 m_output=QJsonDocument(message).toJson(QJsonDocument::Compact);
 if(m_output.isEmpty() || m_output.size()>maxPacket || expired()){fail("Audio request refused");return false;}
 m_stage=Stage::Sending;writable();return m_fd>=0;
}
void AudioClient::writable() {
 if(m_fd<0 || expired()){fail("Audio request expired");return;}
 if(m_stage==Stage::Connecting) {
  int error=0;socklen_t size=sizeof(error);
  if(::getsockopt(m_fd,SOL_SOCKET,SO_ERROR,&error,&size) || size!=sizeof(error) || error || !peer() || expired()) {fail("Audio actor identity refused");return;}
  m_online=true;m_poll.start();m_pending="status";m_offset=0;m_pageRoutes.clear();
  m_read->setEnabled(true);queue({{"op","media_status"},{"offset",0}});return;
 }
 if(m_stage!=Stage::Sending)return;
 if(m_sendToken) {
  PrSoundsCommandV1 command{};const auto result=prepareQueued(command);
  if(result==PR_SOUNDS_EXPIRED || result==PR_SOUNDS_REFUSED) {
   m_sendToken=0;m_pending.clear();m_output.clear();m_stage=Stage::Idle;
   m_write->setEnabled(false);m_watch.stop();emit changed();return;
  }
  if(result!=PR_SOUNDS_OK || command.token!=m_sendToken || command.original_until_boottime_ns!=uint64_t(m_deadline)
     || (command.kind==PR_SOUNDS_ROUTE)!=(m_pending=="route")){fail("Audio queued command refused");return;}
  if(command.kind==PR_SOUNDS_ROUTE) {
   const auto id=QString::fromLatin1(reinterpret_cast<const char*>(command.route.bytes),int(command.route.length));
   m_output=QJsonDocument(QJsonObject{{"op","media_route"},{"request",QJsonObject{{"route",id},{"ttl_ms",qint64(command.ttl_ms)}}}}).toJson(QJsonDocument::Compact);
  }
 }
 const auto count=::send(m_fd,m_output.constData(),size_t(m_output.size()),MSG_DONTWAIT|MSG_NOSIGNAL);
 const auto error=errno;
 if(expired()){fail("Audio request expired");return;}
 if(count<0 && (error==EAGAIN || error==EINTR)){m_write->setEnabled(true);return;}
 if(count!=m_output.size()){fail("Audio request could not be sent");return;}
 if(m_sendToken) {
  if(pr_sounds_queue_sent_v1(m_intents,m_sendToken)!=PR_SOUNDS_OK){fail("Audio request expired");return;}
  m_sendToken=0;
 }
 m_output.clear();m_stage=Stage::Receiving;m_write->setEnabled(false);m_read->setEnabled(true);
 if(m_pending=="route") { m_applied.clear();m_status="Output requested";emit changed(); }
}
void AudioClient::select(const QString &id) {
 if(!canChoose())return;
 bool known=false;for(const auto &route:m_routes)if(route.toMap().value("id").toString()==id)known=true;
 if(!known)return;
 if(m_pending=="status") {
  PrSoundsRouteV1 route{};
  if(!encodeRoute(id,route) || pr_sounds_queue_offer_v1(m_intents,PR_SOUNDS_ROUTE,&route)!=PR_SOUNDS_OK){fail("Audio clock unavailable");return;}
  emit changed();return;
 }
 const auto now=boottime();
 if(now<0 || now>std::numeric_limits<qint64>::max()-requestNs){fail("Audio clock unavailable");return;}
 sendSelection(id,now+requestNs);
}
void AudioClient::sendSelection(const QString &id,qint64 originalUntil) {
 const auto now=boottime();
 if(now<0 || now>=originalUntil)return;
 request({{"op","media_route"},{"request",QJsonObject{{"route",id},{"ttl_ms",1000}}}},"route",originalUntil);
}
int AudioClient::prepareQueued(PrSoundsCommandV1 &command) {
 PrSoundsFreshV1 fresh{};
 fresh.transport_online=m_online;fresh.status_complete=m_pending!="status";
 fresh.available=m_available;fresh.can_choose=m_actorCanChoose;fresh.can_reconnect=m_actorCanReconnect;
 fresh.route_count=uint32_t(m_routes.size());
 if(fresh.route_count<=32) {
  for(uint32_t i=0;i<fresh.route_count;++i) {
   if(!encodeRoute(m_routes[int(i)].toMap().value("id").toString(),fresh.routes[i])){fresh.route_count=33;break;}
  }
 }
 return pr_sounds_queue_prepare_v1(m_intents,&fresh,&command);
}
void AudioClient::dispatchQueued() {
 PrSoundsCommandV1 command{};const auto result=prepareQueued(command);
 if(result==PR_SOUNDS_EMPTY || result==PR_SOUNDS_BUSY || result==PR_SOUNDS_EXPIRED || result==PR_SOUNDS_REFUSED)return;
 if(result!=PR_SOUNDS_OK){fail("Audio queued command refused");return;}
 m_sendToken=command.token;
 if(command.kind==PR_SOUNDS_RECONNECT)request({{"op","media_reconnect"}},"reconnect",qint64(command.original_until_boottime_ns));
 else {
  const auto id=QString::fromLatin1(reinterpret_cast<const char*>(command.route.bytes),int(command.route.length));
  request({{"op","media_route"},{"request",QJsonObject{{"route",id},{"ttl_ms",qint64(command.ttl_ms)}}}},"route",qint64(command.original_until_boottime_ns));
 }
}
void AudioClient::receive() {
 if(m_fd<0)return;
 if(m_stage!=Stage::Receiving || m_pending.isEmpty() || expired()){fail("Audio reply outside its request");return;}
 char bytes[maxPacket];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(16*sizeof(int))]{};
 iovec io{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&io;message.msg_iovlen=1;message.msg_control=control;message.msg_controllen=sizeof(control);
 const auto count=::recvmsg(m_fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);const auto error=errno;
 if(count<0 && (error==EAGAIN || error==EINTR)){if(expired())fail("Audio reply expired");return;}
 bool trusted=false,seen=false,invalid=message.msg_flags&(MSG_TRUNC|MSG_CTRUNC);
 for(auto *c=CMSG_FIRSTHDR(&message);count>=0 && c;c=CMSG_NXTHDR(&message,c)) {
  if(c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_CREDENTIALS && c->cmsg_len==CMSG_LEN(sizeof(ucred)) && !seen) {
   ucred who{};std::memcpy(&who,CMSG_DATA(c),sizeof(who));seen=true;trusted=who.uid==m_ownerUid && who.gid==0 && who.pid==m_peerPid;
  }else {
   invalid=true;
   if(c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_RIGHTS && c->cmsg_len>=CMSG_LEN(0)) {
    const auto handles=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);const auto *fds=reinterpret_cast<const int*>(CMSG_DATA(c));
    for(size_t i=0;i<handles;++i)::close(fds[i]);
   }
  }
 }
 if(count<=0 || !trusted || invalid || expired()){fail("Audio actor packet refused");return;}
 QJsonParseError errorJson{};const auto document=QJsonDocument::fromJson(QByteArray(bytes,int(count)),&errorJson);
 if(errorJson.error!=QJsonParseError::NoError || !document.isObject() || expired()){fail("Audio reply refused");return;}
 const auto result=accept(document.object());
 if(result==Reply::Invalid || expired()){fail("Audio reply refused");return;}
 if(result==Reply::Done){m_pending.clear();m_stage=Stage::Idle;m_watch.stop();dispatchQueued();emit changed();}
}
void AudioClient::applyState(const QJsonObject &s) {
 m_available=s.value("connected").toBool() && s.value("available").toBool();
 m_requested=s.value("requested").toString();m_applied=s.value("applied").toString();
 if(!s.value("connected").toBool() || !s.value("available").toBool())m_status="Audio unavailable; reconnect to inspect";
 else if(s.value("pending").toBool())m_status=m_requested.isEmpty()?"Checking audio":"Applying output";
 else if(!m_applied.isEmpty())m_status="Output applied";
 else if(!m_requested.isEmpty())m_status="Media interrupted";
 else m_status=m_actorCanChoose?"Choose an output":"Media route unavailable";
}
AudioClient::Reply AudioClient::accept(const QJsonObject &reply) {
 const auto result=reply.value("result").toString();
 if(result=="denied" || result=="unavailable") {
  if(reply.size()!=1 || expired())return Reply::Invalid;
  pr_sounds_queue_cancel_v1(m_intents);m_actorCanChoose=false;m_actorCanReconnect=false;m_applied.clear();m_status=result=="denied"?"Media route denied":"Audio unavailable";return Reply::Done;
 }
 if(m_pending=="reconnect") {
  if(reply.size()!=1 || result!="ok" || expired())return Reply::Invalid;
  m_pending.clear();m_stage=Stage::Idle;statusRequest();return Reply::More;
 }
 if(m_pending=="route") {
  if(reply.size()!=2 || result!="media_selected" || !state(reply.value("state")) || expired())return Reply::Invalid;
  applyState(reply.value("state").toObject());return Reply::Done;
 }
 if(m_pending!="status" || result!="media_status" || reply.size()!=6 || !state(reply.value("state"))
    || !reply.value("routes").isArray() || !reply.value("can_choose").isBool() || !reply.value("can_reconnect").isBool())return Reply::Invalid;
 const auto page=reply.value("routes").toArray();
 if(page.size()>pageSize || m_offset!=m_pageRoutes.size() || m_offset+page.size()>maxRoutes)return Reply::Invalid;
 auto collected=m_pageRoutes;QString previous=collected.isEmpty()?QString{}:collected.last().toMap().value("id").toString();
 for(const auto &entry:page) {
  if(!entry.isObject())return Reply::Invalid;
  const auto route=entry.toObject();const auto id=route.value("id").toString(),text=route.value("label").toString();
  if(route.size()!=2 || !selector(id) || !label(text) || (!previous.isEmpty() && id<=previous))return Reply::Invalid;
  collected.append(QVariantMap{{"id",id},{"label",text}});previous=id;
 }
 const auto next=reply.value("next_offset");
 if(!next.isNull()) {
  if(!next.isDouble() || !std::isfinite(next.toDouble()) || next.toInteger(-1)!=m_offset+pageSize
     || next.toDouble()!=m_offset+pageSize || page.size()!=pageSize || m_offset+pageSize>=maxRoutes || expired())return Reply::Invalid;
  m_pageRoutes=collected;m_offset+=pageSize;
  return queue({{"op","media_status"},{"offset",m_offset}})?Reply::More:Reply::Invalid;
 }
 if(expired())return Reply::Invalid;
 m_routes=collected;m_pageRoutes.clear();m_actorCanChoose=reply.value("can_choose").toBool();m_actorCanReconnect=reply.value("can_reconnect").toBool();
 applyState(reply.value("state").toObject());return Reply::Done;
}
