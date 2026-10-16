// Production PhoneAppearanceClient; real forked Linux peers and kernel credentials.
#include "PhoneAppearanceClient.h"
#include <QCoreApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QTemporaryDir>
#include <QElapsedTimer>
#include <QThread>
#include <QQmlEngine>
#include <QQmlComponent>
#include <QColor>
#include <QJSValue>
#include <QScopedPointer>
#include <QStringList>
#include <QVariant>
#include <limits>
#include <cmath>
#include <atomic>
#include <cstdio>
#include <cstring>
#include <memory>
#include <new>
#include <poll.h>
#include <grp.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

static quint64 boot(){timespec t{};if(::clock_gettime(CLOCK_BOOTTIME,&t))std::abort();return quint64(t.tv_sec)*1000000000+quint64(t.tv_nsec);}
template<class F> static bool wait(F predicate,int ms=1000){QElapsedTimer t;t.start();while(t.elapsed()<ms){QCoreApplication::processEvents(QEventLoop::AllEvents,2);if(predicate())return true;QThread::msleep(1);}return predicate();}
static void events(int ms){wait([]{return false;},ms);}
static QByteArray actualLastWire;
static std::atomic<bool> delayReceive{false};
static std::atomic<int> delayedBytes{0};
extern "C" ssize_t __real_recvmsg(int,msghdr*,int);
extern "C" ssize_t __wrap_recvmsg(int fd,msghdr* message,int flags){
 const auto n=__real_recvmsg(fd,message,flags);
 if(n>0&&message->msg_iovlen==1&&size_t(n)<=message->msg_iov[0].iov_len)actualLastWire=QByteArray(static_cast<const char*>(message->msg_iov[0].iov_base),int(n));
 if(n>0&&delayReceive.exchange(false)){delayedBytes=int(n);QThread::msleep(550);}
 return n;
}
enum Mode {Good,Expired,Hold,Malformed,Unavailable,BadUntil,FloatRevision,RollbackRevision,BadOwner,Rights,Oversized,Delegated,PacketUid,SchemaOne,TopUnknown,TopMissing,AppearanceUnknown,AppearanceMissingPalette,AppearanceMissingStyle,AppearanceType,PaletteType,StyleType,StyleUnknown,StyleMissingBase,StyleMissingFonts,StyleMissingSpacings,FontMapType,SpacingMapType,BaseMutation,FontMutation,SpacingMutation,SchemaTwo,RgbaMutation,RgbaRoleMutation,RgbaUpdate,IntegralFloatRgba,IntegralExponentRgba};
struct Shared {std::atomic<int> mode{Good},requests{0},replies{0},uid{-1},requestTrusted{0},delegatePid{0},packetUid{0};std::atomic<int> styleVariant{0},revision{1},tokenIndex{0},mutation{0},wireSchema{0},wireStyleVariant{0},wireRevision{0},wireBytes{0},colorVariant{0},roleIndex{0},channelIndex{0},wireColorVariant{0};std::atomic<bool> stopping{false};};
static const QStringList paletteKeys{"background","foreground","surface","border","accent","muted","urgent","positive","ink"};
static const QStringList channelKeys{"r","g","b","a"};
static const QStringList rgbaMutationNames{"missing","extra","type","fractional","negative","overflow","bool","null"};
static const QStringList roleMutationNames{"missing","extra","type","bool","null","array"};
static QJsonObject legacyPalette(const char* background){return {{"background",background},{"foreground","#a9b1d6"},{"surface",background},{"border","#7aa2f7"},{"accent","#7aa2f7"},{"muted","#414868"},{"urgent","#f7768e"},{"positive","#9ece6a"},{"ink",background}};}
static QJsonObject rgbaOf(const QColor& color){const auto c=color.rgba64();return {{"r",c.red()},{"g",c.green()},{"b",c.blue()},{"a",c.alpha()}};}
static QJsonObject palette(const char* background,int variant=0){
 const auto legacy=legacyPalette(background);QJsonObject colors;
 for(int i=0;i<paletteKeys.size();++i){
  auto value=rgbaOf(QColor(legacy[paletteKeys[i]].toString()));
  if(variant==1)value=QJsonObject{{"r",(i*6007+1234)%65536},{"g",(i*8011+2345)%65536},{"b",(i*9001+3456)%65536},{"a",i==0?32768:(i==1?32896:65535-i)}};
  else if(variant==2)value=QJsonObject{{"r",0},{"g",65535},{"b",i+1},{"a",i%2?0:65535}};
  else if(variant==3){if(i==0)value["a"]=32768;else if(i==1)value["a"]=32896;}
  else if(variant==4&&i==0){value["r"]=value["r"].toInt()+1;value["a"]=65534;}
  colors.insert(paletteKeys[i],value);
 }
 return colors;
}
static bool canonicalRgba16(const QJsonObject& colors){
 if(colors.size()!=paletteKeys.size())return false;
 for(const auto& role:paletteKeys){
  if(!colors[role].isObject()||colors[role].toObject().size()!=4)return false;
  const auto value=colors[role].toObject();
  for(const auto& channel:channelKeys){const auto n=value[channel].toDouble(-1);if(!value[channel].isDouble()||!std::isfinite(n)||n<0||n>65535||std::floor(n)!=n)return false;}
 }
 return true;
}
static QJsonObject observedCppColors(const QVariantMap& snapshot){QJsonObject colors;for(const auto& role:paletteKeys)colors.insert(role,rgbaOf(snapshot.value(role).value<QColor>()));return colors;}
static QJsonObject observedQmlColors(QObject* theme){QJsonObject colors;if(theme)for(const auto& role:paletteKeys)colors.insert(role,rgbaOf(theme->property(role.toUtf8().constData()).value<QColor>()));return colors;}
static void mutateRgba(QJsonObject& value,const QString& channel,int mutation){
 if(mutation==0)value.remove(channel);else if(mutation==1)value.insert(channel+"Unexpected",1);
 else if(mutation==2)value.insert(channel,"65535");else if(mutation==3)value.insert(channel,0.5);
 else if(mutation==4)value.insert(channel,-1);else if(mutation==5)value.insert(channel,65536);
 else if(mutation==6)value.insert(channel,true);else if(mutation==7)value.insert(channel,QJsonValue(QJsonValue::Null));else std::abort();
}
static const QStringList fontKeys{"caption","bodySmall","body","subtitle","title","heading","display","displayLarge"};
static const QStringList spacingKeys{"controlGap","panelPadding","panelGap","rowGap","controlPaddingX","controlPaddingY"};
static const QStringList mutationNames{"unknown","missing","type","negative","fractional","overflow","zero","bool","null"};
static QJsonObject style(int variant){
 const int primaryFonts[]{15,16,18,19,21,24,36,42},nextFonts[]{20,22,24,26,28,32,48,56};
 const int primarySpacing[]{12,27,21,12,15,9},nextSpacing[]{16,36,28,16,20,12};
 QJsonObject fonts,spacing;
 for(int i=0;i<fontKeys.size();++i)fonts.insert(fontKeys[i],variant==2?std::numeric_limits<int>::max():(variant==1?nextFonts[i]:primaryFonts[i]));
 for(int i=0;i<spacingKeys.size();++i)spacing.insert(spacingKeys[i],variant==2?std::numeric_limits<int>::max():(variant==3?0:(variant==1?nextSpacing[i]:primarySpacing[i])));
 return {{"base_size",variant==2?std::numeric_limits<int>::max():(variant==1?24:18)},{"font_tokens",fonts},{"spacing_tokens",spacing}};
}
static void mutate(QJsonObject& object,const QString& key,int mutation){
 if(mutation==0)object.insert("unexpected",1);
 else if(mutation==1)object.remove(key);
 else if(mutation==2)object.insert(key,"12");
 else if(mutation==3)object.insert(key,-1);
 else if(mutation==4)object.insert(key,1.5);
 else if(mutation==5)object.insert(key,2147483648.0);
 else if(mutation==6)object.insert(key,0);
 else if(mutation==7)object.insert(key,true);
 else if(mutation==8)object.insert(key,QJsonValue(QJsonValue::Null));
 else std::abort();
}
static QJsonObject variantObject(const QVariant& value){
 if(value.metaType().id()==qMetaTypeId<QJSValue>())return QJsonObject::fromVariantMap(value.value<QJSValue>().toVariant().toMap());
 return QJsonObject::fromVariantMap(value.toMap());
}
static QJsonObject observeQml(QObject* theme){
 QJsonObject observed{{"object_created",theme!=nullptr}};
 if(!theme)return observed;
 observed["available"]=theme->property("available").toBool();
 observed["background"]=theme->property("background").value<QColor>().name();
 observed["fontScale"]=theme->property("fontScale").toDouble();
 observed["fontFamily"]=theme->property("fontFamily").toString();
 observed["monoFamily"]=theme->property("monoFamily").toString();
 observed["fontTokens"]=variantObject(theme->property("fontTokens"));
 observed["spacingTokens"]=variantObject(theme->property("spacingTokens"));
 QJsonObject fonts,spacing;
 for(const auto& key:fontKeys){QVariant value;const bool invoked=QMetaObject::invokeMethod(theme,"fontPixel",Q_RETURN_ARG(QVariant,value),Q_ARG(QVariant,QVariant(key)),Q_ARG(QVariant,QVariant(999)));if(!invoked)std::abort();fonts[key]=value.toInt();}
 for(const auto& key:spacingKeys){QVariant value;const bool invoked=QMetaObject::invokeMethod(theme,"spacingPixel",Q_RETURN_ARG(QVariant,value),Q_ARG(QVariant,QVariant(key)),Q_ARG(QVariant,QVariant(999)));if(!invoked)std::abort();spacing[key]=value.toInt();}
 QVariant fallbackFont,fallbackSpacing;
 const bool fontInvoked=QMetaObject::invokeMethod(theme,"fontPixel",Q_RETURN_ARG(QVariant,fallbackFont),Q_ARG(QVariant,QVariant(QStringLiteral("unlisted"))),Q_ARG(QVariant,QVariant(20)));
 const bool spacingInvoked=QMetaObject::invokeMethod(theme,"spacingPixel",Q_RETURN_ARG(QVariant,fallbackSpacing),Q_ARG(QVariant,QVariant(QStringLiteral("unlisted"))),Q_ARG(QVariant,QVariant(17)));
 observed["fontPixel"]=fonts;observed["spacingPixel"]=spacing;
 observed["fallback_font_invoked"]=fontInvoked;observed["fallback_font"]=fallbackFont.toInt();
 observed["fallback_spacing_invoked"]=spacingInvoked;observed["fallback_spacing"]=fallbackSpacing.toInt();
 return observed;
}

