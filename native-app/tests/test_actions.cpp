#include <QtTest>
#include <QQmlEngine>
#include <QJSValue>
#include <QQmlComponent>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include "PhoneRpc.h"

class FakeRpc final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool busy MEMBER busy NOTIFY busyChanged)
public:
    bool busy=false;
    double now=1000;
    QList<QJsonObject> requests;
    QString token;
    Q_INVOKABLE QString newId() { return QString::number(++sequence,16).rightJustified(32,'0'); }
    Q_INVOKABLE double boottimeMs() const { return now; }
    Q_INVOKABLE bool request(QString,QString frame,QString caller) {
        if(busy) return false;
        busy=true; token=caller;
        requests.append(QJsonDocument::fromJson(frame.toUtf8()).object());
        emit busyChanged(); return true;
    }
    void reply(QJsonObject result,int status=0) {
        const auto id=requests.last()["id"];
        const auto caller=token; busy=false; emit busyChanged();
        emit completed(status,QJsonDocument(QJsonObject{{"v",1},{"id",id},{"result",result}}).toJson(QJsonDocument::Compact),caller);
    }
signals:
    void busyChanged();
    void completed(int status,QByteArray bytes,QString token);
private:
    int sequence=0;
};
class ActionsTest : public QObject {
    Q_OBJECT
private:
    QQmlEngine engine;
    FakeRpc rpc;
    QScopedPointer<QObject> session;
    QScopedPointer<QObject> actions;
    QVariant call(const char *method,QVariant a=QVariant(),QVariant b=QVariant()) {
        QVariant result;
        bool ok;
        if(!a.isValid()) ok=QMetaObject::invokeMethod(actions.data(),method,Q_RETURN_ARG(QVariant,result));
        else if(!b.isValid()) ok=QMetaObject::invokeMethod(actions.data(),method,Q_RETURN_ARG(QVariant,result),Q_ARG(QVariant,a));
        else ok=QMetaObject::invokeMethod(actions.data(),method,Q_RETURN_ARG(QVariant,result),Q_ARG(QVariant,a),Q_ARG(QVariant,b));
        if(!ok) qFatal("QML method unavailable");
        return result;
    }
    QJsonObject controlSnapshot() {
        QJsonObject call{{"id",1},{"state",2},{"actions",QJsonArray{"answer","reject"}}};
        QJsonObject snapshot{{"generation",QString(32,'b')},{"revision",7},{"controls_allowed",true},{"availability","observed"},{"calls",QJsonArray{call}}};
        return {{"kind","controls"},{"snapshot",snapshot}};
    }
    void controls() {
        QVERIFY(call("observe",QString("controls")).toBool());
        rpc.reply(controlSnapshot());
    }
private slots:
    void init() {
        rpc.busy=false; rpc.now=1000; rpc.requests.clear();
        QQmlComponent sc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        session.reset(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath","/tmp/test.sock"}})); QVERIFY(session);
        session->setProperty("epoch",QString(32,'a'));
        session->setProperty("sessionToken",QString(32,'c'));
        session->setProperty("admissionToken",QString(32,'d'));
        session->setProperty("phase","ready");
        QQmlComponent ac(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QVERIFY2(ac.isReady(),qPrintable(ac.errorString()));
        actions.reset(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
    }
    void cleanup() { actions.reset(); session.reset(); }
    void freshControlsRemainVisibleDuringReadButFailClosed() {
        controls();
        QVERIFY(call("observe",QString("controls")).toBool());
        QVERIFY(actions->property("controlsSnapshot").value<QJSValue>().isObject());
        QVERIFY(!call("control",1,QString("answer")).toBool()); // adapter owns the read
        rpc.reply({},4);
        QVERIFY(!actions->property("controlsSnapshot").value<QJSValue>().isObject());
    }
    void automaticObservationsAlternateAndStop() {
        QQmlComponent oc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneObservations.qml")));
        QVERIFY2(oc.isReady(),qPrintable(oc.errorString()));
        QScopedPointer<QObject> observations(oc.createWithInitialProperties({{"actions",QVariant::fromValue(actions.data())},{"session",QVariant::fromValue(session.data())},{"active",true}}));
        QVERIFY2(observations,qPrintable(oc.errorString()));
        QTRY_COMPARE_WITH_TIMEOUT(rpc.requests.size(),1,1000);
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("controls"));
        rpc.reply(controlSnapshot());
        QTRY_COMPARE_WITH_TIMEOUT(rpc.requests.size(),2,1000);
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("dial_status"));
        rpc.reply({{"kind","dial_status"},{"snapshot",QJsonObject{{"generation",QString(32,'b')},{"can_dial",false},{"offer",""},{"pending",false},{"reason","busy"}}}});
        QVERIFY(call("control",1,QString("answer")).toBool());
        rpc.reply({},3);
        const int before=rpc.requests.size();
        QTest::qWait(600); QCOMPARE(rpc.requests.size(),before);
        observations->setProperty("active",false);
        QVERIFY(!actions->property("controlsSnapshot").value<QJSValue>().isObject());
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
    }
    void inactiveObservationCannotReturn_data() {
        QTest::addColumn<bool>("sleep");
        QTest::newRow("hidden") << false;
        QTest::newRow("sleep") << true;
    }
    void inactiveObservationCannotReturn() {
        QFETCH(bool,sleep);
        QQmlComponent oc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneObservations.qml")));
        QVERIFY2(oc.isReady(),qPrintable(oc.errorString()));
        QScopedPointer<QObject> observations(oc.createWithInitialProperties({{"actions",QVariant::fromValue(actions.data())},{"session",QVariant::fromValue(session.data())},{"active",true}}));
        QVERIFY(observations);
        QTRY_COMPARE_WITH_TIMEOUT(rpc.requests.size(),1,1000);
        if(sleep) session->setProperty("suspended",true);
        else observations->setProperty("active",false);
        rpc.reply(controlSnapshot());
        QTest::qWait(600);
        QCOMPARE(rpc.requests.size(),1);
        QVERIFY(!actions->property("controlsSnapshot").value<QJSValue>().isObject());
        QVERIFY(!call("control",1,QString("answer")).toBool());
    }
    void dialReadPreservesIncomingControls_data() {
        QTest::addColumn<bool>("failure");
        QTest::newRow("dial-unavailable") << false;
        QTest::newRow("dial-read-failed") << true;
    }
    void dialReadPreservesIncomingControls() {
        QFETCH(bool,failure);
        controls();
        QVERIFY(call("observe",QString("dial_status")).toBool());
        if(failure) rpc.reply({},4);
        else rpc.reply({{"kind","dial_status"},{"snapshot",QJsonObject{{"generation",QString(32,'b')},{"can_dial",false},{"offer",""},{"pending",false},{"reason","busy"}}}});
        QVERIFY(call("control",1,QString("answer")).toBool());
        QCOMPARE(rpc.requests.last()["command"].toObject()["revision"].toInt(),7);
    }
    void observationAgesAreIndependent() {
        controls(); rpc.now+=1500;
        QVERIFY(call("observe",QString("dial_status")).toBool());
        rpc.reply({{"kind","dial_status"},{"snapshot",QJsonObject{{"generation",QString(32,'b')},{"can_dial",true},{"offer",QString(32,'e')},{"pending",false},{"reason","ready"}}}});
        rpc.now+=600;
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QVERIFY(call("dial",QString("5550100")).toBool());
    }
    void newAndroidGenerationInvalidatesOtherObservation() {
        controls();
        QVERIFY(call("observe",QString("dial_status")).toBool());
        rpc.reply({{"kind","dial_status"},{"snapshot",QJsonObject{{"generation",QString(32,'9')},{"can_dial",true},{"offer",QString(32,'e')},{"pending",false},{"reason","ready"}}}});
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QVERIFY(call("dial",QString("5550100")).toBool());
    }
    void changedServiceIdentityInvalidatesControls() {
        controls();
        QVERIFY(session->metaObject()->indexOfMethod("checkHealth()") >= 0);
        QVariant accepted;
        QVERIFY(QMetaObject::invokeMethod(session.data(),"checkHealth",Q_RETURN_ARG(QVariant,accepted)));
        QVERIFY(accepted.toBool());
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("hello"));
        rpc.reply({{"kind","hello"},{"epoch",QString(32,'a')},{"session",QString(32,'f')}});
        QVERIFY(!session->property("ready").toBool());
        QVERIFY(session->property("admissionToken").toString().isEmpty());
        QVERIFY(!call("control",1,QString("answer")).toBool());
    }
    void lifecycleRecoversPendingIntentAfterWake() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({{"kind","submission"},{"outcome","pending"}});
        QQmlComponent pc(&engine); pc.setData("import QtQml; QtObject { signal changed(); property bool available: false; property bool preparing: true }",QUrl());
        QScopedPointer<QObject> power(pc.create()); QVERIFY(power);
        QQmlComponent lc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneLifecycle.qml")));
        QVERIFY2(lc.isReady(),qPrintable(lc.errorString()));
        QScopedPointer<QObject> lifecycle(lc.createWithInitialProperties({{"power",QVariant::fromValue(power.data())},{"session",QVariant::fromValue(session.data())},{"actions",QVariant::fromValue(actions.data())},{"rpc",QVariant::fromValue(&rpc)}}));
        QVERIFY2(lifecycle,qPrintable(lc.errorString()));
        QVERIFY(session->property("suspended").toBool());
        QCOMPARE(actions->property("phase").toString(),QString("pending"));
        QCOMPARE(actions->property("intentId").toString(),original["id"].toString());
        QTest::qWait(600); QCOMPARE(rpc.requests.size(),2);
        power->setProperty("available",true); power->setProperty("preparing",false);
        QVERIFY(QMetaObject::invokeMethod(power.data(),"changed"));
        QTRY_COMPARE_WITH_TIMEOUT(rpc.requests.last()["op"].toString(),QString("hello"),1000);
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",QJsonArray{}}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonObject{{"outcome","pending"},{"acknowledged",false}}}});
        QTRY_COMPARE_WITH_TIMEOUT(rpc.requests.size(),6,1500);
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("lookup"));
        QCOMPARE(rpc.requests.last()["target"],original["id"]);
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonObject{{"outcome","unknown"},{"acknowledged",false}}}});
        QCOMPARE(actions->property("outcome").toString(),QString("unknown"));
        QTest::qWait(600); QCOMPARE(rpc.requests.size(),6);
    }
    void sleepInvalidatesSessionAndQueuedControls() {
        QVERIFY(call("observe",QString("controls")).toBool());
        QVERIFY(session->setProperty("suspended",true));
        rpc.reply(controlSnapshot());
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QVariant accepted;
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh",Q_RETURN_ARG(QVariant,accepted)));
        QVERIFY(!accepted.toBool());
        QVERIFY(session->setProperty("suspended",false));
        QVERIFY(!session->property("ready").toBool());
    }
    void duplicatePressKeepsOriginalIntent() {
        controls();
        QVERIFY(call("control",1,QString("answer")).toBool());
        const auto command=rpc.requests.last();
        QCOMPARE(command["command"].toObject()["revision"].toInt(),7);
        QCOMPARE(command["command"].toObject()["state"].toString(),QString("ringing"));
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QCOMPARE(rpc.requests.size(),2);
        rpc.reply({{"kind","submission"},{"outcome","pending"}});
        QCOMPARE(actions->property("phase").toString(),QString("pending"));
        QVERIFY(!call("control",1,QString("reject")).toBool());
        QVERIFY(call("lookup").toBool());
        QCOMPARE(rpc.requests.last()["target"],command["id"]);
        rpc.reply({{"kind","receipt"},{"epoch",command["epoch"]},{"target",command["id"]},{"receipt",QJsonObject{{"outcome","control_submitted"},{"acknowledged",false}}}});
        QCOMPARE(actions->property("outcome").toString(),QString("control_submitted"));
        QVERIFY(!call("control",1,QString("answer")).toBool()); // requires fresh observation
    }
    void completionCannotBeReplacedDuringBusySignal() {
        QVERIFY(call("observe",QString("controls")).toBool());
        bool attempted=false,accepted=false;
        auto connection=connect(&rpc,&FakeRpc::busyChanged,this,[&] {
            if(!rpc.busy && !attempted) {
                attempted=true;
                accepted=call("observe",QString("dial_status")).toBool();
            }
        });
        rpc.reply(controlSnapshot());
        disconnect(connection);
        QVERIFY(attempted); QVERIFY(!accepted);
        QCOMPARE(rpc.requests.size(),1);
        QVERIFY(actions->property("controlsSnapshot").value<QJSValue>().isObject());
    }
    void invalidationRejectsQueuedObservation() {
        QVERIFY(call("observe",QString("controls")).toBool());
        session->setProperty("phase","unavailable");
        session->setProperty("phase","ready");
        rpc.reply(controlSnapshot());
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QCOMPARE(rpc.requests.size(),1);
    }
    void ageCheckedAtPressAndFromQueryStart() {
        controls(); rpc.now+=2001;
        QVERIFY(!call("control",1,QString("answer")).toBool());
        QVERIFY(call("observe",QString("controls")).toBool());
        rpc.now+=2001;
        rpc.reply(controlSnapshot());
        QVERIFY(!call("control",1,QString("answer")).toBool());
    }
    void unknownSurvivesInvalidationAndMissingReceipt() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last();
        rpc.reply({},3);
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        session->setProperty("phase","unavailable");
        QCOMPARE(actions->property("intentId").toString(),original["id"].toString());
        session->setProperty("phase","ready");
        QVERIFY(call("lookup").toBool());
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonValue::Null}});
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        QVERIFY(!call("observe",QString("controls")).toBool());
    }
    void lostReplyRecoveryRequiresBarrierThenLookup() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last();
        rpc.reply({},3);
        QVERIFY(actions->metaObject()->indexOfMethod("recover()") >= 0);
        QVERIFY(call("recover").toBool());
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("hello"));
        QVERIFY(!call("observe",QString("controls")).toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("fence"));
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",QJsonArray{}}});
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("lookup"));
        QCOMPARE(rpc.requests.last()["target"],original["id"]);
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonValue::Null}});
        QCOMPARE(actions->property("phase").toString(),QString("terminal"));
        QCOMPARE(actions->property("outcome").toString(),QString("not_sent"));
        QCOMPARE(actions->property("intentId").toString(),original["id"].toString());
        QCOMPARE(rpc.requests.size(),5); // observation, command, hello, fence, lookup
    }
    void recoveryDoesNotDiscardAcceptedOrUnknownWork_data() {
        QTest::addColumn<QString>("outcome");
        QTest::newRow("pending") << QString("pending");
        QTest::newRow("unknown") << QString("unknown");
        QTest::newRow("submitted") << QString("control_submitted");
    }
    void recoveryDoesNotDiscardAcceptedOrUnknownWork() {
        QFETCH(QString,outcome);
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({},3);
        QVERIFY(actions->metaObject()->indexOfMethod("recover()") >= 0);
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",QJsonArray{}}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonObject{{"outcome",outcome},{"acknowledged",false}}}});
        QCOMPARE(actions->property("outcome").toString(),outcome);
        QCOMPARE(actions->property("unresolved").toBool(),outcome!="control_submitted");
    }
    void acceptedReceiptCannotVanish() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({{"kind","submission"},{"outcome","pending"}});
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",QJsonArray{}}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonValue::Null}});
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        QVERIFY(!call("observe",QString("controls")).toBool());
    }
    void barrierReceiptCannotVanish() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({},3);
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        QJsonArray items{QJsonObject{{"id",original["id"]},{"outcome","pending"}}};
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",items}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonValue::Null}});
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
    }
    void lostAcknowledgementReplyUsesLookupWithoutReplay() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({},3);
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        QJsonArray items{QJsonObject{{"id",original["id"]},{"outcome","unknown"}}};
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",items}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonObject{{"outcome","unknown"},{"acknowledged",false}}}});
        QVERIFY(call("acknowledge",session->property("recoveryKey")).toBool());
        QCOMPARE(rpc.requests.last()["op"].toString(),QString("acknowledge"));
        rpc.reply({},3); // durable acknowledgement happened, reply was lost
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",original["epoch"]},{"session",QString(32,'f')}});
        rpc.reply({{"kind","fence"},{"epoch",original["epoch"]},{"session",QString(32,'f')},{"admission",QString(32,'8')},{"items",QJsonArray{}}});
        rpc.reply({{"kind","receipt"},{"epoch",original["epoch"]},{"target",original["id"]},{"receipt",QJsonObject{{"outcome","unknown"},{"acknowledged",true}}}});
        QCOMPARE(actions->property("phase").toString(),QString("terminal"));
        QCOMPARE(actions->property("outcome").toString(),QString("unknown"));
        QVERIFY(actions->property("acknowledged").toBool());
        int submits=0,acks=0;
        for(const auto &request:rpc.requests) {
            submits+=request["op"].toString()=="submit";
            acks+=request["op"].toString()=="acknowledge";
        }
        QCOMPARE(submits,1); QCOMPARE(acks,1);
    }
    void failedBarrierKeepsIntent() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool());
        const auto original=rpc.requests.last(); rpc.reply({},3);
        QVERIFY(call("recover").toBool());
        rpc.reply({},3);
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        QCOMPARE(actions->property("intentId").toString(),original["id"].toString());
        QCOMPARE(rpc.requests.size(),3);
    }
    void changedJournalCannotResolveOldIntent() {
        controls(); QVERIFY(call("control",1,QString("answer")).toBool()); rpc.reply({},3);
        QVERIFY(actions->metaObject()->indexOfMethod("recover()") >= 0);
        QVERIFY(call("recover").toBool());
        rpc.reply({{"kind","hello"},{"epoch",QString(32,'9')},{"session",QString(32,'f')}});
        rpc.reply({{"kind","fence"},{"epoch",QString(32,'9')},{"session",QString(32,'f')},{"admission",QString(32,'e')},{"items",QJsonArray{}}});
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        QCOMPARE(rpc.requests.size(),4);
        QVERIFY(!call("observe",QString("controls")).toBool());
    }
    void dialUsesCapturedOfferAndNeverRetries() {
        QVERIFY(call("observe",QString("dial_status")).toBool());
        rpc.reply({{"kind","dial_status"},{"snapshot",QJsonObject{{"generation",QString(32,'b')},{"can_dial",true},{"offer",QString(32,'e')},{"pending",false},{"reason","ready"}}}});
        QVERIFY(!call("dial",QString("bad number")).toBool());
        QVERIFY(call("dial",QString("5550100")).toBool());
        QCOMPARE(rpc.requests.last()["command"].toObject()["offer"].toString(),QString(32,'e'));
        QVERIFY(!call("dial",QString("5550100")).toBool());
        rpc.reply({},2);
        QCOMPARE(actions->property("outcome").toString(),QString("not_sent"));
        QCOMPARE(rpc.requests.size(),2);
        QVERIFY(!call("dial",QString("5550100")).toBool());
    }
};
QTEST_GUILESS_MAIN(ActionsTest)
#include "test_actions.moc"
