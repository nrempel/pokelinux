// Coordinate measurement only: actual Qt Text implicit heights, no app model/window.
#include <QGuiApplication>
#include <QQmlEngine>
#include <QQmlComponent>
#include <QJsonDocument>
#include <QJsonObject>
#include <QScopedPointer>
#include <cstdio>
int main(int argc,char**argv){
 qputenv("QT_QPA_PLATFORM","offscreen");qputenv("QSG_RHI_BACKEND","software");
 QGuiApplication app(argc,argv);QQmlEngine engine;QJsonObject heights;
 for(const auto size:{10,12,14,16}){
  QQmlComponent text(&engine);text.setData(QByteArray("import QtQuick\nText { text: \"Local connection: Disabled\"; font.family: \"monospace\"; font.pixelSize: ")+QByteArray::number(size)+" }",QUrl());
  QScopedPointer<QObject> object(text.create());if(!object)return 2;
  const auto height=object->property("implicitHeight").toDouble();if(!(height>0&&height<64))return 3;
  heights.insert(QString::number(size),height);
 }
 const auto bytes=QJsonDocument(QJsonObject{{"actual_Qt_Text_implicit_heights",heights},{"normal_app_getters",false},{"fallback_font",QStringLiteral("monospace")}}).toJson(QJsonDocument::Compact);
 return std::fwrite(bytes.constData(),1,bytes.size(),stdout)==size_t(bytes.size())?0:4;
}