class Peer {
public:
 Shared* stats;pid_t pid=-1,waitedPid=-1;int waitedStatus=-1;QByteArray path;bool waited=false;const char* background;
 Peer(const QString& name,uid_t uid=0,const char* color="#1a1b26",bool shortLease=false):path(name.toLocal8Bit()),background(color){
  void* map=::mmap(nullptr,sizeof(Shared),PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);if(map==MAP_FAILED)std::abort();stats=new(map) Shared;
  int ready[2];if(::pipe2(ready,O_CLOEXEC))std::abort();pid=::fork();if(pid<0)std::abort();
  if(pid==0){
   ::close(ready[0]);if(uid!=0&&(::setgroups(0,nullptr)||::setgid(uid)||::setuid(uid)))_exit(71);stats->uid=int(::getuid());
   const int listener=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0);sockaddr_un addr{};addr.sun_family=AF_UNIX;std::memcpy(addr.sun_path,path.constData(),size_t(path.size())+1);
   int one=1;::setsockopt(listener,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one));if(listener<0||::bind(listener,reinterpret_cast<sockaddr*>(&addr),sizeof(addr))||::listen(listener,1))_exit(72);
   const char ok='1';if(::write(ready[1],&ok,1)!=1)_exit(73);::close(ready[1]);int connection=-1;const auto until=boot()+5000000000;
   while(boot()<until&&!stats->stopping){
    if(connection<0){connection=::accept4(listener,nullptr,nullptr,SOCK_CLOEXEC|SOCK_NONBLOCK);if(connection>=0)::setsockopt(connection,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one));}
    if(connection>=0){
     char bytes[4096];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))];iovec v{bytes,sizeof(bytes)};msghdr msg{};msg.msg_iov=&v;msg.msg_iovlen=1;msg.msg_control=control;msg.msg_controllen=sizeof(control);
     const auto n=::recvmsg(connection,&msg,MSG_DONTWAIT);
     if(n==0){::close(connection);connection=-1;}
     if(n>0){bool trusted=false;for(auto*c=CMSG_FIRSTHDR(&msg);c;c=CMSG_NXTHDR(&msg,c))if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS){ucred u{};std::memcpy(&u,CMSG_DATA(c),sizeof(u));trusted=u.uid==0&&u.pid==::getppid();}
      const auto query=QJsonDocument::fromJson(QByteArray(bytes,int(n))).object();const bool valid=trusted&&!(msg.msg_flags&(MSG_TRUNC|MSG_CTRUNC))&&query.size()==1&&query["op"].toString()=="snapshot";
      stats->requestTrusted=valid;++stats->requests;const auto mode=Mode(stats->mode.load());if(mode==Hold)continue;
      const int selectedState=stats->styleVariant.load(),selectedVariant=selectedState%10;
      QJsonObject value{{"schema",3},{"owner",uid==0&&color[1]=='1'?"11111111111111111111111111111111":"33333333333333333333333333333333"},{"revision",selectedState>=10?2:stats->revision.load()},{"until_ns",QString::number(mode==Expired?boot()-1:boot()+(shortLease?350000000:2000000000))},{"available",true},{"appearance",QJsonObject{{"palette",palette(color,stats->colorVariant.load())},{"style",style(selectedVariant)}}}};
      if(mode==SchemaTwo){value["schema"]=2;auto a=value["appearance"].toObject();a["palette"]=legacyPalette(color);value["appearance"]=a;}
      if(mode==RgbaMutation||mode==RgbaRoleMutation){
       auto a=value["appearance"].toObject();auto colors=a["palette"].toObject();const auto role=paletteKeys[stats->roleIndex];
       if(mode==RgbaMutation){auto channels=colors[role].toObject();mutateRgba(channels,channelKeys[stats->channelIndex],stats->mutation);colors[role]=channels;}
       else{const int mutation=stats->mutation;if(mutation==0)colors.remove(role);else if(mutation==1)colors.insert(role+"Unexpected",colors[role]);else if(mutation==2)colors[role]="#ffffff";else if(mutation==3)colors[role]=true;else if(mutation==4)colors[role]=QJsonValue(QJsonValue::Null);else if(mutation==5)colors[role]=QJsonArray{0,0,0,65535};else std::abort();}
       a["palette"]=colors;value["appearance"]=a;
      }
      if(mode==Malformed){auto a=value["appearance"].toObject();auto p=a["palette"].toObject();p["background"]="#badbad00";a["palette"]=p;value["appearance"]=a;}
      if(mode==Unavailable){value["available"]=false;value["appearance"]=QJsonValue(QJsonValue::Null);}
      if(mode==BadUntil)value["until_ns"]="+00099999999999999999999";
      if(mode==FloatRevision)value["revision"]=1.5;
      if(mode==RollbackRevision)value["revision"]=0;
      if(mode==BadOwner)value["owner"]="not-an-owner";
      if(mode==SchemaOne){value["schema"]=1;value.remove("appearance");value["palette"]=palette(color);}
      if(mode==TopUnknown)value["unexpected"]=true;
      if(mode==TopMissing)value.remove("appearance");
      if(mode==AppearanceType)value["appearance"]="invalid";
      if(mode>=AppearanceUnknown&&mode<=SpacingMutation&&mode!=AppearanceType){
       auto a=value["appearance"].toObject();auto st=a["style"].toObject();
       if(mode==AppearanceUnknown)a["unexpected"]=true;
       else if(mode==AppearanceMissingPalette)a.remove("palette");
       else if(mode==AppearanceMissingStyle)a.remove("style");
       else if(mode==PaletteType)a["palette"]="invalid";
       else if(mode==StyleType)a["style"]="invalid";
       else {
        if(mode==StyleUnknown)st["unexpected"]=1;
        else if(mode==StyleMissingBase)st.remove("base_size");
        else if(mode==StyleMissingFonts)st.remove("font_tokens");
        else if(mode==StyleMissingSpacings)st.remove("spacing_tokens");
        else if(mode==FontMapType)st["font_tokens"]="invalid";
        else if(mode==SpacingMapType)st["spacing_tokens"]="invalid";
        else if(mode==BaseMutation)mutate(st,"base_size",stats->mutation);
        else if(mode==FontMutation){auto tokens=st["font_tokens"].toObject();mutate(tokens,fontKeys[stats->tokenIndex],stats->mutation);st["font_tokens"]=tokens;}
        else if(mode==SpacingMutation){auto tokens=st["spacing_tokens"].toObject();mutate(tokens,spacingKeys[stats->tokenIndex],stats->mutation);st["spacing_tokens"]=tokens;}
        a["style"]=st;
       }
       value["appearance"]=a;
      }
      QByteArray wire=QJsonDocument(value).toJson(QJsonDocument::Compact);
      if(mode==IntegralFloatRgba||mode==IntegralExponentRgba){
       const auto colors=value["appearance"].toObject()["palette"].toObject();
       for(const auto& role:paletteKeys)for(const auto& channel:channelKeys){const auto token=QByteArray("\"")+channel.toUtf8()+"\":"+QByteArray::number(colors[role].toObject()[channel].toInt());for(const char delimiter:{',','}'})wire.replace(token+delimiter,token+(mode==IntegralFloatRgba?".0":"e0")+delimiter);}
      }
      if(mode==Oversized)wire.append(QByteArray(4096,' '));
      ssize_t sent;
      if(mode==Rights){const int extra=::dup(listener);alignas(cmsghdr) char rights[CMSG_SPACE(sizeof(int))]{};iovec output{wire.data(),size_t(wire.size())};msghdr out{};out.msg_iov=&output;out.msg_iovlen=1;out.msg_control=rights;out.msg_controllen=sizeof(rights);auto*c=CMSG_FIRSTHDR(&out);c->cmsg_level=SOL_SOCKET;c->cmsg_type=SCM_RIGHTS;c->cmsg_len=CMSG_LEN(sizeof(int));std::memcpy(CMSG_DATA(c),&extra,sizeof(extra));sent=::sendmsg(connection,&out,MSG_NOSIGNAL);::close(extra);}
      else if(mode==Delegated){const auto delegated=::fork();if(delegated<0)_exit(74);if(delegated==0){::send(connection,wire.constData(),size_t(wire.size()),MSG_NOSIGNAL);_exit(0);}stats->delegatePid=int(delegated);int status;while(::waitpid(delegated,&status,0)<0&&errno==EINTR){}sent=wire.size();}
      else{if(mode==PacketUid){if(::setgroups(0,nullptr)||::setgid(1000)||::setuid(1000))_exit(75);stats->packetUid=int(::getuid());}sent=::send(connection,wire.constData(),size_t(wire.size()),MSG_NOSIGNAL);}
      if(sent==wire.size()){stats->wireSchema=value["schema"].toInt();stats->wireStyleVariant=selectedVariant;stats->wireRevision=value["revision"].toInt();stats->wireBytes=wire.size();stats->wireColorVariant=stats->colorVariant.load();++stats->replies;}
     }
    }
    ::poll(nullptr,0,2);
   }
   if(connection>=0){::close(connection);}::close(listener);_exit(0);
  }
  ::close(ready[1]);char status=0;pollfd input{ready[0],POLLIN,0};if(::poll(&input,1,1000)<=0||::read(ready[0],&status,1)!=1||status!='1')std::abort();::close(ready[0]);
 }
 void stop(){if(waited)return;stats->stopping=true;do{waitedPid=::waitpid(pid,&waitedStatus,0);}while(waitedPid<0&&errno==EINTR);waited=waitedPid==pid;if(!waited)std::abort();::unlink(path.constData());}
 ~Peer(){stop();stats->~Shared();::munmap(stats,sizeof(Shared));}
};
int main(int argc,char**argv){
 QCoreApplication app(argc,argv);if(argc!=3||::getuid()!=0)return 64;const QString mode=QString::fromLocal8Bit(argv[1]);QTemporaryDir dir;if(!dir.isValid())return 70;::chmod(dir.path().toLocal8Bit().constData(),0777);
 const auto socket=dir.path()+"/appearance.sock";auto peer=std::make_unique<Peer>(socket,0,"#1a1b26",mode=="cached-expiry"||mode=="delayed-consumption");auto reader=std::make_unique<PhoneAppearanceClient>(socket);
 int changes=0;QObject::connect(reader.get(),&PhoneAppearanceClient::changed,[&]{changes++;});
 const bool initial=wait([&]{return reader->available()&&reader->snapshot()["background"].value<QColor>().name()=="#1a1b26";},900);
 QJsonObject result{{"case",mode},{"qt_version",qVersion()},{"initial_publication",initial},{"actual_root_peer_pid",qint64(peer->pid)},{"actual_root_peer_uid",peer->stats->uid.load()},{"actual_requests",peer->stats->requests.load()},{"actual_replies",peer->stats->replies.load()},{"actual_request_credentials_and_shape",bool(peer->stats->requestTrusted.load())}};
 bool passed=initial&&peer->stats->requestTrusted;
 result["negative_injection_attempted"]=false;
 if(mode=="publication"||mode=="qml-full-style"){
  const auto snapshot=reader->snapshot();
  const auto expected=style(0);
  const bool normalized=initial&&snapshot.size()==12&&snapshot["foreground"].value<QColor>().name()=="#a9b1d6"&&snapshot["positive"].value<QColor>().name()=="#9ece6a"&&snapshot["fontScale"].toDouble()==1.5&&QJsonObject::fromVariantMap(snapshot["fontTokens"].toMap())==expected["font_tokens"].toObject()&&QJsonObject::fromVariantMap(snapshot["spacingTokens"].toMap())==expected["spacing_tokens"].toObject();
  result["normalized_flat_palette_scale_and_all_tokens"]=normalized;
  result["actual_CPP_snapshot"]=QJsonObject::fromVariantMap(snapshot);
  QQmlEngine engine;QQmlComponent component(&engine,QUrl::fromLocalFile(QString::fromLocal8Bit(argv[2])+"/PhoneAppearance.qml"));QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source",QVariant::fromValue(reader.get())}}));
  const auto observed=observeQml(theme.data());result["actual_QML_observations"]=observed;
  QJsonArray qmlErrors;for(const auto& error:component.errors())qmlErrors.append(error.toString());result["QML_component_errors"]=qmlErrors;
  const bool qml=theme&&observed["available"].toBool()&&observed["background"].toString()=="#1a1b26"&&observed["fontScale"].toDouble()==1.5&&observed["fontTokens"].toObject()==expected["font_tokens"].toObject()&&observed["spacingTokens"].toObject()==expected["spacing_tokens"].toObject()&&observed["fontPixel"].toObject()==expected["font_tokens"].toObject()&&observed["spacingPixel"].toObject()==expected["spacing_tokens"].toObject()&&observed["fallback_font"].toInt()==30&&observed["fallback_spacing"].toInt()==17&&observed["fontFamily"].toString()=="monospace"&&observed["monoFamily"].toString()=="monospace";
  result["actual_QML_full_style_token_scale_contract"]=qml;passed=initial&&normalized&&qml;
 }else if(initial){
  if(mode=="rgba16-exact"||mode=="rgba16-boundaries"||mode=="rgba16-alpha-only-new-revision"||mode=="rgba16-alpha-only-same-revision"||mode=="rgba16-one-unit-new-revision"||mode=="rgba16-integral-float"||mode=="rgba16-integral-exponent"||mode=="rgba16-final-liveness"){
   QQmlEngine engine;QQmlComponent component(&engine,QUrl::fromLocalFile(QString::fromLocal8Bit(argv[2])+"/PhoneAppearance.qml"));QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source",QVariant::fromValue(reader.get())}}));
   QJsonArray errors;for(const auto& error:component.errors())errors.append(error.toString());result["QML_component_errors"]=errors;
   const bool baseline=theme&&theme->property("available").toBool()&&observedCppColors(reader->snapshot())==palette("#1a1b26")&&observedQmlColors(theme.data())==palette("#1a1b26");
   int requests=peer->stats->requests;const int previousChanges=changes;
   if(mode=="rgba16-final-liveness"){
    peer->stats->roleIndex=0;peer->stats->channelIndex=3;peer->stats->mutation=5;peer->stats->mode=RgbaMutation;
    const bool queried=wait([&]{return peer->stats->requests>requests;},600);const bool refused=wait([&]{return !reader->available();},650);
    result["actual_malformed_then_unavailable"]=queried&&refused;requests=peer->stats->requests;
   }
   const int variant=mode=="rgba16-boundaries"?2:(mode.startsWith("rgba16-alpha-only")?3:(mode=="rgba16-one-unit-new-revision"?4:1));
   peer->stats->colorVariant=variant;if(mode!="rgba16-alpha-only-same-revision")peer->stats->revision=2;
   peer->stats->mode=mode=="rgba16-integral-float"?IntegralFloatRgba:(mode=="rgba16-integral-exponent"?IntegralExponentRgba:RgbaUpdate);
   const auto expected=palette("#1a1b26",variant);
   const bool queried=wait([&]{return peer->stats->requests>requests&&peer->stats->wireColorVariant==variant;},600);
   if(mode=="rgba16-alpha-only-same-revision"){
    const bool refused=wait([&]{return !reader->available();},650);passed=baseline&&queried&&refused&&observedCppColors(reader->snapshot())==palette("#1a1b26");result["alpha_only_same_revision_refused"]=passed;
   }else{
    const bool exact=wait([&]{return reader->available()&&observedCppColors(reader->snapshot())==expected&&theme&&theme->property("available").toBool()&&observedQmlColors(theme.data())==expected;},900);
    bool qcolors=true;for(const auto& role:paletteKeys)qcolors=qcolors&&reader->snapshot().value(role).metaType().id()==qMetaTypeId<QColor>();
    const auto wire=QJsonDocument::fromJson(actualLastWire).object();const auto receivedColors=wire["appearance"].toObject()["palette"].toObject();const bool validWire=wire["schema"].toInt()==3&&canonicalRgba16(receivedColors)&&receivedColors==expected;
    passed=baseline&&queried&&exact&&qcolors&&validWire&&errors.isEmpty()&&changes>previousChanges;
    if(mode=="rgba16-final-liveness")passed=passed&&result["actual_malformed_then_unavailable"].toBool();
    if(mode=="rgba16-integral-float")passed=passed&&actualLastWire.contains(".0");
    if(mode=="rgba16-integral-exponent")passed=passed&&actualLastWire.contains("e0");
    result["exact_all_RGBA16_CPP_and_QML"]=exact;result["CPP_colors_are_QColor"]=qcolors;result["actual_canonical_schema3_wire"]=validWire;result["actual_received_wire_RGBA16"]=receivedColors;result["expected_RGBA16"]=expected;
   }
   result["actual_CPP_RGBA16"]=observedCppColors(reader->snapshot());result["actual_QML_RGBA16"]=observedQmlColors(theme.data());result["actual_QML_available"]=bool(theme&&theme->property("available").toBool());
   result["alpha_precision_values_expected"]=QJsonArray{32768,32896};result["probe_is_normal_window"]=false;
  }else if(mode=="style-same-revision"||mode=="style-new-revision"||mode=="spacing-zero-accepted"||mode=="int32-boundary-accepted"){
   const int requests=peer->stats->requests;const int previousChanges=changes;
   const int variant=mode=="int32-boundary-accepted"?2:(mode=="spacing-zero-accepted"?3:1);
   peer->stats->styleVariant=mode=="style-same-revision"?variant:10+variant;
   const bool queried=wait([&]{return peer->stats->requests>requests;},600);
   result["style_only_actual_query"]=queried;result["palette_unchanged"]=true;result["owner_unchanged"]=true;
   if(mode=="style-same-revision"){
    const bool refused=wait([&]{return !reader->available();},650);const auto snapshot=reader->snapshot();
    passed=queried&&refused&&snapshot["fontScale"].toDouble()==1.5&&QJsonObject::fromVariantMap(snapshot["fontTokens"].toMap())==style(0)["font_tokens"].toObject()&&QJsonObject::fromVariantMap(snapshot["spacingTokens"].toMap())==style(0)["spacing_tokens"].toObject();
    result["same_owner_same_revision_changed_style_refused"]=passed;
   }else{
    const auto expected=style(variant);const double scale=expected["base_size"].toDouble()/12.0;
    const bool accepted=wait([&]{const auto snapshot=reader->snapshot();return reader->available()&&snapshot["fontScale"].toDouble()==scale&&QJsonObject::fromVariantMap(snapshot["fontTokens"].toMap())==expected["font_tokens"].toObject()&&QJsonObject::fromVariantMap(snapshot["spacingTokens"].toMap())==expected["spacing_tokens"].toObject();},900);
    passed=queried&&accepted&&changes>previousChanges;result["same_owner_new_revision_style_accepted"]=passed;
   }
   result["actual_CPP_snapshot_after_style_change"]=QJsonObject::fromVariantMap(reader->snapshot());
  }else if(mode=="delayed-consumption"){
   peer->stats->mode=Hold;QThread::msleep(400);passed=!reader->available();result["available_refuses_expired_lease_before_timer_delivery"]=passed;
  }else if(mode=="peer-uid"||mode=="restart"||mode=="disconnect"){
   const auto oldPid=peer->pid;peer->stop();passed=wait([&]{return !reader->available();},550);result["disconnect_invalidates"]=passed;
   if(mode!="disconnect"){
    peer=std::make_unique<Peer>(socket,mode=="peer-uid"?1000:0,"#eff1f5");result["new_actual_peer_pid"]=qint64(peer->pid);result["new_actual_peer_uid"]=peer->stats->uid.load();
    if(mode=="restart"){passed=passed&&wait([&]{return reader->available()&&reader->snapshot()["background"].value<QColor>().name()=="#eff1f5";});result["new_owner_current_publication"]=passed&&peer->pid!=oldPid;}
    else{events(600);passed=passed&&!reader->available()&&reader->snapshot()["background"].value<QColor>().name()!="#eff1f5";result["actual_UID1000_refused"]=passed;}
   }
  }else{
   Mode next=Good;
   if(mode=="expired")next=Expired;else if(mode=="cached-expiry"||mode=="held")next=Hold;
   else if(mode=="malformed")next=Malformed;else if(mode=="unavailable")next=Unavailable;
   else if(mode=="until-syntax")next=BadUntil;else if(mode=="revision-float")next=FloatRevision;
   else if(mode=="revision-rollback")next=RollbackRevision;
   else if(mode=="owner")next=BadOwner;else if(mode=="rights")next=Rights;else if(mode=="oversized")next=Oversized;
   else if(mode=="sender-pid")next=Delegated;
   else if(mode=="sender-uid")next=PacketUid;
   else if(mode=="late-real-receive"){next=Good;delayReceive=true;}
   else if(mode=="schema1")next=SchemaOne;else if(mode=="schema2")next=SchemaTwo;
   else if(mode=="top-unknown")next=TopUnknown;else if(mode=="top-missing")next=TopMissing;
   else if(mode=="appearance-unknown")next=AppearanceUnknown;else if(mode=="appearance-missing-palette")next=AppearanceMissingPalette;else if(mode=="appearance-missing-style")next=AppearanceMissingStyle;
   else if(mode=="appearance-type")next=AppearanceType;else if(mode=="palette-type")next=PaletteType;
   else if(mode=="style-type")next=StyleType;else if(mode=="style-unknown")next=StyleUnknown;
   else if(mode=="style-missing-base")next=StyleMissingBase;else if(mode=="style-missing-fonts")next=StyleMissingFonts;else if(mode=="style-missing-spacings")next=StyleMissingSpacings;
   else if(mode=="font-map-type")next=FontMapType;else if(mode=="spacing-map-type")next=SpacingMapType;
   else if(mode=="font-map-unknown"){next=FontMutation;peer->stats->tokenIndex=0;peer->stats->mutation=0;}
   else if(mode=="spacing-map-unknown"){next=SpacingMutation;peer->stats->tokenIndex=0;peer->stats->mutation=0;}
   else{
    const auto pieces=mode.split('-');
    if(pieces.size()==4&&pieces[0]=="rgba"&&pieces[1]=="role"){
     const int role=paletteKeys.indexOf(pieces[2]),mutation=roleMutationNames.indexOf(pieces[3]);if(role<0||mutation<0)return 64;next=RgbaRoleMutation;peer->stats->roleIndex=role;peer->stats->mutation=mutation;
    }else if(pieces.size()==4&&pieces[0]=="rgba"){
     const int role=paletteKeys.indexOf(pieces[1]),channel=channelKeys.indexOf(pieces[2]),mutation=rgbaMutationNames.indexOf(pieces[3]);if(role<0||channel<0||mutation<0)return 64;next=RgbaMutation;peer->stats->roleIndex=role;peer->stats->channelIndex=channel;peer->stats->mutation=mutation;
    }else if(pieces.size()==2&&pieces[0]=="base"){const int mutation=mutationNames.indexOf(pieces[1]);if(mutation<0)return 64;next=BaseMutation;peer->stats->mutation=mutation;}
    else if(pieces.size()==3&&(pieces[0]=="font"||pieces[0]=="spacing")){const auto& keys=pieces[0]=="font"?fontKeys:spacingKeys;const int index=keys.indexOf(pieces[1]),mutation=mutationNames.indexOf(pieces[2]);if(index<0||mutation<0)return 64;next=pieces[0]=="font"?FontMutation:SpacingMutation;peer->stats->tokenIndex=index;peer->stats->mutation=mutation;}
    else return 64;
   }
   result["negative_injection_attempted"]=true;
   const int requests=peer->stats->requests;peer->stats->mode=next;const auto began=boot();
   const bool queried=wait([&]{return peer->stats->requests>requests;},600);const bool refused=wait([&]{return !reader->available();},650);
   passed=queried&&refused;result["negative_actual_query"]=queried;result["availability_invalidated"]=refused;result["elapsed_boottime_ms"]=qint64((boot()-began)/1000000);
   if(mode=="cached-expiry"){passed=passed&&(boot()-began)<650000000;result["cached_lease_expires_before_held_rpc_budget"]=passed;}
   if(mode=="sender-pid"){result["actual_delegated_root_sender_pid"]=peer->stats->delegatePid.load();passed=passed&&peer->stats->delegatePid>0&&peer->stats->delegatePid!=peer->pid;}
   if(mode=="sender-uid"){result["actual_packet_sender_uid"]=peer->stats->packetUid.load();passed=passed&&peer->stats->packetUid==1000;}
   if(mode=="late-real-receive"){result["actual_receive_bytes_before_550ms_return_delay"]=delayedBytes.load();passed=passed&&delayedBytes>0;}
  }
 }
 result["changed_signals"]=changes;result["passed"]=passed;reader.reset();peer->stop();result["actual_peer_wait_completed"]=peer->waited;result["actual_peer_wait_pid"]=qint64(peer->waitedPid);result["actual_peer_wait_status"]=peer->waitedStatus;result["actual_peer_exit_zero"]=WIFEXITED(peer->waitedStatus)&&WEXITSTATUS(peer->waitedStatus)==0;result["actual_wire_color_variant_sent"]=peer->stats->wireColorVariant.load();result["actual_wire_schema_sent"]=peer->stats->wireSchema.load();result["actual_wire_style_variant_sent"]=peer->stats->wireStyleVariant.load();result["actual_wire_revision_sent"]=peer->stats->wireRevision.load();result["actual_wire_bytes_sent"]=peer->stats->wireBytes.load();passed=passed&&peer->waited&&WIFEXITED(peer->waitedStatus)&&WEXITSTATUS(peer->waitedStatus)==0;result["passed"]=passed;peer.reset();
 std::printf("%s\n",QJsonDocument(result).toJson(QJsonDocument::Compact).constData());return passed?0:101;
}
