// Private model driver around unchanged production MessagesClient. No GUI,
// synthetic reply, authentication API, clock injection or service IPC proxy.
#include "MessagesClient.h"
#include <QCoreApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSocketNotifier>
#include <cerrno>
#include <csignal>
#include <cstring>
#include <memory>
#include <optional>
#include <utility>
#include <sys/signalfd.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

namespace {
constexpr int MaxPacket = 4096;
class OwnedFd {
public:
    explicit OwnedFd(int value = -1) : value_(value) {}
    OwnedFd(const OwnedFd&) = delete;
    OwnedFd& operator=(const OwnedFd&) = delete;
    ~OwnedFd() { reset(); }
    int get() const { return value_; }
    void reset(int value = -1) {
        if (value_ >= 0) ::close(value_);
        value_ = value;
    }
private:
    int value_;
};

bool socketPath(const QByteArray& path) {
    sockaddr_un address{};
    return path.startsWith('/') && path.size() < int(sizeof(address.sun_path));
}

std::optional<QJsonObject> identity() {
    if (::getuid() != 11010 || ::geteuid() != 11010 ||
        ::getgid() != 11010 || ::getegid() != 11010 || ::getgroups(0, nullptr) != 0)
        return {};
    QFile status("/proc/self/status");
    if (!status.open(QIODevice::ReadOnly)) return {};
    const auto bytes = status.read(16385);
    if (bytes.size() > 16384) return {};
    QByteArray capabilities;
    for (const auto& line : bytes.split('\n')) {
        if (line.startsWith("CapEff:")) {
            if (!capabilities.isEmpty()) return {};
            capabilities = line.mid(7).trimmed();
        }
    }
    if (capabilities != "0000000000000000") return {};
    return QJsonObject{{"pid", qint64(::getpid())}, {"uid", qint64(::getuid())},
                       {"gid", qint64(::getgid())}, {"groups", QJsonArray{}},
                       {"cap_eff", QString::fromLatin1(capabilities)}};
}

class Driver final : public QObject {
public:
    Driver(QByteArray path, MessagesClient& client, QJsonObject actual)
        : path_(std::move(path)), client_(client), actual_(std::move(actual)) {}
    ~Driver() override {
        closePeer(false);
        listenerNotifier_.reset();
        listener_.reset();
        struct stat current{};
        if (bound_ && ::lstat(path_.constData(), &current) == 0 &&
            current.st_dev == device_ && current.st_ino == inode_)
            ::unlink(path_.constData());
    }
    bool start() {
        listener_.reset(::socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK | SOCK_CLOEXEC, 0));
        const int one = 1;
        if (listener_.get() < 0 ||
            ::setsockopt(listener_.get(), SOL_SOCKET, SO_PASSCRED, &one, sizeof(one)))
            return false;
        sockaddr_un address{};
        address.sun_family = AF_UNIX;
        std::memcpy(address.sun_path, path_.constData(), size_t(path_.size()) + 1);
        if (::bind(listener_.get(), reinterpret_cast<sockaddr*>(&address), sizeof(address)))
            return false;
        struct stat created{};
        if (::lstat(path_.constData(), &created) || !S_ISSOCK(created.st_mode)) return false;
        device_ = created.st_dev;
        inode_ = created.st_ino;
        bound_ = true;
        if (::chmod(path_.constData(), 0600) || ::listen(listener_.get(), 1)) return false;
        listenerNotifier_ = std::make_unique<QSocketNotifier>(
            listener_.get(), QSocketNotifier::Read, this);
        QObject::connect(listenerNotifier_.get(), &QSocketNotifier::activated,
                         this, [this] { acceptPeer(); });
        return true;
    }
private:
    void closePeer(bool acceptMore = true) {
        if (peerNotifier_) {
            peerNotifier_->setEnabled(false);
            peerNotifier_->deleteLater();
            peerNotifier_ = nullptr;
        }
        peer_.reset();
        if (acceptMore && listenerNotifier_) listenerNotifier_->setEnabled(true);
    }
    void acceptPeer() {
        const int accepted = ::accept4(listener_.get(), nullptr, nullptr,
                                       SOCK_NONBLOCK | SOCK_CLOEXEC);
        if (accepted < 0) return;
        listenerNotifier_->setEnabled(false);
        peer_.reset(accepted);
        socklen_t length = sizeof(peerIdentity_);
        const int one = 1;
        if (::getsockopt(accepted, SOL_SOCKET, SO_PEERCRED, &peerIdentity_, &length) ||
            length != sizeof(peerIdentity_) || peerIdentity_.uid != 0 ||
            peerIdentity_.gid != 0 || peerIdentity_.pid <= 0 ||
            ::setsockopt(accepted, SOL_SOCKET, SO_PASSCRED, &one, sizeof(one))) {
            closePeer();
            return;
        }
        peerNotifier_ = new QSocketNotifier(accepted, QSocketNotifier::Read, this);
        QObject::connect(peerNotifier_, &QSocketNotifier::activated,
                         this, [this] { readRequest(); });
    }
    void readRequest() {
        char bytes[MaxPacket];
        alignas(cmsghdr) char ancillary[CMSG_SPACE(sizeof(ucred)) + CMSG_SPACE(16 * sizeof(int))]{};
        iovec buffer{bytes, sizeof(bytes)};
        msghdr message{};
        message.msg_iov = &buffer;
        message.msg_iovlen = 1;
        message.msg_control = ancillary;
        message.msg_controllen = sizeof(ancillary);
        const auto count = ::recvmsg(peer_.get(), &message, MSG_DONTWAIT | MSG_CMSG_CLOEXEC);
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) return;
        if (count <= 0) { closePeer(); return; }
        bool valid = !(message.msg_flags & (MSG_TRUNC | MSG_CTRUNC));
        bool credentialSeen = false;
        for (auto* item = CMSG_FIRSTHDR(&message); item; item = CMSG_NXTHDR(&message, item)) {
            if (item->cmsg_level == SOL_SOCKET && item->cmsg_type == SCM_CREDENTIALS &&
                item->cmsg_len == CMSG_LEN(sizeof(ucred)) && !credentialSeen) {
                ucred sender{};
                std::memcpy(&sender, CMSG_DATA(item), sizeof(sender));
                credentialSeen = true;
                valid = valid && sender.pid == peerIdentity_.pid &&
                    sender.uid == peerIdentity_.uid && sender.gid == peerIdentity_.gid;
            } else {
                valid = false;
                if (item->cmsg_level == SOL_SOCKET && item->cmsg_type == SCM_RIGHTS &&
                    item->cmsg_len >= CMSG_LEN(0)) {
                    const auto length = item->cmsg_len - CMSG_LEN(0);
                    for (size_t offset = 0; offset + sizeof(int) <= length; offset += sizeof(int)) {
                        int received = -1;
                        std::memcpy(&received, CMSG_DATA(item) + offset, sizeof(received));
                        if (received >= 0) ::close(received);
                    }
                }
            }
        }
        if (!valid || !credentialSeen) { closePeer(); return; }
        QJsonParseError error{};
        const auto request = QJsonDocument::fromJson(QByteArray(bytes, int(count)), &error);
        if (error.error != QJsonParseError::NoError || !request.isObject()) {
            closePeer();
            return;
        }
        const auto object = request.object();
        if (object.size() != 1 || !object.value("op").isString()) { closePeer(); return; }
        QJsonObject reply = actual_;
        const auto operation = object.value("op").toString();
        if (operation == "model") {
            // Getter reads run on the same Qt event thread as queued apply().
            reply.insert("status", client_.status());
            reply.insert("conversations", QJsonArray::fromVariantList(client_.conversations()));
            reply.insert("messages", QJsonArray::fromVariantList(client_.messages()));
            reply.insert("client_kind", QStringLiteral("actual_production_MessagesClient"));
        } else if (operation != "identity") {
            closePeer();
            return;
        }
        const auto packet = QJsonDocument(reply).toJson(QJsonDocument::Compact);
        if (packet.isEmpty() || packet.size() > MaxPacket) { closePeer(); return; }
        const auto sent = ::send(peer_.get(), packet.constData(), size_t(packet.size()),
                                 MSG_DONTWAIT | MSG_NOSIGNAL);
        if (sent != packet.size()) { closePeer(); return; }
        // One bounded request/reply per accepted connection. A full/failed send
        // closes the peer, never blocks the Qt loop or returns a partial model.
        closePeer();
    }
    QByteArray path_;
    MessagesClient& client_;
    QJsonObject actual_;
    OwnedFd listener_;
    OwnedFd peer_;
    std::unique_ptr<QSocketNotifier> listenerNotifier_;
    QSocketNotifier* peerNotifier_ = nullptr;
    ucred peerIdentity_{};
    dev_t device_ = 0;
    ino_t inode_ = 0;
    bool bound_ = false;
};
} // namespace

