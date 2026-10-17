#include "PhoneAppearanceClient.h"
#include <QSocketNotifier>
#include <QColor>
#include <QStringList>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <cerrno>
#include <cstring>

namespace {
const QStringList paletteKeys{"background","foreground","surface","border","accent","muted","urgent","positive","ink"};
const QStringList fontKeys{"caption","bodySmall","body","subtitle","title","heading","display","displayLarge"};
const QStringList spacingKeys{"controlGap","panelPadding","panelGap","rowGap","controlPaddingX","controlPaddingY"};

QVariantMap projected(const PrAppearanceViewV1 &view) {
    QVariantMap result;
    if (!view.has_snapshot) return result;
    for (int i=0;i<paletteKeys.size();++i) {
        const auto &color=view.palette[i];
        result.insert(paletteKeys[i],QVariant::fromValue(QColor::fromRgba64(color.r,color.g,color.b,color.a)));
    }
    QVariantMap fonts,spacing;
    for (int i=0;i<fontKeys.size();++i) fonts.insert(fontKeys[i],int(view.font_tokens[i]));
    for (int i=0;i<spacingKeys.size();++i) spacing.insert(spacingKeys[i],int(view.spacing_tokens[i]));
    result.insert("fontScale",view.font_scale);
    result.insert("fontTokens",fonts);
    result.insert("spacingTokens",spacing);
    return result;
}

void arm(QTimer &timer,uint32_t milliseconds) {
    if (milliseconds) timer.start(int(milliseconds));
    else timer.stop();
}
}

PhoneAppearanceClient::PhoneAppearanceClient(const QString &socketPath,QObject *parent):QObject(parent),path(socketPath) {
    requestTimer.setSingleShot(true);
    expiryTimer.setSingleShot(true);
    connect(&pollTimer,&QTimer::timeout,this,&PhoneAppearanceClient::refresh);
    connect(&requestTimer,&QTimer::timeout,this,&PhoneAppearanceClient::tick);
    connect(&expiryTimer,&QTimer::timeout,this,&PhoneAppearanceClient::tick);
    if (pr_appearance_client_new_v1(&policy)!=PR_APPEARANCE_OK || !policy) return;
    PrAppearanceViewV1 view{};
    if (pr_appearance_client_view_v1(policy,&view)!=PR_APPEARANCE_OK) return;
    // This independent periodic timer starts once. Replies never reset its phase.
    if (view.poll_interval_ms) pollTimer.start(int(view.poll_interval_ms));
    QTimer::singleShot(0,this,&PhoneAppearanceClient::refresh);
}

PhoneAppearanceClient::~PhoneAppearanceClient() {
    pollTimer.stop();
    requestTimer.stop();
    expiryTimer.stop();
    if (policy) {
        pr_appearance_client_stop_v1(policy);
        drain(false);
    }
    // Local resources must be retired even if policy admission refused.
    closeSocket();
    pr_appearance_client_free_v1(policy);
    policy=nullptr;
}

bool PhoneAppearanceClient::available() const {
    uint32_t value=0;
    return policy && pr_appearance_client_available_v1(policy,&value)==PR_APPEARANCE_OK && value!=0;
}

void PhoneAppearanceClient::refresh() {
    if (!policy) return;
    pr_appearance_client_refresh_v1(policy);
    drain();
}

void PhoneAppearanceClient::tick() {
    if (!policy) return;
    pr_appearance_client_tick_v1(policy);
    drain();
}

void PhoneAppearanceClient::notified(uint64_t generation,uint32_t event) {
    if (!policy || pr_appearance_client_ready_v1(policy,generation,event)!=PR_APPEARANCE_OK) return;
    drain();
}

