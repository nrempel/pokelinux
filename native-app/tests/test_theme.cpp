#include <QtTest>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickView>
#include <QQuickItem>
#include <QTemporaryDir>
#include <QSaveFile>
#include <QDir>
#include "PhoneThemeFiles.h"

class ThemeTest : public QObject {
    Q_OBJECT
    static void write(const QString &path, const QByteArray &body) {
        QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
        QSaveFile file(path); QVERIFY(file.open(QIODevice::WriteOnly));
        QCOMPARE(file.write(body), body.size()); QVERIFY(file.commit());
    }
    static QString themePath(const QString &home) {
        return home + "/.local/state/omarchy/current/theme/";
    }
private slots:
    void pinnedColorSyntaxAndUnknownRoles() {
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto path=themePath(home.path());
        write(path+"colors.toml", "background=#101010\nforeground=#eeeeee\naccent=#abcdef\ncolor1='#990000'\nred='#aa3344'\n");
        write(path+"shell.toml", "[popups]\nborder='constructor'\ntext='toString'\n");
        PhoneThemeFiles files(home.path()); QQmlEngine engine;
        QQmlComponent c(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneTheme.qml")));
        QScopedPointer<QObject> palette(c.createWithInitialProperties({{"source",QVariant::fromValue(&files)}}));
        QVERIFY2(palette,qPrintable(c.errorString()));
        QVERIFY(palette->property("available").toBool());
        QCOMPARE(palette->property("urgent").value<QColor>(),QColor("#aa3344"));
        QCOMPARE(palette->property("border").value<QColor>(),QColor("#abcdef"));
        QCOMPARE(palette->property("foreground").value<QColor>(),QColor("#eeeeee"));
    }
    void palettePrecedenceAndInvalidReplacement() {
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto path=themePath(home.path());
        write(path+"colors.toml", "foreground='#eeeeee'\nbackground='#111111'\naccent='#abcdef'\ncolor4='#123456'\ncolor1='#dd3344'\ncolor2='#44bb66'\ncolor8='#888888'\n");
        write(path+"shell.toml", "[popups]\nbackground = '#333333'\nborder = 'accent'\n[font]\nbase-size=18\n");
        write(home.path()+"/.config/omarchy/shell.toml", "[popups]\nborder='#556677'\n");
        PhoneThemeFiles files(home.path()); QQmlEngine engine;
        QQmlComponent c(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneTheme.qml")));
        QScopedPointer<QObject> palette(c.createWithInitialProperties({{"source",QVariant::fromValue(&files)}}));
        QVERIFY2(palette,qPrintable(c.errorString()));
        QCOMPARE(palette->property("accent").value<QColor>(),QColor("#abcdef"));
        QCOMPARE(palette->property("surface").value<QColor>(),QColor("#333333"));
        QCOMPARE(palette->property("border").value<QColor>(),QColor("#556677"));
        QCOMPARE(palette->property("positive").value<QColor>(),QColor("#44bb66"));
        QCOMPARE(palette->property("fontScale").toDouble(),1.5);
        write(path+"colors.toml", "background='broken'\nforeground='#ffffff'\n");
        QTRY_VERIFY(!palette->property("available").toBool());
        QCOMPARE(palette->property("background").value<QColor>(),QColor("#111111"));
        write(path+"colors.toml", "color0='#fafafa'\ncolor7='#222222'\ncolor4='#335577'\ncolor1='#990011'\n");
        QTRY_VERIFY(palette->property("available").toBool());
        QCOMPARE(palette->property("background").value<QColor>(),QColor("#fafafa"));
        QCOMPARE(palette->property("muted").value<QColor>(),QColor("#222222"));
        QCOMPARE(palette->property("positive").value<QColor>(),QColor("#335577"));
    }
    void roleCyclesAndAlphaAreBounded() {
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto path=themePath(home.path());
        write(path+"colors.toml", "background='#000000'\nforeground='#ffffff'\naccent='#abcdef'\n");
        write(path+"shell.toml", "[popups]\nbackground='180deg #ffffff #000000'\nbackground-alpha=0.5\nborder='popups.text'\ntext='popups.border'\n");
        PhoneThemeFiles files(home.path()); QQmlEngine engine;
        QQmlComponent c(&engine,QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneTheme.qml")));
        QScopedPointer<QObject> palette(c.createWithInitialProperties({{"source",QVariant::fromValue(&files)}}));
        QVERIFY2(palette,qPrintable(c.errorString()));
        QVERIFY(palette->property("available").toBool());
        QCOMPARE(palette->property("surface").value<QColor>(),QColor("#808080"));
        QCOMPARE(palette->property("border").value<QColor>(),QColor("#abcdef"));
        QCOMPARE(palette->property("foreground").value<QColor>(),QColor("#ffffff"));
    }
    void realFilesRecolorExistingViewWithoutLosingIntent() {
        QTest::failOnWarning(QRegularExpression(".*(TypeError|ReferenceError).*"));
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto path=themePath(home.path());
        write(path+"colors.toml", "background='#111111'\nforeground='#eeeeee'\naccent='#ddaa55'\ncolor1='#ff6688'\ncolor2='#88bb66'\n");
        PhoneThemeFiles files(home.path()); QQuickView view;
        QQmlComponent c(view.engine(),QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/PhoneTheme.qml")));
        QScopedPointer<QObject> palette(c.createWithInitialProperties({{"source",QVariant::fromValue(&files)}}));
        QVERIFY2(palette,qPrintable(c.errorString()));
        view.setInitialProperties({{"injectedTheme",QVariant::fromValue(palette.data())}});
        view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.setSource(QUrl::fromLocalFile(QStringLiteral(PHONE_SOURCE_DIR "/tests/ViewFixture.qml")));
        QVERIFY(view.status()==QQuickView::Ready);
        view.resize(390,844); view.show(); QVERIFY(QTest::qWaitForWindowExposed(&view));
        auto *root=view.rootObject();
        auto *number=root->findChild<QQuickItem*>("numberField"); QVERIFY(number);
        number->setProperty("text","5550100");
        auto *actions=qvariant_cast<QObject*>(root->property("fixtureActions")); QVERIFY(actions);
        actions->setProperty("unresolved",true); actions->setProperty("outcome","unknown");
        actions->setProperty("intentId",QString(32,'a')); actions->setProperty("canObserve",false);
        QSignalSpy submitted(root,SIGNAL(actionSubmitted(QString)));
        QTest::qWait(50);
        QVERIFY(view.grabWindow().save("/results/phone-theme-files-dark.png"));
        write(path+"colors.toml", "background='#faf4e8'\nforeground='#292522'\naccent='#8a5b1f'\ncolor1='#a83246'\ncolor2='#406524'\n");
        QTRY_COMPARE(palette->property("background").value<QColor>(),QColor("#faf4e8"));
        QTest::qWait(50);
        QCOMPARE(view.rootObject(),root);
        QCOMPARE(number->property("text").toString(),QString("5550100"));
        QCOMPARE(actions->property("intentId").toString(),QString(32,'a'));
        QVERIFY(actions->property("unresolved").toBool());
        QCOMPARE(submitted.count(),0);
        QCOMPARE(view.grabWindow().pixelColor(5,5),QColor("#faf4e8"));
        QVERIFY(view.grabWindow().save("/results/phone-theme-files-light.png"));
        view.setSource(QUrl());
    }
};
QTEST_MAIN(ThemeTest)
#include "test_theme.moc"
