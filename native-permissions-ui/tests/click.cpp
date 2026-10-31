// Unshown real QML buttons -> normal Qt adapter -> Rust C ABI -> authored root peer.
// No compositor consent, native output capture or actual actor qualification.
#include "PermissionsClient.h"
#include "PhoneAppearanceClient.h"
#include <QGuiApplication>
#include <QEventLoop>
#include <QQuickView>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQmlComponent>
#include <QScopedPointer>
#include <QSignalSpy>
#include <QMouseEvent>
#include <QPointingDevice>
#include <cmath>
#include <QDir>
#include <QStringList>
#include <cstdio>
#include <stdexcept>
#include <sys/stat.h>
#include <unistd.h>
#include <time.h>
static void require(bool v,const char *message){if(!v)throw std::runtime_error(message);}
static uint64_t now(){timespec t{};require(::clock_gettime(CLOCK_BOOTTIME,&t)==0,"clock");return uint64_t(t.tv_sec)*1000000000ULL+uint64_t(t.tv_nsec);}
template<class F> static void until(F f,const char *message){const auto end=now()+900000000ULL;while(now()<end){QCoreApplication::processEvents(QEventLoop::AllEvents,1);if(f())return;::usleep(1000);}throw std::runtime_error(message);}
static QQuickItem *item(QQuickView &v,const char *name){
 QList<QQuickItem*> all{v.rootObject()};QQuickItem *found=nullptr;
 for(qsizetype i=0;i<all.size();++i){if(all[i]->objectName()==QLatin1String(name)){require(!found,"duplicate button");found=all[i];}all.append(all[i]->childItems());}
 require(found,"missing button");return found;
}
static QStringList texts(QQuickView &v){
 QStringList result;QList<QQuickItem*> all{v.rootObject()};
 for(qsizetype i=0;i<all.size();++i){const auto text=all[i]->property("text");if(text.isValid())result.append(text.toString());all.append(all[i]->childItems());}
 return result;
}
static QString buttonName(QQuickView &v,const QString &text){
 QString result;QList<QQuickItem*> all{v.rootObject()};
 for(qsizetype i=0;i<all.size();++i){if(all[i]->inherits("QQuickButton")&&all[i]->property("text").toString()==text){require(result.isEmpty(),"duplicate scoped choice");result=all[i]->objectName();}all.append(all[i]->childItems());}
 require(!result.isEmpty(),"authenticated scope choice missing");return result;
}
static void click(QQuickView &v,const char *name){
 QList<QQuickItem*> all{v.rootObject()};for(qsizetype i=0;i<all.size();++i)all.append(all[i]->childItems());
 for(qsizetype i=all.size();i>0;--i)all[i-1]->ensurePolished();
 v.contentItem()->ensurePolished();
 auto *b=item(v,name);require(b->inherits("QQuickButton")&&b->isEnabled()&&b->width()>0&&b->height()>0,"enabled real button");
 const auto center=b->mapToScene(QPointF(b->width()/2,b->height()/2));
 require(QRectF(0,0,v.width(),v.height()).contains(center),"button outside viewport");
 QSignalSpy clicked(b,b->metaObject()->method(b->metaObject()->indexOfSignal("clicked()")));
 QMouseEvent press(QEvent::MouseButtonPress,center,center,center,Qt::LeftButton,Qt::LeftButton,Qt::NoModifier,QPointingDevice::primaryPointingDevice());
 press.setTimestamp(now()/1000000ULL);
 QMouseEvent release(QEvent::MouseButtonRelease,center,center,center,Qt::LeftButton,Qt::NoButton,Qt::NoModifier,QPointingDevice::primaryPointingDevice());
 release.setTimestamp(now()/1000000ULL);
 const bool sentPress=QCoreApplication::sendEvent(&v,&press);
 const bool sentRelease=QCoreApplication::sendEvent(&v,&release);
 require(sentPress&&sentRelease&&clicked.count()==1,"one real press/release click");
 require(!v.isVisible()&&!v.isExposed(),"component must remain unshown");
 std::printf("CLICK %s\n",name);std::fflush(stdout);
}
int main(int argc,char **argv){
 try{
  require(argc==5&&getuid()==11045&&geteuid()==11045,"actual UID11045 and fixed test oracle required");
  // This chooses assertions only; the normal client still receives only its socket path.
  const QString oracle=QString::fromLocal8Bit(argv[4]);
  require(oracle==QStringLiteral("media_route")||oracle==QStringLiteral("network_control"),"fixed expected scope");
  const bool network=oracle==QStringLiteral("network_control");
  const QString subject=network?QStringLiteral("Settings"):QStringLiteral("Sounds");
  const QString permission=network?QStringLiteral("Network control"):QStringLiteral("Audio");
  const QString allow=network?QStringLiteral("Allow network control"):QStringLiteral("Allow audio");
  const QString revoke=network?QStringLiteral("Revoke network control"):QStringLiteral("Revoke audio");
  const QString hint=network?QStringLiteral("Let Settings control the local connection."):QStringLiteral("Let Sounds play audio.");
  const QString nextAction=network?QStringLiteral("Enable the local connection in Settings after allowing control."):QStringLiteral("Start playback in Sounds after allowing audio.");
  const QString runtime=QStringLiteral("/tmp/permissions-qt-runtime-")+QString::number(getpid());
  require(QDir().mkpath(runtime),"runtime");require(::chmod(runtime.toLocal8Bit().constData(),0700)==0,"runtime mode");qputenv("XDG_RUNTIME_DIR",runtime.toLocal8Bit());qputenv("HOME",runtime.toLocal8Bit());
  QGuiApplication app(argc,argv);QQuickStyle::setStyle("Basic");
  PrPermissionsHandle *invalid=nullptr;
  require(pr_permissions_new_v1(nullptr,0,&invalid)==PR_PERMISSIONS_INVALID&&!invalid,"null path ABI");
  require(pr_permissions_view_v2(nullptr,nullptr)==PR_PERMISSIONS_INVALID,"null view ABI");
  require(pr_permissions_tick_v1(nullptr)==PR_PERMISSIONS_INVALID,"null tick ABI");
  PhoneAppearanceClient appearance(QString::fromLocal8Bit(argv[2]));QQuickView view;
  QQmlComponent themeComponent(view.engine(),QUrl::fromLocalFile(QString::fromLocal8Bit(argv[3])));
  QScopedPointer<QObject> theme(themeComponent.createWithInitialProperties({{"source",QVariant::fromValue(&appearance)}}));require(bool(theme),"shared appearance component");
  PermissionsClient client(QString::fromLocal8Bit(argv[1]));
  struct ClearView{QQuickView &v;~ClearView(){v.setSource(QUrl());}} clearView{view};
  view.setInitialProperties({{"theme",QVariant::fromValue(theme.data())},{"permissionClient",QVariant::fromValue(&client)}});
  view.setResizeMode(QQuickView::SizeViewToRootObject);view.setSource(QUrl(QStringLiteral("qrc:/permissions/PermissionsView.qml")));
  require(view.status()==QQuickView::Ready,"normal Permissions QML ready");
  view.rootObject()->setWidth(1080.0/2.625);view.rootObject()->setHeight(2424.0/2.625);
  view.resize(int(std::ceil(1080.0/2.625)),int(std::ceil(2424.0/2.625)));
  const auto initial=texts(view);
  require(!initial.contains(QStringLiteral("Sounds"))&&!initial.contains(QStringLiteral("Settings"))&&
   !initial.contains(QStringLiteral("Allow audio"))&&!initial.contains(QStringLiteral("Allow network control")),
   "unavailable authenticated scope must not display a fixed target or permission");
  require(client.allowed()==-1&&!client.canSet(),"unknown scope cannot grant control");
  client.setActive(true);until([&]{return client.allowed()==0&&client.canSet();},"initial saved permission unavailable");
  auto current=texts(view);require(current.contains(subject),"authenticated target label");
  require(current.contains(hint)&&current.contains(nextAction),"authenticated scope explanation and next action");
  require(current.contains(permission+QStringLiteral(" not allowed")),"authenticated permission label");
  require(!current.contains(network?QStringLiteral("Sounds"):QStringLiteral("Settings")),"other fixed target must not be displayed");
  const auto allowName=buttonName(view,allow).toLatin1();
  const auto revokeName=buttonName(view,revoke).toLatin1();
  click(view,allowName.constData());require(client.allowed()!=1,"Allow must not optimistically display success");
  until([&]{return client.allowed()==1&&client.canSet();},"Allow click did not reach confirmed Rust result");
  require(texts(view).contains(permission+QStringLiteral(" allowed")),"confirmed scoped Allowed label");
  click(view,revokeName.constData());require(client.allowed()!=0,"Revoke must not optimistically display success");
  until([&]{return client.allowed()==0&&client.canSet();},"Revoke click did not reach confirmed Rust result");
  require(texts(view).contains(permission+QStringLiteral(" not allowed")),"confirmed scoped Revoke label");
  client.setActive(false);require(client.allowed()==-1&&!client.canSet(),"inactive confirmation cleared");
  client.close();require(client.allowed()==-1&&!client.canSet(),"closed state");
  std::puts("PASS real QML Allow/Revoke through Rust C ABI; unshown component only");return 0;
 }catch(const std::exception &e){std::fprintf(stderr,"FAIL %s\n",e.what());return 1;}
}
