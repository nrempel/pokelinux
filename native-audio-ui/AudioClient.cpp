#include "AudioClient.h"
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QRegularExpression>
#include <QSet>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>
#include <limits>

namespace {
constexpr qsizetype maxFrame=65536;
constexpr qint64 requestNs=1000000000;
qint64 integer(const QJsonValue &v) { return v.isDouble() ? v.toInteger(-1) : -1; }
bool selector(const QString &s) {
 static const QRegularExpression pattern(QStringLiteral("^[A-Za-z0-9_.-]{1,64}$"));
 return pattern.match(s).hasMatch();
}
bool displayLabel(const QString &s) {
 if(s.trimmed().isEmpty() || s.toUtf8().size()>96) return false;
 for(const QChar c:s) if(c.isNull() || c.category()==QChar::Other_Control) return false;
 return true;
}
}
qint64 AudioClient::boottime() {
 timespec ts{};
 if(clock_gettime(CLOCK_BOOTTIME,&ts)!=0 || ts.tv_sec<0 || ts.tv_sec>std::numeric_limits<qint64>::max()/1000000000) return -1;
 return qint64(ts.tv_sec)*1000000000+ts.tv_nsec;
}
bool AudioClient::expired() const { const auto now=boottime(); return now<0 || now>=m_deadline; }
AudioClient::AudioClient(QString path,uid_t ownerUid,QObject *parent)
 :QObject(parent),m_path(std::move(path)),m_ownerUid(ownerUid) {
 m_socket.setReadBufferSize(maxFrame+1);
 m_poll.setInterval(250);
 m_watch.setInterval(25);
 connect(&m_poll,&QTimer::timeout,this,[this]{ if(m_online && m_pending.isEmpty()) request({{"op","status"}},"status"); });
 connect(&m_watch,&QTimer::timeout,this,[this]{ if(!m_pending.isEmpty() && expired()) fail("Audio request expired; reconnect to inspect"); });
 connect(&m_socket,&QLocalSocket::connected,this,[this]{
  if(m_pending!="connect" || expired()) { fail("Audio connection expired"); return; }
  ucred peer{};socklen_t size=sizeof(peer);
  if(getsockopt(int(m_socket.socketDescriptor()),SOL_SOCKET,SO_PEERCRED,&peer,&size)!=0 || size!=sizeof(peer) || peer.uid!=m_ownerUid || peer.pid<=0) { fail("Audio service identity refused"); return; }
  m_pending.clear();m_online=true;request({{"op","status"}},"status");m_poll.start();
 });
 connect(&m_socket,&QLocalSocket::readyRead,this,&AudioClient::receive);
 connect(&m_socket,&QLocalSocket::disconnected,this,[this]{ if(!m_closing) fail("Audio disconnected; reconnect to inspect"); });
 connect(&m_socket,&QLocalSocket::errorOccurred,this,[this](QLocalSocket::LocalSocketError){ if(!m_closing) fail("Audio unavailable; reconnect to inspect"); });
 QTimer::singleShot(0,this,&AudioClient::reconnect);
}
AudioClient::~AudioClient() { m_closing=true;m_poll.stop();m_watch.stop();m_socket.abort(); }
void AudioClient::fail(const QString &message) {
 if(m_closing) return;
 m_closing=true;m_poll.stop();m_watch.stop();m_socket.abort();m_input.clear();m_pending.clear();
 m_online=false;m_available=false;m_lease=-1;m_intent=-1;m_epoch=-1;m_revision=-1;
 m_routes.clear();m_requested.clear();m_applied.clear();m_status=message;m_closing=false;emit changed();
}
void AudioClient::reconnect() {
 if(!canReconnect()) return;
 const auto now=boottime();if(now<0) { fail("Audio clock unavailable");return; }
 m_deadline=now+requestNs;m_pending="connect";m_status="Connecting to audio";m_watch.start();
 m_socket.connectToServer(m_path,QIODevice::ReadWrite);emit changed();
}
void AudioClient::request(const QJsonObject &message,const QString &kind) {
 if(!m_online || !m_pending.isEmpty()) return;
 const auto now=boottime();if(now<0) { fail("Audio clock unavailable");return; }
 const QByteArray bytes=QJsonDocument(message).toJson(QJsonDocument::Compact)+'\n';
 m_deadline=now+requestNs;m_pending=kind;m_watch.start();
 if(m_socket.write(bytes)!=bytes.size()) { fail("Audio request could not be sent");return; }
 emit changed();
}
void AudioClient::select(const QString &id) {
 if(!canChoose()) return;
 bool known=false;for(const auto &route:m_routes) if(route.toMap().value("id").toString()==id) known=true;
 if(!known) return;
 m_requested=id;m_applied.clear();m_status="Output requested";emit changed();
 if(m_lease<0) request({{"op","acquire"},{"role","media"},{"route",id}},"acquire");
 else request({{"op","route"},{"lease",m_lease},{"route",id}},"route");
}
void AudioClient::receive() {
 if(m_pending.isEmpty() || m_pending=="connect" || expired()) { fail("Audio reply arrived outside its request");return; }
 m_input+=m_socket.read(maxFrame+1-m_input.size());
 if(m_input.size()>maxFrame || m_socket.bytesAvailable()>0) { fail("Audio reply exceeded its limit");return; }
 const auto newline=m_input.indexOf('\n');if(newline<0) return;
 if(newline!=m_input.size()-1 || expired()) { fail("Audio reply framing refused");return; }
 QJsonParseError error{};const auto document=QJsonDocument::fromJson(m_input.left(newline),&error);
 if(error.error!=QJsonParseError::NoError || !document.isObject() || !accept(document.object()) || expired()) { fail("Audio reply refused");return; }
 m_input.clear();m_pending.clear();m_watch.stop();emit changed();
}
bool AudioClient::accept(const QJsonObject &reply) {
 if(!reply.value("ok").isBool()) return false;
 if(!reply.value("ok").toBool()) {
  if(!reply.value("error").isString()) return false;
  m_applied.clear();m_status="Audio request refused";return true;
 }
 if(integer(reply.value("peer_uid"))!=qint64(getuid()) || reply.value("phase").toString()!="running") return false;
 const auto application=reply.value("application").toString();
 if(application!="observed" && application!="pending" && application!="uncertain") return false;
 const auto intent=integer(reply.value("intent_revision"));if(intent<0 || intent<m_intent) return false;
 if(!reply.value("observed").isObject() || !reply.value("desired").isArray()) return false;
 const auto observed=reply.value("observed").toObject();const auto epoch=integer(observed.value("epoch")),revision=integer(observed.value("revision"));
 if(epoch<0 || revision<0 || epoch<m_epoch || (epoch==m_epoch && revision<m_revision) || !observed.value("available").isBool()) return false;
 QVariantList routes=m_routes;
 if(m_pending=="status") {
 if(!reply.value("media_routes").isArray()) return false;
 const auto catalog=reply.value("media_routes").toArray();if(catalog.size()>32) return false;
 routes.clear();QString previous;
 for(const auto &value:catalog) {
  if(!value.isObject()) return false;
  const auto route=value.toObject();const auto id=route.value("id").toString(),label=route.value("label").toString();
  if(route.size()!=2 || !selector(id) || !displayLabel(label) || (!previous.isEmpty() && id<=previous)) return false;
  routes.append(QVariantMap{{"id",id},{"label",label}});previous=id;
 }
 }
 const auto newLease=reply.value("lease");
 if(m_pending=="acquire") { const auto lease=integer(newLease);if(lease<=0) return false;m_lease=lease; }
 else if(!newLease.isNull() && !newLease.isUndefined()) return false;
 bool own=false;QSet<qint64> leases;
 for(const auto &value:reply.value("desired").toArray()) {
  if(!value.isArray()) return false;
  const auto entry=value.toArray();if(entry.size()!=3) return false;
  const auto lease=integer(entry[0]);const auto role=entry[1].toString(),route=entry[2].toString();
  if(lease<=0 || leases.contains(lease) || !selector(route) || (role!="media" && role!="call" && role!="capture")) return false;
  leases.insert(lease);if(lease==m_lease && role=="media" && route==m_requested) own=true;
 }
 m_routes=routes;m_intent=intent;m_epoch=epoch;m_revision=revision;
 m_available=observed.value("available").toBool() && application!="uncertain";
 m_applied.clear();
 if(!m_available) m_status="Audio unavailable; service recovery required";
 else if(own && application=="observed") { m_applied=m_requested;m_status="Output applied"; }
 else if(m_lease>0 && !own) m_status="Media interrupted";
 else if(m_lease>0) m_status="Applying output";
 else m_status="Choose an output";
 return true;
}