int main(int argc, char** argv) {
    if (argc != 3) return 64;
    const QByteArray service(argv[1]), driverPath(argv[2]);
    if (!socketPath(service) || !socketPath(driverPath) || service == driverPath) return 64;
    const auto actual = identity();
    if (!actual) return 65;
    // Block before the production client's QThread exists, so SIGTERM reaches
    // only this owned descriptor and causes normal Qt/client destruction.
    sigset_t termination;
    if (::sigemptyset(&termination) || ::sigaddset(&termination, SIGTERM) ||
        ::sigprocmask(SIG_BLOCK, &termination, nullptr)) return 70;
    OwnedFd terminationFd(::signalfd(-1, &termination, SFD_NONBLOCK | SFD_CLOEXEC));
    if (terminationFd.get() < 0) return 70;
    QCoreApplication application(argc, argv);
    QSocketNotifier terminate(terminationFd.get(), QSocketNotifier::Read);
    QObject::connect(&terminate, &QSocketNotifier::activated, &application, [&] {
        signalfd_siginfo notification{};
        const auto count = ::read(terminationFd.get(), &notification, sizeof(notification));
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) return;
        if (count == ssize_t(sizeof(notification)) && notification.ssi_signo == SIGTERM)
            application.quit();
        else
            application.exit(70);
    });
    int result;
    {
        ::umask(0077);
        MessagesClient client(QString::fromLocal8Bit(service));
        Driver driver(driverPath, client, *actual);
        if (!driver.start()) return 70;
        result = application.exec();
    } // Normal app quit destroys driver, then joins the unchanged client owner.
    return result;
}
