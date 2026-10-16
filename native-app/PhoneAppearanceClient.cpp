#include "PhoneAppearanceClient.h"
#include <QSocketNotifier>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QColor>
#include <QRegularExpression>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <time.h>
#include <cerrno>
#include <cstring>
#include <optional>
#include <limits>
#include <cmath>
#include <algorithm>

namespace {
constexpr quint64 rpcBudget = 500000000;
std::optional<quint64> boot() {
    timespec t{};
    if (::clock_gettime(CLOCK_BOOTTIME,&t) || t.tv_sec < 0 || t.tv_nsec < 0 || t.tv_nsec >= 1000000000) return {};
    const auto seconds = quint64(t.tv_sec), nanos = quint64(t.tv_nsec);
    if (seconds > (std::numeric_limits<quint64>::max()-nanos)/1000000000) return {};
    return seconds*1000000000+nanos;
}
std::optional<quint64> decimal(const QJsonValue &value) {
    if (!value.isString()) return {};
    const auto s=value.toString();
    if (s.isEmpty() || s.size()>20 || (s.size()>1 && s[0]=='0')) return {};
    quint64 out=0;
    for (const auto c:s) {
        if (c<'0' || c>'9') return {};
        const auto digit=quint64(c.unicode()-'0');
        if (out>(std::numeric_limits<quint64>::max()-digit)/10) return {};
        out=out*10+digit;
    }
    return out;
}
const QStringList keys{"background","foreground","surface","border","accent","muted","urgent","positive","ink"};
const QStringList fontKeys{"caption","bodySmall","body","subtitle","title","heading","display","displayLarge"};
const QStringList spacingKeys{"controlGap","panelPadding","panelGap","rowGap","controlPaddingX","controlPaddingY"};
bool pixels(const QJsonValue &value,int minimum) {
    const double n=value.toDouble(-1);
    return value.isDouble() && std::isfinite(n) && n>=minimum
        && n<=std::numeric_limits<int>::max() && std::floor(n)==n;
}
std::optional<QVariantMap> tokens(const QJsonValue &value,const QStringList &names,int minimum) {
    const auto object=value.toObject();
    if (!value.isObject() || object.size()!=names.size()) return {};
    QVariantMap result;
    for (const auto &name:names) {
        if (!pixels(object[name],minimum)) return {};
        result.insert(name,object[name].toInt());
    }
    return result;
}
std::optional<QColor> rgba16(const QJsonValue &value) {
    if (!value.isObject()) return {};
    const auto object=value.toObject();
    if (object.size()!=4) return {};
    for (const auto &key:{"r","g","b","a"}) {
        if (!pixels(object[key],0) || object[key].toDouble()>65535) return {};
    }
    return QColor::fromRgba64(ushort(object["r"].toInt()),ushort(object["g"].toInt()),
                             ushort(object["b"].toInt()),ushort(object["a"].toInt()));
}
}
PhoneAppearanceClient::PhoneAppearanceClient(const QString &socketPath,QObject *parent):QObject(parent),path(socketPath) {
    pollTimer.setInterval(250);
    requestTimer.setSingleShot(true);
    expiryTimer.setSingleShot(true);
    connect(&pollTimer,&QTimer::timeout,this,&PhoneAppearanceClient::refresh);
    connect(&requestTimer,&QTimer::timeout,this,[this] {
        const auto now=boot();
        if (!now || *now>=requestUntil) fail();
        else requestTimer.start(int((requestUntil-*now+999999)/1000000));
    });
    connect(&expiryTimer,&QTimer::timeout,this,&PhoneAppearanceClient::armExpiry);
    pollTimer.start();
    QTimer::singleShot(0,this,&PhoneAppearanceClient::refresh);
}
PhoneAppearanceClient::~PhoneAppearanceClient() { closeSocket(); }
bool PhoneAppearanceClient::available() const {
    const auto now=boot();
    return valid && now && *now<paletteUntil;
}
void PhoneAppearanceClient::invalidate() {
    expiryTimer.stop();
    if (valid) { valid=false;emit changed(); }
}
void PhoneAppearanceClient::closeSocket() {
    ++connectionGeneration;
    for (auto **notifier:{&readNotifier,&writeNotifier}) {
        if (*notifier) { (*notifier)->setEnabled(false);(*notifier)->deleteLater();*notifier=nullptr; }
    }
    if (fd>=0) ::close(fd);
    fd=-1;peerPid=0;stage=Stage::Idle;requestUntil=0;requestTimer.stop();
}
void PhoneAppearanceClient::fail() { closeSocket();invalidate(); }
bool PhoneAppearanceClient::peer() {
    ucred credentials{};socklen_t length=sizeof(credentials);
    if (::getsockopt(fd,SOL_SOCKET,SO_PEERCRED,&credentials,&length) || length!=sizeof(credentials)
        || credentials.uid!=0 || credentials.gid!=0 || credentials.pid<=0) return false;
    peerPid=credentials.pid;
    return true;
}
void PhoneAppearanceClient::refresh() {
    const auto now=boot();
    if (!now) { fail();return; }
    if (valid && *now>=paletteUntil) invalidate();
    if (stage!=Stage::Idle) {
        if (*now>=requestUntil) fail();
        return;
    }
    if (*now>std::numeric_limits<quint64>::max()-rpcBudget) { fail();return; }
    requestUntil=*now+rpcBudget;
    requestTimer.start(500);
    if (fd<0) {
        const auto encoded=path.toLocal8Bit();sockaddr_un address{};address.sun_family=AF_UNIX;
        if (!path.startsWith('/') || encoded.isEmpty() || encoded.contains('\0') || encoded.size()>=int(sizeof(address.sun_path))) { fail();return; }
        std::memcpy(address.sun_path,encoded.constData(),size_t(encoded.size())+1);
        fd=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);
        int one=1;
        if (fd<0 || ::setsockopt(fd,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one))) { fail();return; }
        const auto generation=connectionGeneration;
        readNotifier=new QSocketNotifier(fd,QSocketNotifier::Read,this);
        writeNotifier=new QSocketNotifier(fd,QSocketNotifier::Write,this);
        readNotifier->setEnabled(false);writeNotifier->setEnabled(false);
        connect(readNotifier,&QSocketNotifier::activated,this,[this,generation] { if (generation==connectionGeneration) readable(); });
        connect(writeNotifier,&QSocketNotifier::activated,this,[this,generation] { if (generation==connectionGeneration) writable(); });
        if (::connect(fd,reinterpret_cast<sockaddr*>(&address),sizeof(address))) {
            if (errno!=EINPROGRESS) { fail();return; }
            stage=Stage::Connecting;writeNotifier->setEnabled(true);return;
        }
        if (!peer()) { fail();return; }
    }
    stage=Stage::Sending;
    writable();
}
void PhoneAppearanceClient::writable() {
    const auto now=boot();
    if (!now || *now>=requestUntil || fd<0) { fail();return; }
    if (stage==Stage::Connecting) {
        int error=0;socklen_t length=sizeof(error);
        if (::getsockopt(fd,SOL_SOCKET,SO_ERROR,&error,&length) || error || !peer()) { fail();return; }
        stage=Stage::Sending;
    }
    if (stage!=Stage::Sending) return;
    static const QByteArray query=R"({"op":"snapshot"})";
    const auto count=::send(fd,query.constData(),size_t(query.size()),MSG_DONTWAIT|MSG_NOSIGNAL);
    if (count<0 && (errno==EAGAIN || errno==EINTR)) { writeNotifier->setEnabled(true);return; }
    const auto completed=boot();
    if (count!=query.size() || !completed || *completed>=requestUntil) { fail();return; }
    stage=Stage::Receiving;writeNotifier->setEnabled(false);readNotifier->setEnabled(true);
}
void PhoneAppearanceClient::readable() {
    if (fd<0) return;
    char bytes[4096];alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(16*sizeof(int))];
    iovec vector{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&vector;message.msg_iovlen=1;message.msg_control=control;message.msg_controllen=sizeof(control);
    const auto count=::recvmsg(fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);
    if (count<0 && (errno==EAGAIN || errno==EINTR)) return;
    bool trusted=false,seen=false,invalid=message.msg_flags&(MSG_TRUNC|MSG_CTRUNC);
    for (auto *c=CMSG_FIRSTHDR(&message);count>=0 && c;c=CMSG_NXTHDR(&message,c)) {
        if (c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_CREDENTIALS && c->cmsg_len==CMSG_LEN(sizeof(ucred)) && !seen) {
            seen=true;ucred credentials{};std::memcpy(&credentials,CMSG_DATA(c),sizeof(credentials));
            trusted=credentials.uid==0 && credentials.gid==0 && credentials.pid==peerPid;
        } else {
            invalid=true;
            if (c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_RIGHTS && c->cmsg_len>=CMSG_LEN(0)) {
                const auto handles=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);auto *received=reinterpret_cast<int*>(CMSG_DATA(c));
                for (size_t i=0;i<handles;i++) ::close(received[i]);
            }
        }
    }
    const auto received=boot();
    if (count<=0 || !trusted || invalid || stage!=Stage::Receiving || !received || *received>=requestUntil) { fail();return; }
    QJsonParseError error{};const auto document=QJsonDocument::fromJson(QByteArray(bytes,int(count)),&error);
    const auto value=document.object();const auto until=decimal(value["until_ns"]);
    const auto nextOwner=value["owner"].toString();const auto nextRevision=value["revision"].toDouble(-1);
    static const QRegularExpression ownerPattern("^[0-9a-f]{32}$");
    if (error.error!=QJsonParseError::NoError || !document.isObject() || value.size()!=6
        || !value["schema"].isDouble() || value["schema"].toDouble()!=3 || !ownerPattern.match(nextOwner).hasMatch()
        || !value["revision"].isDouble() || !std::isfinite(nextRevision) || nextRevision<0 || nextRevision>std::numeric_limits<quint32>::max() || std::floor(nextRevision)!=nextRevision
        || !value["available"].isBool() || !until) { fail();return; }
    const bool published=value["available"].toBool();QVariantMap next;
    if (published) {
        const auto appearance=value["appearance"].toObject();
        const auto colors=appearance["palette"].toObject();
        const auto style=appearance["style"].toObject();
        const auto fonts=tokens(style["font_tokens"],fontKeys,1);
        const auto spacing=tokens(style["spacing_tokens"],spacingKeys,0);
        if (!value["appearance"].isObject() || appearance.size()!=2
            || !appearance["palette"].isObject() || colors.size()!=keys.size()
            || !appearance["style"].isObject() || style.size()!=3
            || !pixels(style["base_size"],1) || !fonts || !spacing) { fail();return; }
        for (const auto &key:keys) {
            const auto color=rgba16(colors[key]);
            if (!color) { fail();return; }
            next.insert(key,QVariant::fromValue(*color));
        }
        next.insert("fontScale",style["base_size"].toDouble()/12.0);
        next.insert("fontTokens",*fonts);
        next.insert("spacingTokens",*spacing);
    } else if (!value["appearance"].isNull()) { fail();return; }
    const auto completed=boot();
    if (!completed || *completed>=requestUntil || *completed>=*until
        || (owner==nextOwner && (quint64(nextRevision)<revision || (published && quint64(nextRevision)==revision && !current.isEmpty() && current!=next)))) { fail();return; }
    stage=Stage::Idle;requestTimer.stop();requestUntil=0;
    const bool changed=valid!=published || (published && current!=next);
    owner=nextOwner;revision=quint64(nextRevision);paletteUntil=*until;valid=published;
    if (published) current=next;
    armExpiry();
    if (changed) emit this->changed();
}
void PhoneAppearanceClient::armExpiry() {
    const auto now=boot();
    if (!valid) { expiryTimer.stop();return; }
    if (!now || *now>=paletteUntil) { invalidate();return; }
    const auto remaining=paletteUntil-*now;
    const auto milliseconds=remaining/1000000+(remaining%1000000!=0);
    expiryTimer.start(int(std::min(milliseconds,quint64(std::numeric_limits<int>::max()))));
}
