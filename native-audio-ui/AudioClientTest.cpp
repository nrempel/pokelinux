#include "AudioClient.h"
#include <QtTest>
#include <QLocalServer>
#include <QJsonDocument>
#include <QJsonArray>
#include <QPointer>
#include <QTemporaryDir>
#include <QThread>
#include <functional>
#include <algorithm>
#include <memory>
#include <unistd.h>

namespace {
constexpr qint64 largeLease=9007199254740993LL;
struct Peer {
 QTemporaryDir dir;
 QLocalServer server;
 QPointer<QLocalSocket> socket;
 QByteArray input;
 QList<QJsonObject> requests;
 QJsonArray desired;
 qint64 intent=0,revision=0,epoch=1;
 bool available=true,observed=true;
 int connections=0,disconnects=0;
 std::function<bool(const QJsonObject &)> custom;
 Peer() {
  if(!server.listen(dir.path()+"/audio.sock")) qFatal("fixture listen failed");
  QObject::connect(&server,&QLocalServer::newConnection,&server,[this]{
   socket=server.nextPendingConnection();socket->setParent(&server);++connections;input.clear();
   QObject::connect(socket,&QLocalSocket::disconnected,&server,[this]{++disconnects;});
   QObject::connect(socket,&QLocalSocket::readyRead,&server,[this]{
    input+=socket->readAll();if(input.size()>4096) qFatal("request bound violated");
    while(input.contains('\n')) {
     const auto end=input.indexOf('\n');const auto document=QJsonDocument::fromJson(input.left(end));input.remove(0,end+1);
     if(!document.isObject()) qFatal("invalid client request");
     const auto q=document.object();requests.append(q);if(custom && custom(q)) continue;
     const auto op=q.value("op").toString();
     if(op=="acquire") { desired=QJsonArray{QJsonArray{largeLease,"media",q.value("route")}};++intent;auto r=reply(largeLease);r.remove("media_routes");send(r); }
     else if(op=="route") { desired=QJsonArray{QJsonArray{largeLease,"media",q.value("route")}};++intent;auto r=reply();r.remove("media_routes");send(r); }
     else if(op=="status") send(reply());
     else qFatal("unexpected operation");
    }
   });
  });
 }
 QJsonObject reply(qint64 lease=-1) {
  ++revision;
  return {{"ok",true},{"peer_uid",qint64(getuid())},{"phase","running"},
   {"application",!available?"uncertain":observed?"observed":"pending"},
   {"intent_revision",intent},{"lease",lease<0?QJsonValue(QJsonValue::Null):QJsonValue(lease)},
   {"media_routes",QJsonArray{QJsonObject{{"id","media-a7"},{"label","Speaker"}},QJsonObject{{"id","media-z2"},{"label","Headphones"}}}},
   {"desired",desired},{"observed",QJsonObject{{"epoch",epoch},{"revision",revision},{"available",available},{"links",QJsonArray{}}}}};
 }
 void send(const QJsonObject &value) { if(socket) socket->write(QJsonDocument(value).toJson(QJsonDocument::Compact)+'\n'); }
 int count(const QString &op) const { int n=0;for(const auto &r:requests) if(r.value("op")==op) ++n;return n; }
};
}
class AudioClientTest final:public QObject {
 Q_OBJECT
private slots:
 void authenticatedCatalog() {
  Peer peer;AudioClient client(peer.server.fullServerName(),getuid());
  QTRY_VERIFY_WITH_TIMEOUT(client.canChoose(),1500);
  QCOMPARE(peer.count("status"),1);QCOMPARE(client.routes().size(),2);QVERIFY(client.requested().isEmpty());QVERIFY(client.applied().isEmpty());
  QCOMPARE(client.routes()[0].toMap().value("id").toString(),QString("media-a7"));
  client.select("private.route.speaker");QTest::qWait(100);QCOMPARE(peer.count("acquire"),0);
 }
 void logicalAcquireWaitsForObservedApplication() {
  Peer peer;peer.observed=false;AudioClient client(peer.server.fullServerName(),getuid());
  QTRY_VERIFY_WITH_TIMEOUT(client.canChoose(),1500);client.select("media-a7");
  QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,1500);
  const auto request=*std::find_if(peer.requests.cbegin(),peer.requests.cend(),[](const auto &v){return v.value("op")=="acquire";});
  QCOMPARE(request.size(),3);QCOMPARE(request.value("role").toString(),QString("media"));QCOMPARE(request.value("route").toString(),QString("media-a7"));
  QTRY_VERIFY(client.canChoose());QVERIFY(client.applied().isEmpty());QCOMPARE(client.requested(),QString("media-a7"));
  peer.observed=true;QTRY_COMPARE(client.applied(),QString("media-a7"));
 }
 void routeUsesExactLeaseThenCallInterruptsAndRestores() {
  Peer peer;AudioClient client(peer.server.fullServerName(),getuid());QTRY_VERIFY(client.canChoose());client.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,1500);QTRY_COMPARE(client.applied(),QString("media-a7"));
  client.select("media-z2");QTRY_COMPARE(peer.count("route"),1);
  const auto request=*std::find_if(peer.requests.cbegin(),peer.requests.cend(),[](const auto &v){return v.value("op")=="route";});
  QCOMPARE(request.value("lease").toInteger(-1),largeLease);QCOMPARE(request.value("route").toString(),QString("media-z2"));QTRY_COMPARE(client.applied(),QString("media-z2"));
  peer.desired=QJsonArray{QJsonArray{77,"call","call-earpiece"}};++peer.intent;QTRY_COMPARE(client.status(),QString("Media interrupted"));QVERIFY(client.applied().isEmpty());QCOMPARE(client.requested(),QString("media-z2"));
  peer.desired=QJsonArray{QJsonArray{largeLease,"media","media-z2"}};++peer.intent;QTRY_COMPARE(client.applied(),QString("media-z2"));QCOMPARE(peer.count("acquire"),1);QCOMPARE(peer.count("route"),1);
 }
 void backendLossDisablesMutationsWithoutReplay() {
  Peer peer;AudioClient client(peer.server.fullServerName(),getuid());QTRY_VERIFY(client.canChoose());client.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,1500);QTRY_COMPARE(client.applied(),QString("media-a7"));
  peer.available=false;QTRY_VERIFY(!client.canChoose());QVERIFY(client.applied().isEmpty());client.select("media-z2");QTest::qWait(400);QCOMPARE(peer.count("route"),0);QCOMPARE(peer.count("acquire"),1);
  peer.available=true;++peer.epoch;peer.revision=0;QTRY_COMPARE(client.applied(),QString("media-a7"));QCOMPARE(peer.count("acquire"),1);
 }
 void connectionLossNeedsExplicitReconnectAndNeverReplays() {
  Peer peer;peer.custom=[&](const QJsonObject &q){if(q.value("op")=="acquire") {peer.socket->abort();return true;}return false;};
  AudioClient client(peer.server.fullServerName(),getuid());QTRY_VERIFY(client.canChoose());client.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,1500);QTRY_VERIFY(client.canReconnect());QVERIFY(client.requested().isEmpty());QVERIFY(client.applied().isEmpty());
  QTest::qWait(400);QCOMPARE(peer.connections,1);client.reconnect();QTRY_COMPARE(peer.connections,2);QTRY_VERIFY(client.canChoose());QTest::qWait(300);QCOMPARE(peer.count("acquire"),1);QVERIFY(client.requested().isEmpty());
 }
 void destructorClosesTheLeaseConnection() {
  Peer peer;auto client=std::make_unique<AudioClient>(peer.server.fullServerName(),getuid());QTRY_VERIFY(client->canChoose());client->select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,1500);QTRY_VERIFY(client->canChoose());client.reset();QTRY_COMPARE(peer.disconnects,1);
 }
 void wrongOwnerSendsNothing() {
  Peer peer;AudioClient client(peer.server.fullServerName(),getuid()+1);QTRY_COMPARE(peer.connections,1);QTRY_COMPARE(peer.disconnects,1);QTRY_VERIFY(client.canReconnect());QVERIFY(!client.canChoose());QCOMPARE(peer.requests.size(),0);
 }
 void lateMutationReplyAfterBlockedEventLoopIsRefused() {
  Peer peer;peer.custom=[&](const QJsonObject &q){
   if(q.value("op")!="acquire") return false;
   QThread::msleep(1200);peer.desired=QJsonArray{QJsonArray{largeLease,"media","media-a7"}};++peer.intent;auto r=peer.reply(largeLease);r.remove("media_routes");peer.send(r);return true;
  };
  AudioClient client(peer.server.fullServerName(),getuid());QTRY_VERIFY(client.canChoose());client.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(peer.count("acquire"),1,2500);QTRY_VERIFY(client.canReconnect());QVERIFY(client.applied().isEmpty());QVERIFY(client.requested().isEmpty());QTest::qWait(300);QCOMPARE(peer.connections,1);
 }
 void oversizedReplyClosesConnection() {
  Peer peer;peer.custom=[&](const QJsonObject &){peer.socket->write(QByteArray(65537,' '));return true;};AudioClient client(peer.server.fullServerName(),getuid());QTRY_COMPARE(peer.connections,1);QTRY_COMPARE(peer.disconnects,1);QTRY_VERIFY(client.canReconnect());QVERIFY(client.routes().isEmpty());
 }
 void catalogWithPrivateTargetIsRefused() {
  Peer peer;peer.custom=[&](const QJsonObject &){auto r=peer.reply();r["media_routes"]=QJsonArray{QJsonObject{{"id","media-a7"},{"label","Speaker"},{"target","private.speaker"}}};peer.send(r);return true;};
  AudioClient client(peer.server.fullServerName(),getuid());QTRY_COMPARE(peer.connections,1);QTRY_COMPARE(peer.disconnects,1);QTRY_VERIFY(client.canReconnect());QVERIFY(client.routes().isEmpty());
 }
 void decreasingGraphObservationIsRefused() {
  Peer peer;AudioClient client(peer.server.fullServerName(),getuid());QTRY_VERIFY(client.canChoose());peer.epoch=0;QTRY_VERIFY(client.canReconnect());QVERIFY(client.routes().isEmpty());
 }
};
QTEST_GUILESS_MAIN(AudioClientTest)
#include "AudioClientTest.moc"
