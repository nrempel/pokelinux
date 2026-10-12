#include <QtTest>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QProcess>
#include <QSignalSpy>
#include "PhonePower.h"
#include "PhoneRpc.h"
#include <QQmlEngine>
#include <QQmlComponent>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>

class LoginManager : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.login1.Manager")
    Q_PROPERTY(bool PreparingForSleep READ preparing)
public:
    bool preparing() const { return false; }
signals:
    void PrepareForSleep(bool sleeping);
};
class PowerTest : public QObject {
    Q_OBJECT
private slots:
    void privateBusTracksSleepAndOwnerLoss() {
        QProcess bus;
        bus.start("dbus-daemon",{"--session","--nofork","--print-address=1"});
        QVERIFY(bus.waitForStarted()); QVERIFY2(bus.waitForReadyRead(3000),bus.readAllStandardError().constData());
        const auto address=QString::fromUtf8(bus.readLine().trimmed());
        QVERIFY(!address.isEmpty());
        auto server=QDBusConnection::connectToBus(address,"power-test-server");
        auto client=QDBusConnection::connectToBus(address,"power-test-client");
        auto foreign=QDBusConnection::connectToBus(address,"power-test-foreign");
        QVERIFY(server.isConnected()); QVERIFY(client.isConnected()); QVERIFY(foreign.isConnected());
        LoginManager manager;
        QVERIFY(server.registerObject("/org/freedesktop/login1",&manager,QDBusConnection::ExportAllProperties|QDBusConnection::ExportAllSignals));
        QVERIFY(server.registerService("org.freedesktop.login1"));
        PhonePower power(client);
        QSignalSpy changes(&power,&PhonePower::changed);
        QTRY_VERIFY_WITH_TIMEOUT(power.available(),3000);
        QVERIFY(!power.preparing());
        QTemporaryDir root; QVERIFY(root.isValid());
        const QString runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        const QString binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone"); QQmlEngine engine;
        QQmlComponent sc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QScopedPointer<QObject> session(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(sc.errorString()));
        QQmlComponent ac(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QScopedPointer<QObject> actions(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
        QQmlComponent lc(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneLifecycle.qml")));
        QScopedPointer<QObject> lifecycle(lc.createWithInitialProperties({{"power",QVariant::fromValue(&power)},{"session",QVariant::fromValue(session.data())},{"actions",QVariant::fromValue(actions.data())},{"rpc",QVariant::fromValue(&rpc)}}));
        QVERIFY2(lifecycle,qPrintable(lc.errorString()));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        // Retained unadmitted intent: actual wake recovery must fence then look
        // up its original ID through the real Rust processes and journal.
        actions->setProperty("intentId",QString(32,'c'));
        actions->setProperty("intentEpoch",session->property("epoch"));
        actions->setProperty("phase","unknown");
        // Same interface/path from another sender must not affect power state.
        auto spoof=QDBusMessage::createSignal("/org/freedesktop/login1","org.freedesktop.login1.Manager","PrepareForSleep");
        spoof << true; QVERIFY(foreign.send(spoof));
        QTest::qWait(30); QVERIFY(!power.preparing());
        emit manager.PrepareForSleep(true);
        QTRY_VERIFY_WITH_TIMEOUT(power.preparing(),1000);
        QVERIFY(session->property("suspended").toBool());
        QVERIFY(!session->property("ready").toBool());
        QCOMPARE(actions->property("phase").toString(),QString("unknown"));
        emit manager.PrepareForSleep(false);
        QTRY_VERIFY_WITH_TIMEOUT(!power.preparing(),1000);
        QTRY_COMPARE_WITH_TIMEOUT(actions->property("phase").toString(),QString("terminal"),4000);
        QCOMPARE(actions->property("outcome").toString(),QString("not_sent"));
        QCOMPARE(actions->property("intentId").toString(),QString(32,'c'));
        QVERIFY(server.unregisterService("org.freedesktop.login1"));
        QTRY_VERIFY_WITH_TIMEOUT(!power.available(),1000);
        QVERIFY(session->property("suspended").toBool());
        QVERIFY(server.registerService("org.freedesktop.login1"));
        QTRY_VERIFY_WITH_TIMEOUT(power.available(),3000);
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QVERIFY(changes.count() >= 5);
        const auto epoch=session->property("epoch").toString();
        const auto oldSession=session->property("sessionToken").toString();
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000));
        QTRY_COMPARE_WITH_TIMEOUT(session->property("phase").toString(),QString("unavailable"),5000);
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QCOMPARE(session->property("epoch").toString(),epoch);
        QVERIFY(session->property("sessionToken").toString()!=oldSession);
        QCOMPARE(actions->property("intentId").toString(),QString(32,'c'));
        QCOMPARE(actions->property("outcome").toString(),QString("not_sent"));
        bus.terminate(); QVERIFY(bus.waitForFinished(3000));
        QTRY_VERIFY_WITH_TIMEOUT(!power.available(),3000);
        QVERIFY(session->property("suspended").toBool());
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
        QDBusConnection::disconnectFromBus("power-test-client");
        QDBusConnection::disconnectFromBus("power-test-server");
        QDBusConnection::disconnectFromBus("power-test-foreign");
    }
};
QTEST_GUILESS_MAIN(PowerTest)
#include "test_power.moc"