void PhoneAppearanceClient::drain(bool notify) {
    bool retriedClosing=false;
    for (;;) {
        PrAppearanceStepV1 plan{};
        const auto result=pr_appearance_client_step_v1(policy,&plan);
        if (result==PR_APPEARANCE_OK) {
            execute(plan);
            retriedClosing=false;
            continue;
        }
        if (result==PR_APPEARANCE_REFUSED || result==PR_APPEARANCE_EXPIRED) {
            PrAppearanceViewV1 view{};
            if (pr_appearance_client_view_v1(policy,&view)==PR_APPEARANCE_OK
                && view.stage==PR_APPEARANCE_CLOSING) {
                // Expiry/refusal leaves a Rust CLOSE decision to execute.
                if (!retriedClosing) { retriedClosing=true;continue; }
                // Exhausted identities cannot authorize a fresh effect. Retire
                // local resources without fabricating a CLOSED report.
                closeSocket();
            } else {
                // A terminal refusal may have exhausted the effect identity.
                // Retire native resources even when no CLOSE plan can be minted.
                closeSocket();
            }
        }
        break;
    }
    PrAppearanceViewV1 view{};
    if (pr_appearance_client_view_v1(policy,&view)==PR_APPEARANCE_OK) {
        // Exhaustion can become terminal before step, including refresh/report.
        // Local retirement does not require a newly issued effect identity.
        if (view.stage==PR_APPEARANCE_STOPPED) closeSocket();
        applyView(view,notify);
    }
}

void PhoneAppearanceClient::applyView(const PrAppearanceViewV1 &view,bool notify) {
    current=projected(view);
    if (readNotifier) readNotifier->setEnabled(view.read_enabled!=0);
    if (writeNotifier) writeNotifier->setEnabled(view.write_enabled!=0);
    if (!view.poll_interval_ms) pollTimer.stop();
    arm(requestTimer,view.request_timer_ms);
    arm(expiryTimer,view.expiry_timer_ms);
    const bool changedSequence=deliveredChangeSequence!=view.change_sequence;
    deliveredChangeSequence=view.change_sequence;
    // All effects have completed and copied state is visible before a slot can
    // reenter or destroy this QObject. Do not access any members after emission.
    if (notify && changedSequence) emit changed();
}

void PhoneAppearanceClient::closeSocket() {
    for (auto **notifier:{&readNotifier,&writeNotifier}) {
        if (*notifier) {
            (*notifier)->setEnabled(false);
            (*notifier)->disconnect(this);
            (*notifier)->deleteLater();
            *notifier=nullptr;
        }
    }
    if (fd>=0) ::close(fd);
    fd=-1;
    peerPid=0;
}

bool PhoneAppearanceClient::peer() {
    ucred credentials{};
    socklen_t length=sizeof(credentials);
    if (::getsockopt(fd,SOL_SOCKET,SO_PEERCRED,&credentials,&length) || length!=sizeof(credentials)
        || credentials.uid!=0 || credentials.gid!=0 || credentials.pid<=0) return false;
    peerPid=credentials.pid;
    return true;
}

