#include <QtTest>
#include <QQuickView>
#include <QQuickItem>
#include <QSignalSpy>
#include <QJSValue>
#include <QJsonObject>
#include <QJsonArray>
#include <QJsonDocument>
#include <QQmlComponent>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
#include "PhoneRpc.h"
class ViewTest : public QObject {
    Q_OBJECT
private slots:
    void touchAnswerAndDialCrossAuthenticatedTransport() {
        QTest::failOnWarning(QRegularExpression(QStringLiteral(".*TypeError.*")));
        const auto binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir directory; QVERIFY(directory.isValid());
        QProcess peer; peer.start(qEnvironmentVariable("PHONE_TEST_PEER"),{directory.path()+"/enrollment"});
        QVERIFY(peer.waitForStarted()); QVERIFY(peer.waitForReadyRead(3000));
        QCOMPARE(peer.readLine().trimmed(),QByteArray("ready"));
        const auto runtime=directory.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",directory.path()+"/enrollment","--runtime",runtime,"--state",directory.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone"); QQuickView view;
        QQmlComponent sc(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QScopedPointer<QObject> session(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(sc.errorString()));
        QQmlComponent ac(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QScopedPointer<QObject> actions(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
        QQmlComponent pc(view.engine()); pc.setData("import QtQml; QtObject { signal changed(); property bool available: true; property bool preparing: false }",QUrl());
        QScopedPointer<QObject> power(pc.create()); QVERIFY(power);
        QQmlComponent lc(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneLifecycle.qml")));
        QScopedPointer<QObject> lifecycle(lc.createWithInitialProperties({{"power",QVariant::fromValue(power.data())},{"session",QVariant::fromValue(session.data())},{"actions",QVariant::fromValue(actions.data())},{"rpc",QVariant::fromValue(&rpc)}}));
        QVERIFY2(lifecycle,qPrintable(lc.errorString()));
        QQmlComponent oc(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneObservations.qml")));
        QScopedPointer<QObject> observations(oc.createWithInitialProperties({{"session",QVariant::fromValue(session.data())},{"actions",QVariant::fromValue(actions.data())},{"active",true}}));
        QVERIFY2(observations,qPrintable(oc.errorString()));
        view.setInitialProperties({{"injectedSession",QVariant::fromValue(session.data())},{"injectedActions",QVariant::fromValue(actions.data())}});
        view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.setSource(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/tests/ViewFixture.qml")));
        QVERIFY(view.status()==QQuickView::Ready);
        view.resize(390,844); view.show(); QVERIFY(QTest::qWaitForWindowExposed(&view));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("controlsSnapshot").value<QJSValue>().isObject(),4000);
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("dialSnapshot").value<QJSValue>().isObject(),4000);
        auto *answer=view.rootObject()->findChild<QQuickItem*>("answerButton"); QVERIFY(answer);
        auto *dial=view.rootObject()->findChild<QQuickItem*>("dialButton"); QVERIFY(dial);
        QVERIFY(answer->isEnabled()); QVERIFY(!dial->isEnabled());
        QTest::qWait(30); QVERIFY(view.grabWindow().save("/results/phone-incoming-tls.png"));
        auto *device=QTest::createTouchDevice();
        auto touch=[&](QQuickItem *button) {
            const auto point=button->mapToScene(QPointF(button->width()/2,button->height()/2)).toPoint();
            QTest::touchEvent(&view,device).press(0,point,&view);
            QTest::touchEvent(&view,device).release(0,point,&view);
        };
        touch(answer);
        QTRY_COMPARE_WITH_TIMEOUT(actions->property("outcome").toString(),QString("control_submitted"),3000);
        const auto answerId=actions->property("intentId").toString();
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("canDial").toBool(),4000);
        auto *number=view.rootObject()->findChild<QQuickItem*>("numberField"); QVERIFY(number);
        number->forceActiveFocus();
        for (const auto digit:QString("5550100")) QTest::keyClick(&view,static_cast<Qt::Key>(digit.unicode()));
        QCOMPARE(number->property("text").toString(),QString("5550100"));
        QVERIFY(dial->isEnabled());
        QTest::qWait(30); touch(dial);
        QTRY_COMPARE_WITH_TIMEOUT(actions->property("outcome").toString(),QString("dial_invoked"),3000);
        QVERIFY(actions->property("intentId").toString()!=answerId);
        QTRY_COMPARE_WITH_TIMEOUT(peer.state(),QProcess::NotRunning,3000);
        QCOMPARE(peer.exitStatus(),QProcess::NormalExit); QCOMPARE(peer.exitCode(),0);
        QCOMPARE(peer.readAllStandardOutput().trimmed(),QByteArray("two authenticated commands verified"));
        observations->setProperty("active",false);
        view.setSource(QUrl());
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
    }
    void touchAcknowledgementReachesRealJournal() {
        QTest::failOnWarning(QRegularExpression(QStringLiteral(".*TypeError.*")));
        const auto binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir directory; QVERIFY(directory.isValid());
        QProcess seed; seed.start(qEnvironmentVariable("PHONE_TEST_SEED"),{directory.path()+"/state"});
        QVERIFY(seed.waitForFinished(3000)); QCOMPARE(seed.exitCode(),0);
        const auto runtime=directory.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_TEST_ENROLLMENT"),"--runtime",runtime,"--state",directory.path()+"/state"});
        QVERIFY(daemon.waitForStarted());
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(runtime+"/bridge.sock"),3000);
        PhoneRpc rpc(binaries+"/omarchy-phone"); QQuickView view;
        QQmlComponent sc(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneSession.qml")));
        QScopedPointer<QObject> session(sc.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"socketPath",runtime+"/bridge.sock"}}));
        QVERIFY2(session,qPrintable(sc.errorString()));
        QQmlComponent ac(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneActions.qml")));
        QScopedPointer<QObject> actions(ac.createWithInitialProperties({{"rpc",QVariant::fromValue(&rpc)},{"session",QVariant::fromValue(session.data())}}));
        QVERIFY2(actions,qPrintable(ac.errorString()));
        view.setInitialProperties({{"injectedSession",QVariant::fromValue(session.data())},{"injectedActions",QVariant::fromValue(actions.data())}});
        view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.setSource(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/tests/ViewFixture.qml")));
        QVERIFY(view.status()==QQuickView::Ready);
        view.resize(390,844); view.show(); QVERIFY(QTest::qWaitForWindowExposed(&view));
        QVERIFY(QMetaObject::invokeMethod(session.data(),"refresh"));
        QTRY_VERIFY_WITH_TIMEOUT(session->property("canAcknowledge").toBool(),4000);
        auto *button=view.rootObject()->findChild<QQuickItem*>("recoveryButton"); QVERIFY(button);
        QVERIFY(button->isVisible()); QVERIFY(button->isEnabled());
        QTest::qWait(30);
        const auto point=button->mapToScene(QPointF(button->width()/2,button->height()/2)).toPoint();
        auto *touch=QTest::createTouchDevice();
        QTest::touchEvent(&view,touch).press(0,point,&view);
        QTest::touchEvent(&view,touch).release(0,point,&view);
        QTRY_VERIFY_WITH_TIMEOUT(session->property("ready").toBool(),4000);
        QProcess lookup;
        lookup.start(binaries+"/omarchy-phone",{"--socket",runtime+"/bridge.sock","rpc"});
        QVERIFY(lookup.waitForStarted());
        QJsonObject request{{"v",1},{"id",QString(32,'a')},{"op","lookup"},{"epoch",session->property("epoch").toString()},{"target",QString(32,'c')}};
        lookup.write(QJsonDocument(request).toJson(QJsonDocument::Compact)+"\n"); lookup.closeWriteChannel();
        QVERIFY(lookup.waitForFinished(3000)); QCOMPARE(lookup.exitCode(),0);
        const auto receipt=QJsonDocument::fromJson(lookup.readAllStandardOutput()).object()["result"].toObject()["receipt"].toObject();
        QCOMPARE(receipt["outcome"].toString(),QString("unknown")); QVERIFY(receipt["acknowledged"].toBool());
        QVERIFY(actions->property("intentId").toString().isEmpty());
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
        // App state must outlive its view; destroy visual bindings first.
        view.setSource(QUrl());
    }
    void recoveryIsExplicitAndPressContextIsStable() {
        QQuickView view; view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.setSource(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/tests/ViewFixture.qml")));
        QVERIFY(view.status()==QQuickView::Ready);
        view.resize(390,844); view.show(); QVERIFY(QTest::qWaitForWindowExposed(&view));
        auto *root=view.rootObject(); QVERIFY(root);
        auto *session=qvariant_cast<QObject*>(root->property("fixtureSession")); QVERIFY(session);
        auto *actions=qvariant_cast<QObject*>(root->property("fixtureActions")); QVERIFY(actions);
        session->setProperty("ready",false); session->setProperty("needsRecovery",true);
        session->setProperty("canAcknowledge",true); session->setProperty("recoveryKey","journal:command:unknown");
        actions->setProperty("canObserve",false);
        QSignalSpy commands(root,SIGNAL(actionSubmitted(QString)));
        QSignalSpy recovery(root,SIGNAL(recoveryRequested(QString,QString)));
        auto *button=root->findChild<QQuickItem*>("recoveryButton"); QVERIFY(button);
        QTRY_VERIFY(button->isVisible()); QVERIFY(button->isEnabled());
        QCOMPARE(button->property("text").toString(),QString("Acknowledge uncertainty"));
        QTest::qWait(30);
        const auto point=button->mapToScene(QPointF(button->width()/2,button->height()/2)).toPoint();
        QTest::mousePress(&view,Qt::LeftButton,Qt::NoModifier,point);
        session->setProperty("recoveryKey","journal:other:unknown");
        QTest::mouseRelease(&view,Qt::LeftButton,Qt::NoModifier,point);
        QCOMPARE(recovery.count(),0);
        QTest::mouseClick(&view,Qt::LeftButton,Qt::NoModifier,point);
        QTRY_COMPARE(recovery.count(),1);
        QCOMPARE(recovery[0][0].toString(),QString("session_ack"));
        QCOMPARE(recovery[0][1].toString(),QString("journal:other:unknown"));
        QCOMPARE(commands.count(),0);
        session->setProperty("canAcknowledge",false); session->setProperty("needsRecovery",false);
        actions->setProperty("unresolved",true); actions->setProperty("phase","unknown"); actions->setProperty("outcome","unknown");
        QCOMPARE(button->property("text").toString(),QString("Check result"));
        QTest::mouseClick(&view,Qt::LeftButton,Qt::NoModifier,point);
        QTRY_COMPARE(recovery.count(),2); QCOMPARE(recovery[1][0].toString(),QString("recover"));
        QCOMPARE(commands.count(),0);
        QTest::qWait(30); QVERIFY(view.grabWindow().save("/results/phone-recovery.png"));
    }
    void touchKeyboardAndChangedCall() {
        QQuickView view;
        view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.setSource(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/tests/ViewFixture.qml")));
        QVERIFY2(view.status()==QQuickView::Ready,qPrintable(view.errors().isEmpty()?QString():view.errors().first().toString()));
        view.resize(390,844); view.show(); QVERIFY(QTest::qWaitForWindowExposed(&view));
        auto *root=view.rootObject(); QVERIFY(root);
        auto *answer=root->findChild<QQuickItem*>("answerButton"); QVERIFY(answer);
        auto *dial=root->findChild<QQuickItem*>("dialButton"); QVERIFY(dial);
        auto *number=root->findChild<QQuickItem*>("numberField"); QVERIFY(number);
        QCOMPARE(number->property("inputMethodHints").toInt(),int(Qt::ImhDialableCharactersOnly));
        QVERIFY(answer->isEnabled()); QVERIFY(!dial->isEnabled());
        QVERIFY(answer->height()>=56); QVERIFY(answer->width()>=56);
        QSignalSpy submitted(root,SIGNAL(actionSubmitted(QString)));
        const auto point=answer->mapToScene(QPointF(answer->width()/2,answer->height()/2)).toPoint();
        auto *touch=QTest::createTouchDevice();
        QTest::touchEvent(&view,touch).press(0,point,&view);
        QTest::touchEvent(&view,touch).release(0,point,&view);
        QTRY_COMPARE(submitted.count(),1); QCOMPARE(submitted[0][0].toString(),QString("answer"));
        answer->forceActiveFocus();
        QTest::keyClick(&view,Qt::Key_Space);
        QTRY_COMPARE(submitted.count(),2);
        QTest::mousePress(&view,Qt::LeftButton,Qt::NoModifier,point);
        auto *actions=qvariant_cast<QObject*>(root->property("fixtureActions")); QVERIFY(actions);
        QVariantMap call{{"id",2},{"state",2},{"actions",QStringList{"answer","reject"}}};
        actions->setProperty("controlsSnapshot",QVariantMap{{"generation",QString(32,'d')},{"revision",8},{"controls_allowed",true},{"calls",QVariantList{call}}});
        QTest::mouseRelease(&view,Qt::LeftButton,Qt::NoModifier,point);
        QCOMPARE(submitted.count(),2);
        number->setProperty("text","5550100");
        QTest::qWait(50);
        QVERIFY(view.grabWindow().save("/results/phone-incoming.png"));
        auto *theme=qvariant_cast<QObject*>(root->property("fixtureTheme")); QVERIFY(theme);
        theme->setProperty("background",QColor("#faf4e8"));
        theme->setProperty("surface",QColor("#ede6d7"));
        theme->setProperty("foreground",QColor("#292522"));
        theme->setProperty("muted",QColor("#5c5b56"));
        theme->setProperty("border",QColor("#afa99d"));
        theme->setProperty("accent",QColor("#8a5b1f"));
        theme->setProperty("positive",QColor("#406524"));
        theme->setProperty("urgent",QColor("#a83246"));
        theme->setProperty("ink",QColor("#ffffff"));
        QTest::qWait(50);
        QCOMPARE(number->property("text").toString(),QString("5550100"));
        QCOMPARE(submitted.count(),2);
        QCOMPARE(view.grabWindow().pixelColor(5,5),QColor("#faf4e8"));
        QVERIFY(view.grabWindow().save("/results/phone-incoming-light.png"));
    }
};
QTEST_MAIN(ViewTest)
#include "test_view.moc"
