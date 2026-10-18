#include "AudioClient.h"
#include <QtTest>
#include <QJsonDocument>
#include <QJsonArray>
#include <QSocketNotifier>
#include <QTemporaryDir>
#include <QThread>
#include <functional>
#include <memory>
#include <cerrno>
#include <cstring>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <unistd.h>

namespace {
QJsonObject emptyState() {
 return {{"connected",true},{"available",true},{"pending",false},
         {"requested",QJsonValue::Null},{"applied",QJsonValue::Null},{"policy_settled",false},{"preview","idle"}};
}
QJsonArray catalog(int count=2) {
 if(count==2) return {QJsonObject{{"id","media-a7"},{"label","Speaker"}},QJsonObject{{"id","media-z2"},{"label","Headphones"}}};
 QJsonArray values;for(int i=0;i<count;++i) values.append(QJsonObject{{"id",QString("route-%1").arg(i,2,10,QChar('0'))},{"label",QString("Output %1").arg(i)}});return values;
}
struct Peer {
 QTemporaryDir dir;QObject context;
 int listener=-1,fd=-1,connections=0,disconnects=0;
 QSocketNotifier *acceptor=nullptr,*reader=nullptr;
 QJsonObject state=emptyState();QJsonArray routes=catalog();
 bool choose=true,reconnect=false,credentials=false;
 QList<QJsonObject> requests;
 std::function<bool(const QJsonObject &)> custom;
 QString path() const { return dir.path()+"/app.sock"; }
 Peer() {
  listener=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);
  int one=1;sockaddr_un address{};address.sun_family=AF_UNIX;const auto encoded=path().toLocal8Bit();std::memcpy(address.sun_path,encoded.constData(),size_t(encoded.size())+1);
  if(listener<0 || ::setsockopt(listener,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one)) || ::bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address)) || ::listen(listener,8))qFatal("SEQPACKET fixture setup failed");
  acceptor=new QSocketNotifier(listener,QSocketNotifier::Read,&context);
  QObject::connect(acceptor,&QSocketNotifier::activated,&context,[this]{
   const int accepted=::accept4(listener,nullptr,nullptr,SOCK_NONBLOCK|SOCK_CLOEXEC);if(accepted<0)return;
   closeClient();fd=accepted;++connections;int enabled=1;if(::setsockopt(fd,SOL_SOCKET,SO_PASSCRED,&enabled,sizeof(enabled)))qFatal("fixture passcred");
   reader=new QSocketNotifier(fd,QSocketNotifier::Read,&context);
   QObject::connect(reader,&QSocketNotifier::activated,&context,[this]{receive();});
  });
 }
 ~Peer(){closeClient();delete acceptor;if(listener>=0)::close(listener);}
 void closeClient(){if(reader){reader->setEnabled(false);reader->deleteLater();reader=nullptr;}if(fd>=0){::close(fd);fd=-1;}}
 void receive(){
  char bytes[4097];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(16*sizeof(int))]{};
  iovec io{bytes,sizeof(bytes)};msghdr msg{};msg.msg_iov=&io;msg.msg_iovlen=1;msg.msg_control=control;msg.msg_controllen=sizeof(control);
  const auto size=::recvmsg(fd,&msg,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);
  if(size<0 && (errno==EAGAIN || errno==EINTR))return;
  if(size<0 && errno==ECONNRESET){++disconnects;closeClient();return;}
  if(size<0)qFatal("fixture recvmsg failed");
  if(size==0){++disconnects;closeClient();return;}
  bool trusted=false;int credentialCount=0;
  for(auto *c=CMSG_FIRSTHDR(&msg);c;c=CMSG_NXTHDR(&msg,c)) {
   if(c->cmsg_level!=SOL_SOCKET || c->cmsg_type!=SCM_CREDENTIALS || c->cmsg_len!=CMSG_LEN(sizeof(ucred)))qFatal("unexpected client ancillary");
   ucred who{};std::memcpy(&who,CMSG_DATA(c),sizeof(who));trusted=who.pid==::getpid()&&who.uid==::getuid()&&who.gid==::getgid();++credentialCount;
  }
  if(size<=0 || size>4096 || msg.msg_flags&(MSG_TRUNC|MSG_CTRUNC) || !trusted || credentialCount!=1)qFatal("actual client credentials or bound failed");
  credentials=true;QJsonParseError error{};const auto document=QJsonDocument::fromJson(QByteArray(bytes,int(size)),&error);
  if(error.error!=QJsonParseError::NoError || !document.isObject())qFatal("invalid client packet");
  const auto q=document.object();requests.append(q);if(custom && custom(q))return;
  const auto op=q.value("op").toString();
  if(op=="media_status") {
   if(q.size()!=2 || !q.value("offset").isDouble())qFatal("status shape");
   send(status(q.value("offset").toInteger(-1)));
  }else if(op=="media_route") {
   const auto request=q.value("request").toObject();
   if(q.size()!=2 || request.size()!=2 || request.value("ttl_ms").toInteger()!=1000 || !request.value("route").isString())qFatal("route shape");
   state["requested"]=request.value("route");state["applied"]=request.value("route");state["pending"]=false;send({{"result","media_selected"},{"state",state}});
  }else if(op=="media_reconnect") {
   if(q.size()!=1)qFatal("reconnect shape");
   state=emptyState();choose=true;reconnect=false;send({{"result","ok"}});
  }else qFatal("legacy or unexpected operation");
 }
 QJsonObject status(qint64 offset=0)const {
  if(offset<0 || offset>routes.size())qFatal("offset bound");
  QJsonArray page;for(qint64 i=offset;i<routes.size()&&i<offset+8;++i)page.append(routes[int(i)]);
  const QJsonValue next=offset+page.size()<routes.size()?QJsonValue(offset+8):QJsonValue(QJsonValue::Null);
  return {{"result","media_status"},{"state",state},{"routes",page},{"next_offset",next},{"can_choose",choose},{"can_reconnect",reconnect}};
 }
 void raw(const QByteArray &wire,int passed=-1){
  if(fd<0)qFatal("fixture absent connection");
  ssize_t sent;
  if(passed<0)sent=::send(fd,wire.constData(),size_t(wire.size()),MSG_DONTWAIT|MSG_NOSIGNAL);
  else {alignas(cmsghdr) char control[CMSG_SPACE(sizeof(int))]{};iovec io{const_cast<char*>(wire.constData()),size_t(wire.size())};msghdr msg{};msg.msg_iov=&io;msg.msg_iovlen=1;msg.msg_control=control;msg.msg_controllen=sizeof(control);auto *c=CMSG_FIRSTHDR(&msg);c->cmsg_level=SOL_SOCKET;c->cmsg_type=SCM_RIGHTS;c->cmsg_len=CMSG_LEN(sizeof(int));std::memcpy(CMSG_DATA(c),&passed,sizeof(passed));sent=::sendmsg(fd,&msg,MSG_DONTWAIT|MSG_NOSIGNAL);}
  if(sent!=wire.size())qFatal("fixture actual send failed");
 }
 void send(const QJsonObject &value,int passed=-1){raw(QJsonDocument(value).toJson(QJsonDocument::Compact),passed);}
 int count(const QString &op)const {int count=0;for(const auto &q:requests)if(q.value("op").toString()==op)++count;return count;}
};
}
class AudioClientTest final:public QObject {
 Q_OBJECT
private slots:
 void initTestCase(){QVERIFY2(::getuid()==0&&::getgid()==0,"actual root actor peer required for this fixed socket suite");}
 void authenticatedPaginatedCatalog(){
  Peer p;p.routes=catalog(10);AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QCOMPARE(c.routes().size(),10);QVERIFY(p.credentials);QCOMPARE(p.count("media_status"),2);QCOMPARE(p.requests[0].value("offset").toInteger(),0);QCOMPARE(p.requests[1].value("offset").toInteger(),8);c.select("private.sink");QTest::qWait(50);QCOMPARE(p.count("media_route"),0);
 }
 void grantDeniedCannotChoose(){
  Peer p;p.choose=false;AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QVERIFY(!c.canChoose());c.select("media-a7");QTest::qWait(100);QCOMPARE(p.count("media_route"),0);QVERIFY(c.requested().isEmpty());QVERIFY(c.applied().isEmpty());
 }
 void acceptedPendingThenObserved(){
  Peer p;p.custom=[&](const QJsonObject&q){if(q.value("op")!="media_route")return false;p.state["requested"]=q.value("request").toObject().value("route");p.state["pending"]=true;p.send({{"result","media_selected"},{"state",p.state}});return true;};
  AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,1500);QTRY_COMPARE_WITH_TIMEOUT(c.requested(),QString("media-a7"),1500);QVERIFY(c.applied().isEmpty());QCOMPARE(c.status(),QString("Applying output"));const auto q=p.requests.last();QCOMPARE(q.size(),2);QCOMPARE(q.value("request").toObject(),(QJsonObject{{"route","media-a7"},{"ttl_ms",1000}}));p.state["pending"]=false;p.state["applied"]="media-a7";QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-a7"),1500);
 }
 void actorInterruptionAndRestoration(){
  Peer p;AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-z2");QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-z2"),1500);p.state["applied"]=QJsonValue::Null;QTRY_COMPARE_WITH_TIMEOUT(c.status(),QString("Media interrupted"),1500);QCOMPARE(c.requested(),QString("media-z2"));p.state["applied"]="media-z2";QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-z2"),1500);QCOMPARE(p.count("media_route"),1);
 }
 void foregroundObservationDisablesChoice(){
  Peer p;AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);p.choose=false;QTRY_VERIFY_WITH_TIMEOUT(!c.canChoose(),1500);c.select("media-a7");QTest::qWait(100);QCOMPARE(p.count("media_route"),0);
 }
 void explicitProviderReconnectNeverReplays(){
  Peer p;AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-a7"),1500);p.state=emptyState();p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(!c.canChoose());c.reconnect();QTRY_COMPARE_WITH_TIMEOUT(p.count("media_reconnect"),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QVERIFY(c.requested().isEmpty());QVERIFY(c.applied().isEmpty());QTest::qWait(300);QCOMPARE(p.count("media_route"),1);QCOMPARE(p.connections,1);
 }
 void transportReconnectOnlyInspects(){
  Peer p;AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-a7"),1500);p.closeClient();p.state=emptyState();QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QTest::qWait(300);QCOMPARE(p.connections,1);c.reconnect();QTRY_COMPARE_WITH_TIMEOUT(p.connections,2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QVERIFY(c.requested().isEmpty());QCOMPARE(p.count("media_route"),1);QCOMPARE(p.count("media_reconnect"),0);
 }
 void denialDoesNotClaimApplication(){
  Peer p;p.custom=[&](const QJsonObject&q){if(q.value("op")!="media_route")return false;p.choose=false;p.send({{"result","denied"}});return true;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,1500);QTRY_VERIFY_WITH_TIMEOUT(!c.canChoose(),1500);QVERIFY(c.applied().isEmpty());QVERIFY(c.requested().isEmpty());QCOMPARE(p.connections,1);
 }
 void destructorClosesActorConnection(){
  Peer p;auto c=std::make_unique<AudioClient>(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c->canChoose(),1500);c.reset();QTRY_COMPARE_WITH_TIMEOUT(p.disconnects,1,1500);
 }
 void wrongPeerOwnerSendsNothing(){
  Peer p;AudioClient c(p.path(),1);QTRY_COMPARE_WITH_TIMEOUT(p.connections,1,1500);QTRY_COMPARE_WITH_TIMEOUT(p.disconnects,1,1500);QVERIFY(p.requests.isEmpty());QVERIFY(c.canReconnect());QVERIFY(!c.canChoose());
 }
 void actualReceivedDescriptorIsClosedAndRefused(){
  Peer p;int pipefd[2];QVERIFY(::pipe2(pipefd,O_CLOEXEC|O_NONBLOCK)==0);p.custom=[&](const QJsonObject&){p.send(p.status(),pipefd[1]);::close(pipefd[1]);pipefd[1]=-1;return true;};AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(p.requests.size(),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());char byte;QTRY_VERIFY_WITH_TIMEOUT(::read(pipefd[0],&byte,1)==0,1500);::close(pipefd[0]);if(pipefd[1]>=0)::close(pipefd[1]);
 }
 void actualDifferentSenderPidIsRefused(){
  Peer p;bool reaped=false;p.custom=[&](const QJsonObject&){const auto wire=QJsonDocument(p.status()).toJson(QJsonDocument::Compact);const auto child=::fork();if(child<0)qFatal("fixture fork");if(child==0){const auto n=::send(p.fd,wire.constData(),size_t(wire.size()),MSG_DONTWAIT|MSG_NOSIGNAL);_exit(n==wire.size()?0:71);}int status=0;if(::waitpid(child,&status,0)!=child||!WIFEXITED(status)||WEXITSTATUS(status)!=0)qFatal("actual sender child wait");reaped=true;return true;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(reaped,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());
 }
 void actualTruncatedPacketIsRefused(){
  Peer p;p.custom=[&](const QJsonObject&){p.raw(QByteArray(4097,' '));return true;};AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(p.requests.size(),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());
 }
 void malformedPageOffsetIsRefused(){
  Peer p;p.routes=catalog(10);p.custom=[&](const QJsonObject&){auto value=p.status();value["next_offset"]=7;p.send(value);return true;};AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(p.requests.size(),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());
 }
 void privateTargetInCatalogIsRefused(){
  Peer p;p.custom=[&](const QJsonObject&){auto value=p.status();value["routes"]=QJsonArray{QJsonObject{{"id","media-a7"},{"label","Speaker"},{"target","private.sink"}}};p.send(value);return true;};AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(p.requests.size(),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());
 }
 void lateMutationAfterBlockedEventLoopIsRefused(){
  Peer p;p.custom=[&](const QJsonObject&q){if(q.value("op")!="media_route")return false;QThread::msleep(1200);p.state["requested"]="media-a7";p.state["applied"]="media-a7";p.send({{"result","media_selected"},{"state",p.state}});return true;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,2500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.applied().isEmpty());QVERIFY(c.requested().isEmpty());QCOMPARE(p.connections,1);
 }
 void paginationCannotExtendOriginalDeadline(){
  Peer p;p.routes=catalog(10);p.custom=[&](const QJsonObject&q){QThread::msleep(600);p.send(p.status(q.value("offset").toInteger()));return true;};AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(p.count("media_status"),2,2500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.routes().isEmpty());QVERIFY(c.applied().isEmpty());
 }
 void readonlyGestureQueuesOneRouteWithRemainingTtl(){
  Peer p;bool hold=false,held=false;qint64 ttl=0;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}if(q.value("op")=="media_route"){ttl=q.value("request").toObject().value("ttl_ms").toInteger();p.state["requested"]=q.value("request").toObject().value("route");p.state["applied"]=p.state["requested"];p.send({{"result","media_selected"},{"state",p.state}});return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());QTest::qWait(120);QVERIFY(c.canChoose());c.select("media-a7");c.select("media-z2");QVERIFY(!c.canChoose());QCOMPARE(p.count("media_route"),0);QTest::qWait(250);hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,1500);QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-a7"),1500);QVERIFY(ttl>0&&ttl<900);QTest::qWait(300);QCOMPARE(p.count("media_route"),1);
 }
 void readonlyQueuePreservesAppliedUntilSend(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}if(hold==false&&q.value("op")=="media_route"&&q.value("request").toObject().value("ttl_ms").toInteger()<1000){p.state["requested"]=q.value("request").toObject().value("route");p.state["applied"]=p.state["requested"];p.send({{"result","media_selected"},{"state",p.state}});return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);c.select("media-a7");QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-a7"),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-z2");c.select("media-a7");QCOMPARE(c.applied(),QString("media-a7"));QCOMPARE(p.count("media_route"),1);hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),2,1500);QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("media-z2"),1500);QTest::qWait(300);QCOMPARE(p.count("media_route"),2);
 }
 void queuedChoiceCancelledByFreshPermissionLoss(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-a7");p.choose=false;hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(c.status(),QString("Media route unavailable"),1500);QTest::qWait(300);QCOMPARE(p.count("media_route"),0);QVERIFY(!c.canChoose());
 }
 void queuedChoiceRechecksFreshCatalog(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-z2");p.routes=QJsonArray{QJsonObject{{"id","media-a7"},{"label","Speaker"}}};p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),1,1500);QCOMPARE(p.count("media_route"),0);p.closeClient();
 }
 void queuedChoiceExpiresWithoutSend(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-a7");QTest::qWait(1150);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=false;c.reconnect();QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QTest::qWait(300);QCOMPARE(p.count("media_route"),0);QVERIFY(c.requested().isEmpty());
 }
 void queuedChoiceDisconnectedNeverReplays(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-a7");p.closeClient();QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=false;c.reconnect();QTRY_COMPARE_WITH_TIMEOUT(p.connections,2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QTest::qWait(300);QCOMPARE(p.count("media_route"),0);QCOMPARE(p.count("media_reconnect"),0);
 }
 void queuedEffectReplyKeepsOriginalInputDeadline(){
  Peer p;bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}if(q.value("op")=="media_route"){QThread::msleep(600);p.state["requested"]="media-a7";p.state["applied"]="media-a7";p.send({{"result","media_selected"},{"state",p.state}});return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("media-a7");QTest::qWait(600);hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,2500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QVERIFY(c.applied().isEmpty());QTest::qWait(300);QCOMPARE(p.count("media_route"),1);QCOMPARE(p.connections,1);
 }
 void readonlyPaginationQueuesUntilFinalFreshPage(){
  Peer p;p.routes=catalog(10);bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"&&q.value("offset").toInteger()==8){held=true;return true;}if(q.value("op")=="media_route"){p.state["requested"]=q.value("request").toObject().value("route");p.state["applied"]=p.state["requested"];p.send({{"result","media_selected"},{"state",p.state}});return true;}return false;};AudioClient c(p.path(),0);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);QCOMPARE(c.routes().size(),10);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canChoose());c.select("route-09");QCOMPARE(p.count("media_route"),0);hold=false;p.send(p.status(8));QTRY_COMPARE_WITH_TIMEOUT(p.count("media_route"),1,1500);QTRY_COMPARE_WITH_TIMEOUT(c.applied(),QString("route-09"),1500);QTest::qWait(300);QCOMPARE(p.count("media_route"),1);
 }


 void readonlyReconnectGestureQueuesOneExplicitReconnect(){
  Peer p;p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;
  bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};
  AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);
  QVERIFY(c.canReconnect());QTest::qWait(120);QVERIFY(c.canReconnect());c.reconnect();c.reconnect();QVERIFY(!c.canReconnect());QCOMPARE(p.count("media_reconnect"),0);
  hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(p.count("media_reconnect"),1,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canChoose(),1500);
  QTest::qWait(300);QCOMPARE(p.count("media_reconnect"),1);QCOMPARE(p.count("media_route"),0);QCOMPARE(p.connections,1);
 }
 void readonlyReconnectFreshPermissionLossCancels(){
  Peer p;p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;
  bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};
  AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canReconnect());c.reconnect();
  p.reconnect=false;hold=false;p.send(p.status());QTest::qWait(350);QVERIFY(!c.canReconnect());QVERIFY(!c.canChoose());QCOMPARE(p.count("media_reconnect"),0);QCOMPARE(p.count("media_route"),0);
 }
 void readonlyReconnectExpiresWithoutEffectOrReplay(){
  Peer p;p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;
  bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};
  AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canReconnect());c.reconnect();
  QTest::qWait(1150);QTRY_COMPARE_WITH_TIMEOUT(p.disconnects,1,1500);QVERIFY(c.canReconnect());QCOMPARE(p.count("media_reconnect"),0);
  hold=false;c.reconnect();QTRY_COMPARE_WITH_TIMEOUT(p.connections,2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QTest::qWait(300);QCOMPARE(p.count("media_reconnect"),0);QCOMPARE(p.count("media_route"),0);
 }
 void readonlyReconnectDisconnectNeverReplays(){
  Peer p;p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;
  bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}return false;};
  AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canReconnect());c.reconnect();
  p.closeClient();QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=false;c.reconnect();QTRY_COMPARE_WITH_TIMEOUT(p.connections,2,1500);
  QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);QTest::qWait(300);QCOMPARE(p.count("media_reconnect"),0);QCOMPARE(p.count("media_route"),0);
 }
 void readonlyReconnectReplyKeepsOriginalInputDeadline(){
  Peer p;p.state["connected"]=false;p.state["available"]=false;p.choose=false;p.reconnect=true;
  bool hold=false,held=false;p.custom=[&](const QJsonObject&q){if(hold&&q.value("op")=="media_status"){held=true;return true;}if(q.value("op")=="media_reconnect"){QThread::msleep(600);p.state=emptyState();p.choose=true;p.reconnect=false;p.send({{"result","ok"}});return true;}return false;};
  AudioClient c(p.path(),0);QTRY_COMPARE_WITH_TIMEOUT(c.routes().size(),2,1500);QTRY_VERIFY_WITH_TIMEOUT(c.canReconnect(),1500);hold=true;QTRY_VERIFY_WITH_TIMEOUT(held,1500);QVERIFY(c.canReconnect());c.reconnect();
  QTest::qWait(600);hold=false;p.send(p.status());QTRY_COMPARE_WITH_TIMEOUT(p.count("media_reconnect"),1,2500);QTRY_COMPARE_WITH_TIMEOUT(p.disconnects,1,1500);
  QVERIFY(c.canReconnect());QVERIFY(!c.canChoose());QVERIFY(c.applied().isEmpty());QTest::qWait(300);QCOMPARE(p.count("media_reconnect"),1);QCOMPARE(p.count("media_route"),0);QCOMPARE(p.connections,1);
 }

};
QTEST_GUILESS_MAIN(AudioClientTest)
#include "AudioClientTest.moc"
