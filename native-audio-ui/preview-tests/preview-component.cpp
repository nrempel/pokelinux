// Real production QML and both real Rust-backed adapters; no app window or mock engine.
#include "AudioClient.h"
#include "PhoneAppearanceClient.h"
#include <QGuiApplication>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickView>
#include <QTemporaryDir>
#include <QtTest>
#include <memory>
#include <unistd.h>

static QString appearanceDirectory;
class SoundsPreviewComponentTest final : public QObject {
    Q_OBJECT
private:
    std::unique_ptr<QTemporaryDir> runtime;
    std::unique_ptr<PhoneAppearanceClient> appearance;
    std::unique_ptr<AudioClient> audio;
    std::unique_ptr<QObject> theme;
    std::unique_ptr<QQuickView> view;
    QStringList qmlWarnings;
    QObject *buttonWithText(const QString &text) const {
        for (auto *item : view->rootObject()->findChildren<QObject *>()) {
            if (item->inherits("QQuickButton") && item->property("text").toString() == text) return item;
        }
        return nullptr;
    }
    bool plainTextExists(const QString &text) const {
        for (auto *item : view->rootObject()->findChildren<QObject *>()) {
            if (item->property("text").toString() == text && item->property("textFormat").isValid()
                && item->property("textFormat").toInt() == 0) return true;
        }
        return false;
    }
private slots:
    void initTestCase() {
        QCOMPARE(getuid(), uid_t(11042));
        QCOMPARE(geteuid(), uid_t(11042));
        QVERIFY(!appearanceDirectory.isEmpty());
    }
    void init() {
        qmlWarnings.clear();
        runtime = std::make_unique<QTemporaryDir>(QStringLiteral("/run/sounds-preview-fixture-XXXXXX"));
        QVERIFY(runtime->isValid());
        appearance = std::make_unique<PhoneAppearanceClient>(runtime->path() + "/appearance-missing.sock");
        audio = std::make_unique<AudioClient>(runtime->path() + "/actor-missing.sock", uid_t(0));
        view = std::make_unique<QQuickView>();
        connect(view->engine(), &QQmlEngine::warnings, this, [this](const QList<QQmlError> &errors) {
            for (const auto &error : errors) qmlWarnings.append(error.toString());
        });
        QQmlComponent themeComponent(view->engine(), QUrl::fromLocalFile(appearanceDirectory + "/PhoneAppearance.qml"));
        theme.reset(themeComponent.createWithInitialProperties({{"source", QVariant::fromValue(appearance.get())}}));
        QVERIFY2(theme != nullptr, qPrintable(themeComponent.errorString()));
        view->setInitialProperties({{"theme", QVariant::fromValue(theme.get())}, {"audioClient", QVariant::fromValue(audio.get())}});
        view->setSource(QUrl(QStringLiteral("qrc:/sounds/SoundsView.qml")));
        QStringList errors;
        for (const auto &error : view->errors()) errors.append(error.toString());
        QVERIFY2(view->status() == QQuickView::Ready && view->rootObject(), qPrintable(errors.join("\n")));
        QTRY_VERIFY_WITH_TIMEOUT(audio->canReconnect() && audio->status().contains(QStringLiteral("unavailable"), Qt::CaseInsensitive), 1000);
        QVERIFY(!audio->canChoose());
        QVERIFY(audio->routes().isEmpty());
        QVERIFY(audio->status().contains(QStringLiteral("unavailable"), Qt::CaseInsensitive));
        QVERIFY(!appearance->available());
        QVERIFY2(qmlWarnings.isEmpty(), qPrintable(qmlWarnings.join("\n")));
    }
    void oldViewConstruction() {
        QVERIFY(plainTextExists(QStringLiteral("Sounds")));
        QVERIFY(plainTextExists(QStringLiteral("Media output")));
        QVERIFY(plainTextExists(audio->status()));
        QObject *reconnect = buttonWithText(QStringLiteral("Reconnect"));
        QVERIFY(reconnect);
        QVERIFY(reconnect->metaObject()->indexOfSignal("clicked()") >= 0);
        QVERIFY(audio->canReconnect() && !audio->canChoose());
        QVERIFY(!view->isVisible());
    }
    void previewControls_data() {
        QTest::addColumn<QString>("objectName");
        QTest::addColumn<QString>("label");
        QTest::newRow("previewPlay") << QStringLiteral("previewPlay") << QStringLiteral("Play preview");
        QTest::newRow("previewStop") << QStringLiteral("previewStop") << QStringLiteral("Stop");
    }
    void previewControls() {
        QFETCH(QString, objectName);
        QFETCH(QString, label);
        QObject *button = view->rootObject()->findChild<QObject *>(objectName);
        QVERIFY2(button, qPrintable(QStringLiteral("missing required real QML Button: ") + objectName));
        QVERIFY(button->inherits("QQuickButton"));
        QCOMPARE(button->property("text").toString(), label);
        QVERIFY(button->metaObject()->indexOfSignal("clicked()") >= 0);
        QVERIFY(button->property("enabled").isValid());
        QVERIFY2(!button->property("enabled").toBool(), "preview action must be disabled with actual unavailable actor");
    }
    void cleanup() {
        view.reset();
        theme.reset();
        audio.reset();
        appearance.reset();
        runtime.reset();
    }
};
int main(int argc, char **argv) {
    if (argc < 2) return 90;
    appearanceDirectory = QString::fromLocal8Bit(argv[1]);
    for (int i = 1; i + 1 < argc; ++i) argv[i] = argv[i + 1];
    argv[--argc] = nullptr;
    QQuickStyle::setStyle("Basic");
    QGuiApplication application(argc, argv);
    SoundsPreviewComponentTest test;
    return QTest::qExec(&test, argc, argv);
}
#include "preview-component.moc"
