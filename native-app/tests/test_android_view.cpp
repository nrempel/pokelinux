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
class AndroidViewTest : public QObject {
    Q_OBJECT
private slots:
    void touchAnswerAndEndReachAndroidTelecom() {
        QTest::failOnWarning(QRegularExpression(QStringLiteral(".*TypeError.*")));
        const auto binaries=qEnvironmentVariable("PHONE_TEST_BINARIES");
        QTemporaryDir directory; QVERIFY(directory.isValid());
        const auto runtime=directory.path()+"/runtime";
        QVERIFY(QDir().mkdir(runtime));
        QVERIFY(QFile::setPermissions(runtime,QFile::ReadOwner|QFile::WriteOwner|QFile::ExeOwner));
        QProcess daemon;
        daemon.start(binaries+"/omarchy-phoned",{"--enrollment",qEnvironmentVariable("PHONE_EMULATOR_ENROLLMENT"),"--runtime",runtime,"--state",directory.path()+"/state"});
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
        QTRY_VERIFY_WITH_TIMEOUT(answer->isEnabled(),10000); QVERIFY(!dial->isEnabled());
        QTest::qWait(30); QVERIFY(view.grabWindow().save("/results/phone-android-incoming.png"));
        auto *device=QTest::createTouchDevice();
        auto touch=[&](QQuickItem *button) {
            const auto point=button->mapToScene(QPointF(button->width()/2,button->height()/2)).toPoint();
            QTest::touchEvent(&view,device).press(0,point,&view);
            QTest::touchEvent(&view,device).release(0,point,&view);
        };
        touch(answer);
        QTRY_COMPARE_WITH_TIMEOUT(actions->property("outcome").toString(),QString("control_submitted"),3000);
        auto *end=view.rootObject()->findChild<QQuickItem*>("endButton"); QVERIFY(end);
        QTRY_VERIFY_WITH_TIMEOUT(end->isVisible() && end->isEnabled(),10000);
        QCOMPARE(actions->property("controlsSnapshot").value<QJSValue>().property("calls").property(0).property("state").toInt(),4);
        QVERIFY(view.grabWindow().save("/results/phone-android-active.png"));
        touch(end);
        QTRY_COMPARE_WITH_TIMEOUT(actions->property("outcome").toString(),QString("control_submitted"),5000);
        QTRY_VERIFY_WITH_TIMEOUT(actions->property("controlsSnapshot").value<QJSValue>().property("calls").isArray() && actions->property("controlsSnapshot").value<QJSValue>().property("calls").property("length").toInt()==0,10000);
        observations->setProperty("active",false);
        view.setSource(QUrl());
        daemon.terminate(); QVERIFY(daemon.waitForFinished(4000)); QCOMPARE(daemon.exitCode(),0);
    }
};
QTEST_MAIN(AndroidViewTest)
#include "test_android_view.moc"