void PhoneAppearanceClient::execute(const PrAppearanceStepV1 &plan) {
    switch (plan.action) {
    case PR_APPEARANCE_CONNECT: {
        const auto encoded=path.toLocal8Bit();
        sockaddr_un address{};
        address.sun_family=AF_UNIX;
        if (!path.startsWith('/') || encoded.isEmpty() || encoded.contains('\0')
            || encoded.size()>=int(sizeof(address.sun_path))) {
            closeSocket();
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECT_FAILED_CLOSED,0);
            return;
        }
        std::memcpy(address.sun_path,encoded.constData(),size_t(encoded.size())+1);
        fd=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);
        int one=1;
        if (fd<0 || ::setsockopt(fd,SOL_SOCKET,SO_PASSCRED,&one,sizeof(one))) {
            closeSocket();
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECT_FAILED_CLOSED,0);
            return;
        }
        const auto generation=plan.connection_generation;
        readNotifier=new QSocketNotifier(fd,QSocketNotifier::Read,this);
        writeNotifier=new QSocketNotifier(fd,QSocketNotifier::Write,this);
        readNotifier->setEnabled(false);
        writeNotifier->setEnabled(false);
        connect(readNotifier,&QSocketNotifier::activated,this,[this,generation] {
            notified(generation,PR_APPEARANCE_READ_READY);
        });
        connect(writeNotifier,&QSocketNotifier::activated,this,[this,generation] {
            notified(generation,PR_APPEARANCE_WRITE_READY);
        });
        if (::connect(fd,reinterpret_cast<sockaddr*>(&address),sizeof(address))) {
            if (errno==EINPROGRESS) {
                pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECT_PENDING,0);
                return;
            }
            closeSocket();
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECT_FAILED_CLOSED,0);
            return;
        }
        if (!peer()) {
            closeSocket();
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECT_FAILED_CLOSED,0);
            return;
        }
        pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CONNECTED,0);
        return;
    }
    case PR_APPEARANCE_FINISH_CONNECT: {
        int error=0;
        socklen_t length=sizeof(error);
        const bool connected=fd>=0 && !::getsockopt(fd,SOL_SOCKET,SO_ERROR,&error,&length)
            && length==sizeof(error) && !error && peer();
        pr_appearance_client_report_v1(policy,&plan,connected?PR_APPEARANCE_CONNECTED:PR_APPEARANCE_IO_FAILED,0);
        return;
    }
    case PR_APPEARANCE_SEND: {
        const auto count=::send(fd,plan.packet,size_t(plan.packet_length),MSG_DONTWAIT|MSG_NOSIGNAL);
        if (count<0 && (errno==EAGAIN || errno==EWOULDBLOCK || errno==EINTR)) {
            const auto outcome=errno==EINTR?PR_APPEARANCE_INTERRUPTED:PR_APPEARANCE_WOULD_BLOCK;
            pr_appearance_client_report_v1(policy,&plan,outcome,0);
            return;
        }
        if (count!=ssize_t(plan.packet_length)) {
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_IO_FAILED,0);
            return;
        }
        pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_SENT,uint32_t(count));
        return;
    }
    case PR_APPEARANCE_RECEIVE: {
        char bytes[4096];
        alignas(cmsghdr) char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(16*sizeof(int))];
        iovec vector{bytes,sizeof(bytes)};
        msghdr message{};
        message.msg_iov=&vector;
        message.msg_iovlen=1;
        message.msg_control=control;
        message.msg_controllen=sizeof(control);
        const auto count=::recvmsg(fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);
        if (count<0 && (errno==EAGAIN || errno==EWOULDBLOCK || errno==EINTR)) {
            const auto outcome=errno==EINTR?PR_APPEARANCE_INTERRUPTED:PR_APPEARANCE_WOULD_BLOCK;
            pr_appearance_client_report_v1(policy,&plan,outcome,0);
            return;
        }
        bool trusted=false,seen=false;
        bool invalid=message.msg_flags&(MSG_TRUNC|MSG_CTRUNC);
        for (auto *c=CMSG_FIRSTHDR(&message);count>=0 && c;c=CMSG_NXTHDR(&message,c)) {
            if (c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_CREDENTIALS
                && c->cmsg_len==CMSG_LEN(sizeof(ucred)) && !seen) {
                seen=true;
                ucred credentials{};
                std::memcpy(&credentials,CMSG_DATA(c),sizeof(credentials));
                trusted=credentials.uid==0 && credentials.gid==0 && credentials.pid==peerPid;
            } else {
                invalid=true;
                if (c->cmsg_level==SOL_SOCKET && c->cmsg_type==SCM_RIGHTS && c->cmsg_len>=CMSG_LEN(0)) {
                    const auto handles=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);
                    auto *received=reinterpret_cast<int*>(CMSG_DATA(c));
                    for (size_t i=0;i<handles;++i) ::close(received[i]);
                }
            }
        }
        if (count<=0 || count>ssize_t(sizeof(bytes)) || !trusted || invalid) {
            pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_IO_FAILED,0);
            return;
        }
        pr_appearance_client_received_v1(policy,&plan,reinterpret_cast<const uint8_t*>(bytes),uint32_t(count));
        return;
    }
    case PR_APPEARANCE_CLOSE:
        closeSocket();
        pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_CLOSED,0);
        return;
    default:
        pr_appearance_client_report_v1(policy,&plan,PR_APPEARANCE_IO_FAILED,0);
        return;
    }
}
