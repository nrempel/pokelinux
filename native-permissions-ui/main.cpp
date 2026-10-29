#include "PermissionsClient.h"
#include "PhoneAppearanceClient.h"
#include <QGuiApplication>
#include <QQuickView>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QScopedPointer>
#include <QSocketNotifier>
#include <cerrno>
#include <csignal>
#include <sys/signalfd.h>
#include <unistd.h>
int main(int argc,char **argv){
 if(argc!=7)return 64;
 if(getuid()!=11045 || geteuid()!=11045 || getgid()!=11045 || getegid()!=11045)return 65;
 for(int i=1;i<7;++i)if(argv[i][0]!='/')return 64;
 sigset_t termination;
 if(::sigemptyset(&termination)||::sigaddset(&termination,SIGTERM)||::sigprocmask(SIG_BLOCK,&termination,nullptr))return 70;
 struct OwnedFd{int value;~OwnedFd(){if(value>=0)::close(value);}} fd{::signalfd(-1,&termination,SFD_NONBLOCK|SFD_CLOEXEC)};
 if(fd.value<0)return 70;
 qputenv("HOME",argv[1]);qputenv("XDG_RUNTIME_DIR",argv[2]);qputenv("WAYLAND_DISPLAY",argv[3]);
 qputenv("QT_QPA_PLATFORM","wayland");qputenv("PATH","/usr/bin");
 QGuiApplication::setApplicationName(QStringLiteral("pokelinux-permissions"));
 QGuiApplication app(argc,argv);app.setQuitOnLastWindowClosed(false);
 QGuiApplication::setDesktopFileName(QStringLiteral("org.omarchy.Permissions"));
 QQuickStyle::setStyle("Basic");
 PhoneAppearanceClient appearance(QString::fromLocal8Bit(argv[4]));
 QQuickView view;
 QQmlComponent component(view.engine(),QUrl::fromLocalFile(QString::fromLocal8Bit(argv[6])+"/PhoneAppearance.qml"));
 QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source",QVariant::fromValue(&appearance)}}));
 if(!theme)return 66;
 PermissionsClient client(QString::fromLocal8Bit(argv[5]));
 view.setTitle(QStringLiteral("Permissions"));
 QObject::connect(&view,&QWindow::activeChanged,&client,[&]{client.setActive(view.isActive());});
 QSocketNotifier terminate(fd.value,QSocketNotifier::Read);
 bool exiting=false;
 const auto stop=[&](int code){if(exiting)return;exiting=true;terminate.setEnabled(false);client.close();view.hide();app.exit(code);};
 QObject::connect(&app,&QGuiApplication::lastWindowClosed,&app,[&]{stop(0);});
 QObject::connect(&terminate,&QSocketNotifier::activated,&app,[&]{
  signalfd_siginfo event{};const auto n=::read(fd.value,&event,sizeof(event));
  if(n<0&&(errno==EAGAIN||errno==EINTR))return;
  stop(n==ssize_t(sizeof(event))&&event.ssi_signo==SIGTERM?0:70);
 });
 view.setInitialProperties({{"theme",QVariant::fromValue(theme.data())},{"permissionClient",QVariant::fromValue(&client)}});
 view.setResizeMode(QQuickView::SizeRootObjectToView);view.setSource(QUrl(QStringLiteral("qrc:/permissions/PermissionsView.qml")));
 if(view.status()!=QQuickView::Ready){client.close();return 67;}
 view.showMaximized();const int result=app.exec();
 client.close();view.setSource(QUrl());return result;
}
