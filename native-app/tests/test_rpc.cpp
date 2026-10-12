#include "PhoneRpc.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QFile>
#include <QtTest>
#include <QQmlEngine>
#include <QJSValue>
#include <QQmlComponent>
class RpcTest : public QObject {
    Q_OBJECT
private slots:
    void suspendAwareClockAvailable() {
        PhoneRpc rpc;
        double before=-1,after=-1;
        QVERIFY(QMetaObject::invokeMethod(&rpc,"boottimeMs",Q_RETURN_ARG(double,before)));
        QVERIFY(before >= 0);
        QTest::qWait(5);
        QVERIFY(QMetaObject::invokeMethod(&rpc,"boottimeMs",Q_RETURN_ARG(double,after)));
        QVERIFY(after >= before);
    }
    void realDaemon() {
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        const QString enrollment=qEnvironmentVariable("PHONE_TEST_ENROLLMENT");
        QVERIFY(!binaries.isEmpty()); QVERIFY(!enrollment.isEmpty());
        QTemporaryDir root; QVERIFY(root.isValid());
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",enrollment,"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone");
        QSignalSpy completed(&rpc,&PhoneRpc::completed);
        QString frame="{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"hello\"}\n";
        QVERIFY(rpc.request(runtime+"/bridge.sock",frame,"hello-test"));
        QVERIFY(!rpc.request(runtime+"/bridge.sock",frame,"hello-test"));
        QTRY_COMPARE_WITH_TIMEOUT(completed.count(),1,4000);
        QCOMPARE(completed[0][0].toInt(),0);
        auto reply=QJsonDocument::fromJson(completed[0][1].toByteArray()).object();
        QCOMPARE(reply["result"].toObject()["kind"].toString(),QString("hello"));
        QVERIFY(!rpc.busy());
        QVERIFY(rpc.request(runtime+"/bridge.sock","{}\n","invalid-test"));
        QTRY_COMPARE_WITH_TIMEOUT(completed.count(),2,4000);
        QCOMPARE(completed[1][0].toInt(),2);
        QVERIFY(completed[1][1].toByteArray().isEmpty());
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000));
        QCOMPARE(daemon.exitCode(),0);
    }
    void boundedChildFailure_data() {
        QTest::addColumn<QByteArray>("script");
        QTest::newRow("oversize") << QByteArray("#!/bin/sh\nhead -c 20000 /dev/zero\n");
        QTest::newRow("timeout") << QByteArray("#!/bin/sh\nexec sleep 8\n");
    }
    void boundedChildFailure() {
        QFETCH(QByteArray,script);
        QTemporaryDir root; QVERIFY(root.isValid());
        const QString path=root.path()+"/client";
        QFile file(path); QVERIFY(file.open(QIODevice::WriteOnly));
        QCOMPARE(file.write(script),script.size()); file.close();
        QVERIFY(file.setPermissions(QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        PhoneRpc rpc(path); QSignalSpy done(&rpc,&PhoneRpc::completed);
        QVERIFY(rpc.request("/tmp/missing.sock","{}\n","failure-test"));
        QTRY_COMPARE_WITH_TIMEOUT(done.count(),1,5500);
        QCOMPARE(done[0][0].toInt(),3); QVERIFY(done[0][1].toByteArray().isEmpty());
        QVERIFY(!rpc.busy()); QTest::qWait(30); QCOMPARE(done.count(),1);
    }
    void qmlSessionUsesRealDaemon() {
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir root; QVERIFY(root.isValid());
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone");
        QQmlEngine engine;
        QQmlComponent component(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QVERIFY2(component.isReady(),qPrintable(component.errorString()));
        QScopedPointer<QObject> session(component.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(component.errorString()));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        rpc.completed(3,QByteArray(),QStringLiteral("unrelated-action"));
        QVERIFY(session->property("phase").toString() != QStringLiteral("unavailable"));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QCOMPARE(session->property("epoch").toString().size(),32);
        QCOMPARE(session->property("sessionToken").toString().size(),32);
        QCOMPARE(session->property("admissionToken").toString().size(),32);
        const auto admission=session->property("admissionToken").toString();
        QVERIFY(session->metaObject()->indexOfMethod("checkHealth()") >= 0);
        QVariant checked;
        QVERIFY(QMetaObject::invokeMethod(session.data(),"checkHealth",Q_RETURN_ARG(QVariant,checked)));
        QVERIFY(checked.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(!rpc.busy(),4000);
        QVERIFY(session->property("ready").toBool());
        QCOMPARE(session->property("admissionToken").toString(),admission);
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000));
        // Automatic health check must invalidate the session without a user refresh.
        QTRY_VERIFY_WITH_TIMEOUT(!session->property("ready").toBool(),5000);
        QTRY_COMPARE_WITH_TIMEOUT(session->property("phase").toString(),QString("unavailable"),4000);
        QVERIFY(session->property("admissionToken").toString().isEmpty());
    }
    void qmlRecoveryRequiresExplicitAcknowledgement() {
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir root; QVERIFY(root.isValid());
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess seed;
        seed.start(qEnvironmentVariable("PHONE_TEST_SEED"),{root.path()+"/state"});
        QVERIFY(seed.waitForFinished(3000)); QCOMPARE(seed.exitCode(),0);
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone");
        QQmlEngine engine;
        QQmlComponent component(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QVERIFY2(component.isReady(),qPrintable(component.errorString()));
        QScopedPointer<QObject> session(component.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(component.errorString()));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("needsRecovery").toBool(),4000);
        QVERIFY(!session->property("ready").toBool());
        QVERIFY(session->property("canAcknowledge").toBool());
        const QString key=session->property("recoveryKey").toString(); QVERIFY(!key.isEmpty());
        const QString admission=session->property("admissionToken").toString();
        QCOMPARE(admission.size(),32);
        QVariant accepted;
        QVERIFY(QMetaObject::invokeMethod(session.data(),"acknowledge",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("stale-key"))));
        QVERIFY(!accepted.toBool()); QVERIFY(session->property("needsRecovery").toBool());
        QVERIFY(QMetaObject::invokeMethod(session.data(),"acknowledge",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,key)));
        QVERIFY(accepted.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QVERIFY(!session->property("canAcknowledge").toBool());
        QCOMPARE(session->property("admissionToken").toString().size(),32);
        QVERIFY(session->property("admissionToken").toString() != admission);
        QProcess lookup;
        lookup.start(binaries+"/omarchy-phone",{"--socket",runtime+"/bridge.sock","rpc"});
        QVERIFY(lookup.waitForStarted());
        QJsonObject query{{"v",1},{"id","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},{"op","lookup"},{"epoch",session->property("epoch").toString()},{"target","cccccccccccccccccccccccccccccccc"}};
        lookup.write(QJsonDocument(query).toJson(QJsonDocument::Compact)+"\n"); lookup.closeWriteChannel();
        QVERIFY(lookup.waitForFinished(3000)); QCOMPARE(lookup.exitCode(),0);
        const auto receipt=QJsonDocument::fromJson(lookup.readAllStandardOutput()).object()["result"].toObject()["receipt"].toObject();
        QCOMPARE(receipt["outcome"].toString(),QString("unknown")); QVERIFY(receipt["acknowledged"].toBool());

        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        QTRY_VERIFY_WITH_TIMEOUT(!session->property("ready").toBool(),1000);
        QTRY_COMPARE_WITH_TIMEOUT(session->property("phase").toString(),QString("unavailable"),4000);
        QVERIFY(session->property("admissionToken").toString().isEmpty());
    }
    void qmlLiveRecoveryUsesRealJournal_data() {
        QTest::addColumn<bool>("seeded");
        QTest::newRow("unadmitted-command") << false;
        QTest::newRow("durable-unknown-command") << true;
    }
    void qmlLiveRecoveryUsesRealJournal() {
        QFETCH(bool,seeded);
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir root; QVERIFY(root.isValid());
        if(seeded) {
            QProcess seed; seed.start(qEnvironmentVariable("PHONE_TEST_SEED"),{root.path()+"/state"});
            QVERIFY(seed.waitForFinished(3000)); QCOMPARE(seed.exitCode(),0);
        }
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone"); QQmlEngine engine;
        QQmlComponent sc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QScopedPointer<QObject> session(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(sc.errorString()));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool() || session->property("needsRecovery").toBool(),4000);
        const auto admission=session->property("admissionToken").toString();
        QQmlComponent ac(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QScopedPointer<QObject> actions(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
        // Model the app's retained intent after losing its reply. The recovery
        // operations below all use actual packaged processes and SQLite state.
        actions->setProperty("intentId",QString(32,'c'));
        actions->setProperty("intentEpoch",session->property("epoch"));
        actions->setProperty("phase","unknown");
        QVariant accepted;
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"recover",Q_RETURN_ARG(QVariant,accepted)));
        QVERIFY(accepted.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("phase").toString()!="recovering",4000);
        QVERIFY(session->property("admissionToken").toString()!=admission);
        QCOMPARE(actions->property("outcome").toString(),QString(seeded ? "unknown" : "not_sent"));
        QCOMPARE(actions->property("unresolved").toBool(),seeded);
        if(seeded) {
            QVERIFY(actions->metaObject()->indexOfMethod("acknowledge(QVariant)") >= 0);
            QVERIFY(QMetaObject::invokeMethod(actions.data(),"acknowledge",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("stale"))));
            QVERIFY(!accepted.toBool());
            QVERIFY(QMetaObject::invokeMethod(actions.data(),"acknowledge",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,session->property("recoveryKey"))));
            QVERIFY(accepted.toBool());
            QTRY_VERIFY_WITH_TIMEOUT(!actions->property("unresolved").toBool(),4000);
            QCOMPARE(actions->property("outcome").toString(),QString("unknown"));
            QVERIFY(actions->property("acknowledged").toBool());
            QVERIFY(session->property("ready").toBool());
        }
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
    }
    void qmlActionsCrossAuthenticatedTransport() {
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir root; QVERIFY(root.isValid());
        QProcess peer;
        peer.start(qEnvironmentVariable("PHONE_TEST_PEER"),{root.path()+"/enrollment"});
        QVERIFY(peer.waitForStarted());
        QVERIFY(peer.waitForReadyRead(3000));
        QCOMPARE(peer.readLine().trimmed(),QByteArray("ready"));
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",root.path()+"/enrollment","--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone");
        QQmlEngine engine;
        QQmlComponent sc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QScopedPointer<QObject> session(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(sc.errorString()));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QQmlComponent ac(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QScopedPointer<QObject> actions(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
        QQmlComponent pc(&engine); pc.setData("import QtQml; QtObject { signal changed(); property bool available: true; property bool preparing: false }",QUrl());
        QScopedPointer<QObject> power(pc.create()); QVERIFY(power);
        const auto admission=session->property("admissionToken").toString();
        QQmlComponent lc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneLifecycle.qml")));
        QScopedPointer<QObject> lifecycle(lc.createWithInitialProperties({{"power",QVariant::fromValue(power.data())},{"session",QVariant::fromValue(session.data())},{"actions",QVariant::fromValue(actions.data())},{"rpc",QVariant::fromValue(&rpc)}}));
        QVERIFY2(lifecycle,qPrintable(lc.errorString()));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool() && session->property("admissionToken").toString()!=admission,4000);
        QVariant accepted;
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"observe",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("controls"))));
        QVERIFY(accepted.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("controlsSnapshot").value<QJSValue>().isObject(),4000);
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"observe",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("dial_status"))));
        QVERIFY(accepted.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("dialSnapshot").value<QJSValue>().isObject(),4000);
        QVERIFY(!actions->property("canDial").toBool());
        QVERIFY(actions->property("controlsSnapshot").value<QJSValue>().isObject());
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"control",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,1),Q_ARG(QVariant,QString("answer"))));
        QVERIFY(accepted.toBool());
        const auto answerId=actions->property("intentId").toString();
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"control",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,1),Q_ARG(QVariant,QString("answer"))));
        QVERIFY(!accepted.toBool());
        auto resolved=[&] {
            return actions->property("phase").toString()=="terminal";
        };
        QTRY_VERIFY_WITH_TIMEOUT(resolved(),2000);
        QCOMPARE(actions->property("intentId").toString(),answerId);
        QCOMPARE(actions->property("outcome").toString(),QString("control_submitted"));
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"observe",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("dial_status"))));
        QVERIFY(accepted.toBool());
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("canDial").toBool(),4000);
        QVERIFY(QMetaObject::invokeMethod(actions.data(),"dial",Q_RETURN_ARG(QVariant,accepted),Q_ARG(QVariant,QString("5550100"))));
        QVERIFY(accepted.toBool());
        QVERIFY(actions->property("intentId").toString()!=answerId);
        QTRY_VERIFY_WITH_TIMEOUT(resolved(),2000);
        QCOMPARE(actions->property("outcome").toString(),QString("dial_invoked"));
        QTRY_COMPARE_WITH_TIMEOUT(peer.state(),QProcess::NotRunning,3000);
        QCOMPARE(peer.exitStatus(),QProcess::NormalExit); QCOMPARE(peer.exitCode(),0);
        QCOMPARE(peer.readAllStandardOutput().trimmed(),QByteArray("two authenticated commands verified"));
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
    }
    void destroyingAdapterDoesNotDeliverLateCompletion() {
        QTemporaryDir root; QVERIFY(root.isValid());
        QFile script(root.path()+"/client"); QVERIFY(script.open(QIODevice::WriteOnly));
        script.write("#!/bin/sh\nexec sleep 8\n"); script.close();
        QVERIFY(script.setPermissions(QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        auto *rpc=new PhoneRpc(script.fileName()); int completions=0;
        connect(rpc,&PhoneRpc::completed,this,[&] { ++completions; });
        QVERIFY(rpc->request("/tmp/missing.sock","{}\n","closing-ui"));
        QTest::qWait(30);
        delete rpc;
        QCOMPARE(completions,0);
    }
    void completionCarriesCallerToken() {
        PhoneRpc rpc("/does-not-exist/omarchy-phone");
        QSignalSpy done(&rpc,&PhoneRpc::completed);
        QVERIFY(rpc.request("/tmp/missing.sock","{}\n","session-query-1"));
        QTRY_COMPARE_WITH_TIMEOUT(done.count(),1,1000);
        QCOMPARE(done[0][2].toString(),QString("session-query-1"));
    }
    void missingClientIsNotSent() {
        PhoneRpc rpc("/does-not-exist/omarchy-phone"); QSignalSpy done(&rpc,&PhoneRpc::completed);
        QVERIFY(rpc.request("/tmp/missing.sock","{}\n","failure-test"));
        QTRY_COMPARE_WITH_TIMEOUT(done.count(),1,1000);
        QCOMPARE(done[0][0].toInt(),2); QVERIFY(!rpc.busy());
    }
};
QTEST_GUILESS_MAIN(RpcTest)
#include "test_rpc.moc"
