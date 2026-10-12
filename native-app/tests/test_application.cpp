#include <QtTest>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusReply>
#include <QTemporaryDir>
#include <QProcess>
#include <QFile>
#include <QDir>
#include <signal.h>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include "PhoneRpc.h"
#include "PhoneThemeFiles.h"

class ApplicationTest : public QObject {
    Q_OBJECT
private slots:
    void completeWindowConnectsToRustDaemon() {
        QTest::failOnWarning(QRegularExpression(".*(TypeError|ReferenceError|Binding loop|Unable to assign).*"));
        QTemporaryDir root; QVERIFY(root.isValid());
        const auto runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        const auto binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",root.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone");
        PhoneThemeFiles files(root.path());
        QQmlApplicationEngine engine;
        QQmlComponent pc(&engine); pc.setData("import QtQml; QtObject { signal changed(); property bool available: true; property bool preparing: false }",QUrl());
        QScopedPointer<QObject> power(pc.create()); QVERIFY(power);
        engine.setInitialProperties({{"rpc",QVariant::fromValue(&rpc)},
            {"power",QVariant::fromValue(power.data())},{"themeFiles",QVariant::fromValue(&files)},
            {"socketPath",runtime+"/bridge.sock"}});
        engine.load(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/Main.qml")));
        QCOMPARE(engine.rootObjects().size(),1);
        auto *session=engine.rootObjects().first()->findChild<QObject*>("phoneSession");
        QVERIFY(session);
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        power->setProperty("preparing",true); QVERIFY(QMetaObject::invokeMethod(power.data(),"changed"));
        QTRY_VERIFY(session->property("suspended").toBool());
        QVERIFY(!session->property("ready").toBool());
        delete engine.rootObjects().first();
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
    }
    void installedApplicationActivatesOneOwner() {
        const auto binary=qEnvironmentVariable("PHONE_TEST_APP");
        QVERIFY2(QFile::exists(binary),"installed native app is missing");
        QTemporaryDir root; QVERIFY(root.isValid());
        const auto runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess bus;
        bus.start("dbus-daemon",{"--session","--nofork","--print-address=1"});
        QVERIFY(bus.waitForStarted()); QVERIFY(bus.waitForReadyRead(3000));
        const auto address=QString::fromUtf8(bus.readLine().trimmed());
        auto client=QDBusConnection::connectToBus(address,"application-test");
        QVERIFY(client.isConnected());
        auto environment=QProcessEnvironment::systemEnvironment();
        environment.insert("HOME",root.path());
        environment.insert("XDG_RUNTIME_DIR",runtime);
        environment.insert("DBUS_SESSION_BUS_ADDRESS",address);
        environment.insert("DBUS_SYSTEM_BUS_ADDRESS",address);
        environment.insert("QT_QPA_PLATFORM","offscreen");
        environment.insert("QT_LOGGING_TO_CONSOLE","1");
        environment.insert("QT_QUICK_BACKEND","software");
        QProcess first; first.setProcessEnvironment(environment); first.start(binary,{});
        QVERIFY(first.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(client.interface()->isServiceRegistered("org.omarchy.Phone").value(),3000);
        QDBusInterface activation("org.omarchy.Phone","/org/omarchy/Phone","org.omarchy.Phone",client);
        QTRY_VERIFY_WITH_TIMEOUT(QDBusReply<bool>(activation.call("Activate")).value(),3000);
        const auto owner=client.interface()->serviceOwner("org.omarchy.Phone").value();
        QProcess second; second.setProcessEnvironment(environment); second.start(binary,{});
        QVERIFY(second.waitForStarted()); QVERIFY(second.waitForFinished(3000));
        QCOMPARE(second.exitCode(),0);
        QCOMPARE(first.state(),QProcess::Running);
        QCOMPARE(client.interface()->serviceOwner("org.omarchy.Phone").value(),owner);
        first.terminate(); QVERIFY(first.waitForFinished(3000));
        const auto errors=first.readAllStandardError();
        QVERIFY2(!errors.contains("TypeError") && !errors.contains("ReferenceError")
            && !errors.contains("failed to load") && !errors.contains("qrc:/phone/"),errors.constData());
        QTRY_VERIFY(!client.interface()->isServiceRegistered("org.omarchy.Phone").value());
        bus.terminate(); QVERIFY(bus.waitForFinished(3000));
        QDBusConnection::disconnectFromBus("application-test");
    }
    void installedBusActivationStartsOneOwner() {
        const auto services=qEnvironmentVariable("PHONE_TEST_SERVICES");
        QVERIFY2(QFile::exists(services+"/org.omarchy.Phone.service"),"installed D-Bus activation file is missing");
        QTemporaryDir root; QVERIFY(root.isValid());
        const auto runtime=root.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QFile config(root.path()+"/bus.conf"); QVERIFY(config.open(QIODevice::WriteOnly));
        config.write(("<busconfig><include>/usr/share/dbus-1/session.conf</include><servicedir>"
            +services.toHtmlEscaped()+"</servicedir></busconfig>").toUtf8()); config.close();
        auto environment=QProcessEnvironment::systemEnvironment();
        environment.insert("HOME",root.path());
        environment.insert("XDG_RUNTIME_DIR",runtime);
        environment.insert("QT_QPA_PLATFORM","offscreen");
        environment.insert("QT_LOGGING_TO_CONSOLE","1");
        environment.insert("QT_QUICK_BACKEND","software");
        QProcess bus; bus.setProcessEnvironment(environment);
        bus.start("dbus-daemon",{"--config-file="+config.fileName(),"--nofork","--print-address=1"});
        QVERIFY(bus.waitForStarted()); QVERIFY(bus.waitForReadyRead(3000));
        const auto address=QString::fromUtf8(bus.readLine().trimmed());
        auto client=QDBusConnection::connectToBus(address,"application-autostart-test");
        QVERIFY(client.isConnected());
        QVERIFY(!client.interface()->isServiceRegistered("org.omarchy.Phone").value());
        struct Cleanup { uint pid=0; ~Cleanup() { if(pid) ::kill(pid,SIGTERM); } } cleanup;
        const auto activator=qEnvironmentVariable("PHONE_TEST_ACTIVATOR");
        QVERIFY2(QFile::exists(activator),"Rust activation integration harness is missing");
        environment.insert("DBUS_SESSION_BUS_ADDRESS",address);
        QProcess request; request.setProcessEnvironment(environment);
        request.start(activator,{});
        QVERIFY(request.waitForStarted());
        const bool finished=request.waitForFinished(4000);
        // Record the exact broker-owned child for cleanup even if Activate failed.
        cleanup.pid=client.interface()->servicePid("org.omarchy.Phone").value();
        QVERIFY(finished); QCOMPARE(request.exitCode(),0);
        QVERIFY(cleanup.pid>0);
        const auto owner=client.interface()->serviceOwner("org.omarchy.Phone").value();
        request.start(activator,{});
        QVERIFY(request.waitForStarted()); QVERIFY(request.waitForFinished(4000));
        QCOMPARE(request.exitCode(),0);
        QCOMPARE(client.interface()->serviceOwner("org.omarchy.Phone").value(),owner);
        QCOMPARE(client.interface()->servicePid("org.omarchy.Phone").value(),cleanup.pid);
        QCOMPARE(::kill(cleanup.pid,SIGTERM),0); cleanup.pid=0;
        QTRY_VERIFY(!client.interface()->isServiceRegistered("org.omarchy.Phone").value());
        bus.terminate(); QVERIFY(bus.waitForFinished(3000));
        QDBusConnection::disconnectFromBus("application-autostart-test");
    }
    void missingRuntimeFailsBeforeActivation() {
        const auto binary=qEnvironmentVariable("PHONE_TEST_APP");
        QVERIFY(QFile::exists(binary));
        auto environment=QProcessEnvironment::systemEnvironment();
        environment.remove("XDG_RUNTIME_DIR");
        environment.insert("QT_QPA_PLATFORM","offscreen");
        environment.insert("QT_LOGGING_TO_CONSOLE","1");
        QProcess app; app.setProcessEnvironment(environment); app.start(binary,{});
        QVERIFY(app.waitForStarted()); QVERIFY(app.waitForFinished(3000));
        QCOMPARE(app.exitCode(),2);
        const auto errors=app.readAllStandardError();
        QVERIFY2(errors.contains("XDG_RUNTIME_DIR"),errors.constData());
    }
};
QTEST_MAIN(ApplicationTest)
#include "test_application.moc"
