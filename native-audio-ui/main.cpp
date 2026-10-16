#include "AudioClient.h"
#include "PhoneAppearanceClient.h"
#include <QGuiApplication>
#include <QQuickView>
#include <QQuickItem>
#include <QScreen>
#include <QJsonArray>
#include <cstdio>
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
 if(argc!=7)return 64;
 if(getuid()!=11042||geteuid()!=11042)return 65;
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
 QGuiApplication app(argc,argv);QGuiApplication::setDesktopFileName("org.omarchy.Sounds");QQuickStyle::setStyle("Basic");
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
 // argv5 is the root actor app.sock, authenticated per packet.
 AudioClient client(QString::fromLocal8Bit(argv[5]),0);
 view.setTitle("Sounds");view.setInitialProperties({{"theme",QVariant::fromValue(theme.data())},{"audioClient",QVariant::fromValue(&client)}});
 view.setResizeMode(QQuickView::SizeRootObjectToView);view.setSource(QUrl("qrc:/sounds/SoundsView.qml"));if(view.status()!=QQuickView::Ready)return 67;view.resize(390,844);
 // Bounded normal-app observation, with no input/control interface.
 // The independent compositor/capture observer still qualifies native pixels.
 QObject::connect(&view,&QQuickWindow::frameSwapped,&app,
  [&view,previous=QByteArray{},reports=0]() mutable {
   if(!view.isExposed() || reports>=8 || !view.screen())return;
   const auto *screen=view.screen();
   const auto rect=[](QRect r){return QJsonArray{r.x(),r.y(),r.width(),r.height()};};
   const QJsonObject state{{"kind","sounds_window_metadata"},{"schema",1},
    {"pid",qint64(getpid())},{"title",view.title()},
    {"window",QJsonObject{{"exposed",view.isExposed()},{"visible",view.isVisible()},
     {"geometry",rect(view.geometry())},{"dpr",view.devicePixelRatio()}}},
    {"screen",QJsonObject{{"name",screen->name()},{"geometry",rect(screen->geometry())},
     {"available_geometry",rect(screen->availableGeometry())},{"dpr",screen->devicePixelRatio()},
     {"physical_mm",QJsonArray{screen->physicalSize().width(),screen->physicalSize().height()}},
     {"physical_dpi",QJsonArray{screen->physicalDotsPerInchX(),screen->physicalDotsPerInchY()}},
     {"logical_dpi",QJsonArray{screen->logicalDotsPerInchX(),screen->logicalDotsPerInchY()}}}}};
   const auto line=QJsonDocument(state).toJson(QJsonDocument::Compact);
   if(line==previous || line.size()>2048)return;
   previous=line;++reports;
   std::fwrite(line.constData(),1,size_t(line.size()),stdout);std::fputc('\n',stdout);std::fflush(stdout);
  });
 view.show();
 return app.exec();
}
