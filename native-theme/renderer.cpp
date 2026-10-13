// Own conformance probe: production PhoneThemeFiles/PhoneTheme/PhoneView renderer.
// Synthetic call and number draft only. No enrolled service or outside action.
#include <QGuiApplication>
#include <QQuickView>
#include <QQuickItem>
#include <QQmlComponent>
#include <QFile>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJSValue>
#include <QSaveFile>
#include <QTimer>
#include <QFont>
#include <QFontInfo>
#include <QFontMetrics>
#include <QImage>
#include <QQuickStyle>
#include "PhoneThemeFiles.h"
int main(int argc,char **argv) {
    QGuiApplication app(argc,argv);
    if (argc!=4) return 64;
    const QString home=QString::fromLocal8Bit(argv[1]),source=QString::fromLocal8Bit(argv[2]),out=QString::fromLocal8Bit(argv[3]);
    QQuickStyle::setStyle("Basic");
    PhoneThemeFiles files(home); QQuickView view;
    QQmlComponent themeComponent(view.engine(),QUrl::fromLocalFile(source+"/PhoneTheme.qml"));
    QScopedPointer<QObject> theme(themeComponent.createWithInitialProperties({{"source",QVariant::fromValue(&files)}}));
    if (!theme) { qCritical("Theme failed: %s",qPrintable(themeComponent.errorString())); return 65; }
    view.setTitle("Native theme conformance · synthetic call");
    view.setInitialProperties({{"injectedTheme",QVariant::fromValue(theme.data())}});
    view.setResizeMode(QQuickView::SizeRootObjectToView);
    view.setSource(QUrl::fromLocalFile(source+"/tests/ViewFixture.qml"));
    if (view.status()!=QQuickView::Ready) return 66;
    view.resize(390,844); view.show();
    auto *root=view.rootObject(); auto *number=root->findChild<QQuickItem*>("numberField");
    auto *actions=qvariant_cast<QObject*>(root->property("fixtureActions"));
    if (!number||!actions) return 67;
    number->setProperty("text","5550100");
    actions->setProperty("controlsSnapshot",QVariantMap{{"generation",QString(32,'d')},{"revision",7},{"controls_allowed",false},
        {"calls",QVariantList{QVariantMap{{"id",1},{"state",4},{"actions",QVariantList{}}}}}});
    actions->setProperty("unresolved",true); actions->setProperty("outcome","unknown");
    actions->setProperty("intentId",QString(32,'a')); actions->setProperty("canObserve",false);
    int last=-1;
    QTimer poll; poll.setInterval(100);
    QObject::connect(&poll,&QTimer::timeout,&app,[&] {
        QFile f(out+"/control.json"); if (!f.open(QIODevice::ReadOnly)||f.size()>4096) return;
        const auto c=QJsonDocument::fromJson(f.readAll()).object();
        if (c.value("quit").toBool()) { app.quit(); return; }
        const int seq=c.value("sequence").toInt(-1); if (seq<0||seq==last) return;
        const auto image=view.grabWindow(); if (image.isNull()) return;
        QJsonObject state{{"sequence",seq},{"pid",qint64(QCoreApplication::applicationPid())},
            {"sameRoot",root==view.rootObject()},{"draft",number->property("text").toString()},
            {"intentId",actions->property("intentId").toString()},{"unresolved",actions->property("unresolved").toBool()},
            {"callTitle",root->findChild<QQuickItem*>() ? "synthetic state retained" : ""},
            {"available",theme->property("available").toBool()},
            {"pixel",image.pixelColor(5,5).name()},{"files",QJsonObject::fromVariantMap(files.snapshot())}};
        for (auto name:{"background","foreground","accent","fontFamily","monoFamily","fontScale","fontTokens","spacingTokens"}) {
            auto v=theme->property(name);
            if (v.metaType() == QMetaType::fromType<QColor>()) state[name]=v.value<QColor>().name();
            else state[name]=QJsonValue::fromVariant(v.metaType()==QMetaType::fromType<QJSValue>() ? v.value<QJSValue>().toVariant() : v);
        }
        const auto font=number->property("font").value<QFont>();
        state["numberFont"]=QJsonObject{{"requestedFamily",font.family()},{"resolvedFamily",QFontInfo(font).family()},
            {"pixelSize",font.pixelSize()},{"advance",QFontMetrics(font).horizontalAdvance("5550100")}};
        image.save(out+"/native-"+QString::number(seq)+".png");
        QSaveFile output(out+"/native-"+QString::number(seq)+".json");
        if(output.open(QIODevice::WriteOnly)) { output.write(QJsonDocument(state).toJson()); output.commit(); last=seq; }
    });
    poll.start();
    return app.exec();
}
