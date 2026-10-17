#include "MessagesClient.h"
#include "PhoneAppearanceClient.h"
#include "PhoneBatteryClient.h"
#include <QGuiApplication>
#include <QQuickView>
#include <QQuickItem>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QFile>
#include <QDir>
#include <QSaveFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTimer>
#include <QJSValue>
#include <QSocketNotifier>
#include <cerrno>
#include <csignal>
#include <sys/signalfd.h>
#include <unistd.h>
int main(int argc,char**argv){
#ifdef MESSAGES_ISOLATED_OBSERVER
 if(argc!=9)return 64;
#else
 if(argc!=7)return 64;
#endif
 if(getuid()!=11010||geteuid()!=11010)return 65;
 // Block before QGuiApplication creates any platform/render/client threads.
 // Normal actor TERM then requests event-loop exit and owned Qt destruction.
 sigset_t termination;
 if(::sigemptyset(&termination)||::sigaddset(&termination,SIGTERM)
    ||::sigprocmask(SIG_BLOCK,&termination,nullptr))return 70;
 struct TerminationFd {
  int value;
  ~TerminationFd(){if(value>=0)::close(value);}
 } terminationFd{::signalfd(-1,&termination,SFD_NONBLOCK|SFD_CLOEXEC)};
 if(terminationFd.value<0)return 70;
 qputenv("HOME",argv[1]);qputenv("XDG_RUNTIME_DIR",argv[2]);qputenv("WAYLAND_DISPLAY",argv[3]);qputenv("QT_QPA_PLATFORM","wayland");qputenv("PATH","/usr/bin");
 QGuiApplication app(argc,argv);QQuickStyle::setStyle("Basic");
 QSocketNotifier terminate(terminationFd.value,QSocketNotifier::Read);
 QObject::connect(&terminate,&QSocketNotifier::activated,&app,[&]{
  signalfd_siginfo notification{};
  const auto count=::read(terminationFd.value,&notification,sizeof(notification));
  if(count<0&&(errno==EAGAIN||errno==EINTR))return;
  if(count==ssize_t(sizeof(notification))&&notification.ssi_signo==SIGTERM)app.quit();
  else app.exit(70);
 });
 // argv4 is the authenticated public appearance socket, never a HOME path.
 PhoneAppearanceClient appearance(QString::fromLocal8Bit(argv[4]));QQuickView view;
 QQmlComponent component(view.engine(),QUrl::fromLocalFile(QString::fromLocal8Bit(argv[6])+"/PhoneAppearance.qml"));
 QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source",QVariant::fromValue(&appearance)}}));if(!theme)return 66;
 MessagesClient client(QString::fromLocal8Bit(argv[5]));
 PhoneBatteryClient battery;
 view.setTitle("Messages");view.setInitialProperties({{"theme",QVariant::fromValue(theme.data())},{"messagesClient",QVariant::fromValue(&client)},{"batteryClient",QVariant::fromValue(&battery)}});
 view.setResizeMode(QQuickView::SizeRootObjectToView);view.setSource(QUrl("qrc:/messages/MessagesView.qml"));if(view.status()!=QQuickView::Ready)return 67;view.resize(390,844);view.show();
#ifdef MESSAGES_ISOLATED_OBSERVER
 // Fixed root-installed test profile only. Production binary has no observer.
 const QString output=QString::fromLocal8Bit(argv[8]);if(QString::fromLocal8Bit(argv[7])!="--isolated-observe")return 68;
 QTimer observer;observer.setInterval(200);int last=-1;
 QObject::connect(&observer,&QTimer::timeout,&app,[&]{QFile f(output+"/control.json");if(!f.open(QIODevice::ReadOnly)||f.size()>1024)return;const auto control=QJsonDocument::fromJson(f.readAll()).object();const int seq=control["sequence"].toInt(-1);if(seq<0||last==seq)return;auto*root=view.rootObject();QList<QQuickItem*> bodies;auto collect=[&](auto&& self,QQuickItem*item)->void{if(item->objectName()=="messageBody")bodies.append(item);for(auto*child:item->childItems())self(self,child);};collect(collect,root);
  bool plain=!bodies.isEmpty(),literal=false;for(auto*b:bodies){plain=plain&&b->property("textFormat").toInt()==0;literal=literal||b->property("text").toString()=="<b>synthetic plain text</b>";}
  const auto image=view.grabWindow();if(image.isNull())return;
  QJsonObject state{{"sequence",seq},{"pid",qint64(getpid())},{"uid",qint64(getuid())},{"plain",plain},{"literalFixture",literal},{"bodyCount",bodies.size()},{"sourceBodyCount",client.messages().size()},{"conversationCount",client.conversations().size()},{"accessDenied",client.status()=="Messages access denied"},{"available",theme->property("available").toBool()},{"background",theme->property("background").value<QColor>().name()},{"foreground",theme->property("foreground").value<QColor>().name()},{"fontFamily",theme->property("fontFamily").toString()},{"fontTokens",QJsonValue::fromVariant(theme->property("fontTokens").value<QJSValue>().toVariant())},{"pixel",image.pixelColor(2,2).name()}};
  image.save(output+"/window-"+QString::number(seq)+".png");QSaveFile out(output+"/window-"+QString::number(seq)+".json");if(out.open(QIODevice::WriteOnly)){out.write(QJsonDocument(state).toJson());out.commit();last=seq;}
 });observer.start();
#endif
 return app.exec();
}
