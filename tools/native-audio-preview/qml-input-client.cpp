// Private source-only consumer. No normal main/window, actor policy or PCM substitute.
#include "AudioClient.h"
#include "PhoneAppearanceClient.h"
#include <QDebug>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QList>
#include <QMetaMethod>
#include <QMouseEvent>
#include <QPointingDevice>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickView>
#include <QSignalSpy>
#include <QRectF>
#include <QSocketNotifier>
#include <QTimer>
#include <atomic>
#include <cerrno>
#include <cmath>
#include <cstddef>
#include <cstring>
#include <functional>
#include <memory>
#include <stdexcept>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

namespace {
constexpr unsigned RecordLimit = 128, PacketLimit = 4096, PageLimit = 4;
constexpr unsigned ControlRequestLimit = 256, ControlReplyLimit = 65536;
struct SendRecord {
    std::atomic_bool ready{false};
    int fd = -1, controllerFd = -1, flags = 0, error = 0;
    unsigned playChoice = 0;
    ssize_t result = -1;
    size_t length = 0, copied = 0;
    ucred peer{};
    sockaddr_un address{};
    socklen_t addressLength = 0;
    bool peerValid = false, addressValid = false, clocksValid = false;
    timespec before{}, after{};
    unsigned char bytes[PacketLimit]{};
};
SendRecord records[RecordLimit];
std::atomic_uint recordCount{0};
std::atomic_bool recordOverflow{false};
std::atomic_int controllerFd{-1};
std::atomic_uint currentPlayChoice{0};
void require(bool condition, const char *reason) {
    if (!condition) throw std::runtime_error(reason);
}
quint64 ns(const timespec &v) {
    return quint64(v.tv_sec) * 1000000000ULL + quint64(v.tv_nsec);
}
quint64 now() {
    timespec value{};
    require(::clock_gettime(CLOCK_BOOTTIME, &value) == 0, "CLOCK_BOOTTIME unavailable");
    return ns(value);
}
QString decimal(quint64 value) { return QString::number(value); }
bool allHidden(const QQuickView &view) {
    if (view.isVisible() || view.isExposed()) return false;
    for (auto *window : QGuiApplication::topLevelWindows()) {
        if (qobject_cast<QQuickWindow *>(window) && (window->isVisible() || window->isExposed())) return false;
    }
    return true;
}
QByteArray peerPath(const SendRecord &record) {
    const auto offset = offsetof(sockaddr_un, sun_path);
    if (!record.addressValid || record.address.sun_family != AF_UNIX || record.addressLength <= offset
        || record.address.sun_path[0] == '\0') return {};
    const size_t room = size_t(record.addressLength) - offset;
    return QByteArray(record.address.sun_path, int(strnlen(record.address.sun_path,
        room < sizeof(record.address.sun_path) ? room : sizeof(record.address.sun_path))));
}
}

extern "C" ssize_t __real_send(int, const void *, size_t, int);
extern "C" ssize_t __wrap_send(int fd, const void *bytes, size_t length, int flags) {
    // Only fixed storage, telemetry syscalls and memcpy. No Qt/parse/callback/retry.
    const int entryError = errno;
    ucred peer{}; socklen_t peerLength = sizeof(peer);
    const bool peerValid = ::getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &peer, &peerLength) == 0
        && peerLength == sizeof(peer);
    errno = entryError;
    sockaddr_un address{}; socklen_t addressLength = sizeof(address);
    const bool addressValid = ::getpeername(fd, reinterpret_cast<sockaddr *>(&address), &addressLength) == 0;
    errno = entryError;
    const unsigned choiceAtSend = currentPlayChoice.load(std::memory_order_relaxed);
    timespec before{}, after{};
    const bool beforeValid = ::clock_gettime(CLOCK_BOOTTIME, &before) == 0;
    errno = entryError;
    const ssize_t result = __real_send(fd, bytes, length, flags); // Exactly one unchanged forward.
    const int actualError = errno;
    const bool afterValid = ::clock_gettime(CLOCK_BOOTTIME, &after) == 0;
    errno = actualError;
    const unsigned index = recordCount.fetch_add(1, std::memory_order_relaxed);
    if (index < RecordLimit) {
        auto &record = records[index];
        record.fd = fd; record.controllerFd = controllerFd.load(std::memory_order_relaxed);
        record.flags = flags; record.error = actualError; record.result = result; record.length = length;
        record.playChoice = choiceAtSend;
        record.peerValid = peerValid; record.addressValid = addressValid;
        record.addressLength = addressLength; record.clocksValid = beforeValid && afterValid;
        std::memcpy(&record.peer, &peer, sizeof(peer));
        std::memcpy(&record.address, &address, sizeof(address));
        std::memcpy(&record.before, &before, sizeof(before));
        std::memcpy(&record.after, &after, sizeof(after));
        // The real production C ABI supplies bounded valid storage. EFAULT is not dereferenced.
        if (length <= PacketLimit && (length == 0 || bytes != nullptr) && !(result < 0 && actualError == EFAULT)) {
            if (length) std::memcpy(record.bytes, bytes, length);
            record.copied = length;
        }
        record.ready.store(true, std::memory_order_release);
    } else recordOverflow.store(true, std::memory_order_release);
    errno = actualError;
    return result;
}

class Component final : public QObject {
    QByteArray appPath, controlPath;
    std::unique_ptr<PhoneAppearanceClient> appearance;
    std::unique_ptr<AudioClient> audio;
    std::unique_ptr<QQuickView> view;
    std::unique_ptr<QObject> theme;
    QQuickItem *statusItem = nullptr, *levelItem = nullptr, *level50Item = nullptr, *level100Item = nullptr, *muteItem = nullptr;
    std::unique_ptr<QSocketNotifier> acceptNotice, readNotice, writeNotice;
    QTimer controlWatch;
    int listener = -1, control = -1;
    ucred controller{};
    quint64 controlUntil = 0;
    QByteArray reply;
    QString diagnosticOperation;
    QJsonObject observedClick;
    bool controlTrusted = false;
    bool routeClicked = false, stopClicked = false, levelClicked = false, muteClicked = false, initialized = false, qmlWarning = false;
    unsigned playClicks = 0;
    dev_t socketDevice{}; ino_t socketInode{};
    static void retireNotice(std::unique_ptr<QSocketNotifier> &notice) {
        if (notice) { notice->setEnabled(false); notice.release()->deleteLater(); }
    }
    void sendFailureDiagnostic(const char *reason) noexcept {
        // Best effort only: same authenticated private request, same original
        // deadline, one nonblocking sendmsg, then unchanged terminal exit91.
        try {
            timespec observed{};
            if (control < 0 || !controlTrusted || ::clock_gettime(CLOCK_BOOTTIME, &observed) != 0
                || ns(observed) >= controlUntil) return;
            const QJsonObject body{{"diagnostic", QStringLiteral("QML_INPUT_COMPONENT_SETUP_FAILURE")},
                {"operation", diagnosticOperation}, {"reason", QString::fromLocal8Bit(reason).left(256)},
                {"observed", observedClick}, {"original_control_until_boottime_ns", decimal(controlUntil)},
                {"observed_boottime_ns", decimal(ns(observed))}};
            const QByteArray bytes = QJsonDocument(body).toJson(QJsonDocument::Compact);
            if (bytes.size() > 4096 || ::clock_gettime(CLOCK_BOOTTIME, &observed) != 0
                || ns(observed) >= controlUntil) return;
            iovec part{const_cast<char *>(bytes.constData()), size_t(bytes.size())};
            msghdr message{}; message.msg_iov = &part; message.msg_iovlen = 1;
            (void)::sendmsg(control, &message, MSG_DONTWAIT | MSG_NOSIGNAL);
        } catch (...) {} // Diagnostic failure never retries an event or effect.
    }
    void safe(const std::function<void()> &work) {
        try { require(allHidden(*view), "visible/exposed top-level Quick window"); work(); }
        catch (const std::exception &error) {
            sendFailureDiagnostic(error.what());
            qCritical("QML_INPUT_COMPONENT_SETUP_FAILURE: %s", error.what());
            if (acceptNotice) acceptNotice->setEnabled(false);
            if (readNotice) readNotice->setEnabled(false);
            if (writeNotice) writeNotice->setEnabled(false);
            controlWatch.stop(); QCoreApplication::exit(91);
        }
    }
    QJsonObject state() const {
        require(!qmlWarning, "QML warning fenced component state");
        require(allHidden(*view), "state observed a visible/exposed Quick window");
        require(statusItem && levelItem && level50Item && level100Item && muteItem, "real named QML status/level controls absent");
        return {{"routes", QJsonArray::fromVariantList(audio->routes())},
            {"requested", audio->requested()}, {"applied", audio->applied()}, {"status", audio->status()},
            {"canChoose", audio->canChoose()}, {"canPlay", audio->canPlay()}, {"canStop", audio->canStop()},
            {"qml_status", statusItem->property("text").toString()},
            {"previewLevelPercent", audio->previewLevelPercent()}, {"previewMuted", audio->previewMuted()},
            {"canAdjustPreview", audio->canAdjustPreview()}, {"qml_level", levelItem->property("text").toString()},
            {"qml_muted", muteItem->property("checked").toBool()},
            {"qml_level50_enabled", level50Item->isEnabled()}, {"qml_level100_enabled", level100Item->isEnabled()},
            {"qml_mute_enabled", muteItem->isEnabled()}};
    }
    QJsonObject telemetry(unsigned after) const {
        const unsigned count = recordCount.load(std::memory_order_acquire);
        require(!recordOverflow.load(std::memory_order_acquire) && count <= RecordLimit, "send telemetry overflow");
        require(after <= count, "telemetry marker exceeds actual records");
        struct Choice { quint64 until = 0, sentUntil = 0; qint64 previousTtl = 1001; unsigned attempts = 0, complete = 0; };
        Choice choices[2]; QJsonArray page; unsigned completePlays = 0;
        for (unsigned i = 0; i < count; ++i) {
            const auto &record = records[i];
            require(record.ready.load(std::memory_order_acquire), "send telemetry is still in flight");
            const bool actor = record.peerValid && record.peer.pid > 0 && record.peer.uid == 0 && record.peer.gid == 0
                && record.fd != record.controllerFd && peerPath(record) == appPath;
            const bool complete = record.result >= 0 && size_t(record.result) == record.length;
            QJsonObject wire;
            if (actor) {
                require(record.clocksValid && record.copied == record.length, "actual actor send lacks exact bytes/clock");
                QJsonParseError error{};
                const auto document = QJsonDocument::fromJson(QByteArray(reinterpret_cast<const char *>(record.bytes), int(record.copied)), &error);
                require(error.error == QJsonParseError::NoError && document.isObject(), "actual actor packet is not JSON object");
                wire = document.object();
                if (wire.value("op").toString() == QStringLiteral("media_preview_play")) {
                    const auto request = wire.value("request").toObject();
                    const auto untilValue = request.value("until_boottime_ns"), ttlValue = request.value("ttl_ms");
                    const auto levelValue = request.value("level_percent"), mutedValue = request.value("muted");
                    require(wire.size() == 2 && request.size() == 5 && request.value("route").toString() == QStringLiteral("media-a7")
                        && levelValue.isDouble() && levelValue.toInteger(-1) == 50 && levelValue.toDouble() == 50.0
                        && mutedValue.isBool(), "mandatory actual Play level fields refused");
                    require(untilValue.isDouble() && ttlValue.isDouble(), "actual Play integer fields absent");
                    const qint64 until = untilValue.toInteger(-1), ttl = ttlValue.toInteger(-1);
                    require(record.playChoice >= 1 && record.playChoice <= 2, "actual Play has no explicit choice ordinal");
                    require(mutedValue.toBool() == (record.playChoice == 2), "actual explicit Play level/mute choice changed");
                    auto &choice = choices[record.playChoice - 1]; ++choice.attempts;
                    require(until > 0 && ttl > 0 && ttl <= 1000 && ttl <= choice.previousTtl, "same-choice Play TTL invalid/increased");
                    require(quint64(until) > ns(record.before) && quint64(until) - ns(record.before) <= 1000000000ULL,
                        "actual original Play cutoff exceeds one second");
                    require(choice.until == 0 || choice.until == quint64(until), "same-choice Play absolute deadline changed");
                    choice.until = quint64(until); choice.previousTtl = ttl;
                    if (complete) { ++choice.complete; ++completePlays; choice.sentUntil = quint64(until); }
                    require(choice.complete <= 1, "more than one fully sent Play for one explicit choice");
                }
            }
            if (i >= after && i - after < PageLimit) {
                QJsonObject row{{"marker", int(i + 1)}, {"play_choice", int(record.playChoice)}, {"fd", record.fd}, {"controller_fd_at_send", record.controllerFd},
                    {"actor_peer", actor}, {"peer_valid", record.peerValid}, {"peer_pid", record.peer.pid},
                    {"peer_uid", int(record.peer.uid)}, {"peer_gid", int(record.peer.gid)},
                    {"length", decimal(record.length)}, {"result", QString::number(qlonglong(record.result))},
                    {"flags", record.flags}, {"errno_after_real_send", record.error}, {"complete_send", complete},
                    {"before_boottime_ns", decimal(ns(record.before))}, {"after_boottime_ns", decimal(ns(record.after))},
                    {"clocks_valid", record.clocksValid}, {"copied_bytes", int(record.copied)},
                    {"bytes_base64", QString::fromLatin1(QByteArray(reinterpret_cast<const char *>(record.bytes), int(record.copied)).toBase64())}};
                if (actor) row.insert("literal_packet", wire);
                page.append(row);
            }
        }
        QJsonArray choiceViews;
        for (unsigned i = 0; i < 2; ++i) {
            QJsonObject choice{{"ordinal", int(i + 1)}, {"attempts", int(choices[i].attempts)}, {"complete_sends", int(choices[i].complete)}};
            if (choices[i].complete) choice.insert("original_until_boottime_ns", decimal(choices[i].sentUntil));
            choiceViews.append(choice);
        }
        const unsigned next = after + (count - after < PageLimit ? count - after : PageLimit);
        QJsonObject result{{"after", int(after)}, {"next", int(next)}, {"available_through", int(count)},
            {"more", next < count}, {"overflow", false}, {"records", page}, {"complete_play_sends", int(completePlays)},
            {"play_choices", choiceViews}};
        return result;
    }
    QJsonObject click(const QString &operation) {
        if (operation == QStringLiteral("click_play")) {
            require(playClicks < 2, "only two separate explicit Play choices are admitted");
            currentPlayChoice.store(++playClicks, std::memory_order_release);
        } else {
            bool &used = operation == QStringLiteral("click_route") ? routeClicked
                : operation == QStringLiteral("click_level50") ? levelClicked
                : operation == QStringLiteral("click_mute") ? muteClicked : stopClicked;
            require(!used, "route/level/mute/Stop click cannot be retried"); used = true;
        }
        const QString name = operation == QStringLiteral("click_route") ? QStringLiteral("route_media-a7")
            : operation == QStringLiteral("click_play") ? QStringLiteral("previewPlay")
            : operation == QStringLiteral("click_level50") ? QStringLiteral("previewLevel50")
            : operation == QStringLiteral("click_mute") ? QStringLiteral("previewMute") : QStringLiteral("previewStop");
        // Repeater delegates belong to the visual tree, independently of QObject parenting.
        QQuickItem *button = nullptr;
        unsigned matches = 0;
        QList<QQuickItem *> visualItems{view->rootObject()};
        QJsonArray namedVisualItems;
        for (qsizetype index = 0; index < visualItems.size(); ++index) {
            auto *item = visualItems.at(index);
            const QString objectName = item->objectName();
            if (!objectName.isEmpty() && namedVisualItems.size() < 16) namedVisualItems.append(objectName.left(64));
            if (objectName == name) { button = item; ++matches; }
            visualItems.append(item->childItems());
        }
        observedClick = {{"stage", QStringLiteral("button_preconditions")}, {"button", name}, {"found", button != nullptr},
            {"visual_exact_matches", int(matches)}, {"visual_item_count", int(visualItems.size())}, {"named_visual_items", namedVisualItems}};
        require(matches == 1, "real visual Button name absent or not unique");
        const auto layoutGeometry = [this, &visualItems] {
            QJsonArray result;
            const auto append = [&result](QQuickItem *item, const QString &role) {
                const QPointF origin = item->mapToScene(QPointF(0, 0));
                result.append(QJsonObject{{"role", role}, {"x", item->x()}, {"y", item->y()},
                    {"width", item->width()}, {"height", item->height()}, {"scene_x", origin.x()}, {"scene_y", origin.y()}});
            };
            append(view->contentItem(), QStringLiteral("window_content"));
            append(view->rootObject(), QStringLiteral("root"));
            for (auto *item : visualItems) {
                const QString name = item->objectName();
                if (result.size() < 6 && (name.startsWith(QStringLiteral("route_"))
                    || name == QStringLiteral("previewPlay") || name == QStringLiteral("previewStop"))) append(item, name.left(64));
            }
            return result;
        };
        observedClick.insert("layout_before_polish", layoutGeometry());
        // One public layout pass, children before parents; never show or retry input.
        for (qsizetype index = visualItems.size(); index > 0; --index) visualItems.at(index - 1)->ensurePolished();
        view->contentItem()->ensurePolished();
        observedClick.insert("layout_after_polish", layoutGeometry());
        require(allHidden(*view), "layout polish made a Quick window visible/exposed");
        if (button) {
            observedClick.insert("inherits_QQuickButton", button->inherits("QQuickButton"));
            observedClick.insert("enabled", button->isEnabled()); observedClick.insert("visible", button->isVisible());
            observedClick.insert("width", button->width()); observedClick.insert("height", button->height());
        }
        require(button && button->inherits("QQuickButton") && button->isEnabled() && button->isVisible(), "real enabled Button absent");
        observedClick.insert("stage", QStringLiteral("button_geometry"));
        require(button->width() > 0 && button->height() > 0, "real Button has empty geometry");
        const QPointF center = button->mapToScene(QPointF(button->width() / 2, button->height() / 2));
        observedClick.insert("stage", QStringLiteral("button_center"));
        if (std::isfinite(center.x())) observedClick.insert("scene_x", center.x());
        if (std::isfinite(center.y())) observedClick.insert("scene_y", center.y());
        observedClick.insert("view_width", view->width()); observedClick.insert("view_height", view->height());
        require(std::isfinite(center.x()) && std::isfinite(center.y()) && center.x() >= 0 && center.y() >= 0
            && center.x() < view->width() && center.y() < view->height(), "real Button center outside component");
        for (auto *parent = button->parentItem(); parent; parent = parent->parentItem()) {
            if (parent->clip()) {
                const bool inside = QRectF(0, 0, parent->width(), parent->height()).contains(parent->mapFromScene(center));
                observedClick.insert("stage", QStringLiteral("button_clip")); observedClick.insert("inside_current_clip", inside);
                require(inside, "real Button center clipped");
            }
        }
        observedClick.insert("stage", QStringLiteral("clicked_observer"));
        const int signalIndex = button->metaObject()->indexOfSignal("clicked()");
        observedClick.insert("clicked_signal_index", signalIndex);
        require(signalIndex >= 0, "real Button lacks clicked signal");
        QSignalSpy clicked(button, button->metaObject()->method(signalIndex));
        require(clicked.isValid(), "clicked observer invalid");
        observedClick.insert("stage", QStringLiteral("mouse_pair_dispatch"));
        const quint64 before = now();
        QMouseEvent press(QEvent::MouseButtonPress, center, center, center,
            Qt::LeftButton, Qt::LeftButton, Qt::NoModifier, QPointingDevice::primaryPointingDevice());
        press.setTimestamp(before / 1000000ULL);
        const bool pressDelivered = QCoreApplication::sendEvent(view.get(), &press);
        const bool hiddenAfterPress = allHidden(*view);
        QMouseEvent release(QEvent::MouseButtonRelease, center, center, center,
            Qt::LeftButton, Qt::NoButton, Qt::NoModifier, QPointingDevice::primaryPointingDevice());
        release.setTimestamp(now() / 1000000ULL);
        const bool releaseDelivered = QCoreApplication::sendEvent(view.get(), &release);
        const quint64 after = now();
        observedClick.insert("stage", QStringLiteral("mouse_pair_observed"));
        observedClick.insert("press_delivered", pressDelivered); observedClick.insert("release_delivered", releaseDelivered);
        observedClick.insert("press_accepted", press.isAccepted()); observedClick.insert("release_accepted", release.isAccepted());
        observedClick.insert("clicked_signals", clicked.count()); observedClick.insert("hidden_after_press", hiddenAfterPress);
        observedClick.insert("before_boottime_ns", decimal(before)); observedClick.insert("after_boottime_ns", decimal(after));
        require(hiddenAfterPress && allHidden(*view), "mouse delivery made a Quick window visible/exposed");
        require(pressDelivered && releaseDelivered && clicked.count() == 1, "actual mouse pair did not click the real Button once");
        return {{"button", name}, {"play_choice", int(currentPlayChoice.load(std::memory_order_acquire))}, {"scene_x", center.x()}, {"scene_y", center.y()},
            {"press_delivered", pressDelivered}, {"release_delivered", releaseDelivered},
            {"press_accepted", press.isAccepted()}, {"release_accepted", release.isAccepted()},
            {"clicked_signals", clicked.count()}, {"all_quick_windows_hidden", true},
            {"before_boottime_ns", decimal(before)}, {"after_boottime_ns", decimal(after)}};
    }
    void accepted() {
        require(control < 0, "overlapping private control request");
        control = ::accept4(listener, nullptr, nullptr, SOCK_CLOEXEC | SOCK_NONBLOCK);
        if (control < 0 && (errno == EAGAIN || errno == EINTR)) return;
        require(control >= 0, "control accept failed");
        controllerFd.store(control, std::memory_order_release);
        controlTrusted = false; diagnosticOperation.clear(); observedClick = {};
        socklen_t length = sizeof(controller); int enabled = 1;
        require(::getsockopt(control, SOL_SOCKET, SO_PEERCRED, &controller, &length) == 0
            && length == sizeof(controller) && controller.pid > 0 && controller.uid == 0 && controller.gid == 0,
            "private controller peer is not root");
        require(::setsockopt(control, SOL_SOCKET, SO_PASSCRED, &enabled, sizeof(enabled)) == 0, "control SO_PASSCRED failed");
        controlUntil = now() + 2000000000ULL; acceptNotice->setEnabled(false);
        readNotice = std::make_unique<QSocketNotifier>(control, QSocketNotifier::Read, this);
        QObject::connect(readNotice.get(), &QSocketNotifier::activated, this, [this] { safe([this] { received(); }); });
        controlWatch.start();
    }
    void received() {
        require(now() < controlUntil, "original control receive deadline expired");
        char bytes[ControlRequestLimit];
        alignas(cmsghdr) char ancillary[CMSG_SPACE(sizeof(ucred)) + CMSG_SPACE(16 * sizeof(int))]{};
        iovec part{bytes, sizeof(bytes)}; msghdr message{};
        message.msg_iov = &part; message.msg_iovlen = 1; message.msg_control = ancillary; message.msg_controllen = sizeof(ancillary);
        const ssize_t count = ::recvmsg(control, &message, MSG_DONTWAIT | MSG_CMSG_CLOEXEC);
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) return;
        bool trusted = false, seen = false, invalid = message.msg_flags & (MSG_TRUNC | MSG_CTRUNC);
        for (auto *header = CMSG_FIRSTHDR(&message); count >= 0 && header; header = CMSG_NXTHDR(&message, header)) {
            if (header->cmsg_level == SOL_SOCKET && header->cmsg_type == SCM_CREDENTIALS
                && header->cmsg_len == CMSG_LEN(sizeof(ucred)) && !seen) {
                ucred peer{}; std::memcpy(&peer, CMSG_DATA(header), sizeof(peer)); seen = true;
                trusted = peer.pid == controller.pid && peer.uid == controller.uid && peer.gid == controller.gid;
            } else {
                invalid = true;
                if (header->cmsg_level == SOL_SOCKET && header->cmsg_type == SCM_RIGHTS && header->cmsg_len >= CMSG_LEN(0)) {
                    const size_t handles = (header->cmsg_len - CMSG_LEN(0)) / sizeof(int);
                    const auto *fds = reinterpret_cast<const int *>(CMSG_DATA(header));
                    for (size_t i = 0; i < handles; ++i) ::close(fds[i]);
                }
            }
        }
        require(count > 0 && trusted && !invalid, "private control packet credentials/framing refused");
        controlTrusted = true;
        QJsonParseError error{}; const auto document = QJsonDocument::fromJson(QByteArray(bytes, int(count)), &error);
        require(error.error == QJsonParseError::NoError && document.isObject(), "control request is not JSON object");
        const auto request = document.object(); const QString operation = request.value("op").toString();
        const auto marker = request.value("after"); const qint64 after = marker.toInteger(-1);
        require(request.size() == 2 && marker.isDouble() && after >= 0 && after <= RecordLimit
            && marker.toDouble() == double(after), "control request/marker refused");
        require(operation == QStringLiteral("state") || operation == QStringLiteral("click_route")
            || operation == QStringLiteral("click_play") || operation == QStringLiteral("click_stop")
            || operation == QStringLiteral("click_level50") || operation == QStringLiteral("click_mute"), "unknown private operation");
        require(now() < controlUntil, "parsed control request exceeded its original deadline");
        diagnosticOperation = operation;
        QJsonObject response{{"operation", operation}};
        if (operation != QStringLiteral("state")) response.insert("dispatch", click(operation));
        response.insert("state", state()); response.insert("telemetry", telemetry(unsigned(after)));
        require(now() < controlUntil, "original control response deadline expired");
        reply = QJsonDocument(response).toJson(QJsonDocument::Compact);
        require(reply.size() <= ControlReplyLimit, "bounded private response exceeded");
        readNotice->setEnabled(false);
        writeNotice = std::make_unique<QSocketNotifier>(control, QSocketNotifier::Write, this);
        QObject::connect(writeNotice.get(), &QSocketNotifier::activated, this, [this] { safe([this] { sent(); }); });
        sent();
    }
    void sent() {
        require(now() < controlUntil, "original control send deadline expired");
        iovec part{reply.data(), size_t(reply.size())}; msghdr message{}; message.msg_iov = &part; message.msg_iovlen = 1;
        // Separate private edge, intentionally not the wrapped production ::send.
        const auto count = ::sendmsg(control, &message, MSG_DONTWAIT | MSG_NOSIGNAL);
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) return;
        require(count == reply.size(), "private control response not fully sent");
        controlWatch.stop(); retireNotice(readNotice); retireNotice(writeNotice);
        ::close(control); control = -1; controllerFd.store(-1, std::memory_order_release); controlTrusted = false;
        reply.clear(); acceptNotice->setEnabled(true);
    }
public:
    Component(const QByteArray &app, const QByteArray &path, const QString &appearanceDirectory)
        : appPath(app), controlPath(path) {
        require(getuid() == 11042 && geteuid() == 11042 && getgid() == 11042 && getegid() == 11042, "actor-launched UID/GID11042 required");
        require(!app.isEmpty() && app[0] == '/' && app.size() < int(sizeof(sockaddr_un::sun_path)), "app path refused");
        require(!path.isEmpty() && path[0] == '/' && path.size() < int(sizeof(sockaddr_un::sun_path)), "control path refused");
        struct stat metadata{}; const auto qml = (appearanceDirectory + QStringLiteral("/PhoneAppearance.qml")).toLocal8Bit();
        require(::lstat(qml.constData(), &metadata) == 0 && S_ISREG(metadata.st_mode) && metadata.st_uid == 0
            && (metadata.st_mode & 0777) == 0644, "protected shared appearance QML refused");
        const auto directory = appearanceDirectory.toLocal8Bit();
        require(::lstat(directory.constData(), &metadata) == 0 && S_ISDIR(metadata.st_mode) && metadata.st_uid == 0
            && !(metadata.st_mode & 0022), "protected appearance directory refused");
        const QByteArray missing = path + ".appearance-unavailable";
        require(missing.size() < int(sizeof(sockaddr_un::sun_path)) && ::lstat(missing.constData(), &metadata) < 0
            && errno == ENOENT, "explicit unavailable appearance endpoint not absent");
        appearance = std::make_unique<PhoneAppearanceClient>(QString::fromLocal8Bit(missing));
        audio = std::make_unique<AudioClient>(QString::fromLocal8Bit(app), uid_t(0));
        view = std::make_unique<QQuickView>();
        view->setResizeMode(QQuickView::SizeRootObjectToView); view->resize(600, 1400); // Arbitrary component layout only.
        QObject::connect(view->engine(), &QQmlEngine::warnings, this, [this](const QList<QQmlError> &) {
            qmlWarning = true;
            if (initialized) safe([] { require(false, "production QML emitted a warning"); });
        });
        QQmlComponent appearanceComponent(view->engine(), QUrl::fromLocalFile(QString::fromLocal8Bit(qml)));
        theme.reset(appearanceComponent.createWithInitialProperties({{"source", QVariant::fromValue(appearance.get())}}));
        require(theme != nullptr, "real shared appearance QML did not construct");
        view->setInitialProperties({{"theme", QVariant::fromValue(theme.get())}, {"audioClient", QVariant::fromValue(audio.get())}});
        view->setSource(QUrl(QStringLiteral("qrc:/sounds/SoundsView.qml")));
        require(view->status() == QQuickView::Ready && view->rootObject() && allHidden(*view) && !qmlWarning, "real unshown SoundsView did not construct");
        for (auto *item : view->rootObject()->findChildren<QQuickItem *>()) {
            if (item->inherits("QQuickText") && item->property("text").toString() == audio->status()) {
                require(statusItem == nullptr, "ambiguous actual QML status Text"); statusItem = item;
            }
        }
        for (auto *item : view->rootObject()->findChildren<QQuickItem *>()) {
            QQuickItem **slot = item->objectName() == QStringLiteral("previewLevelValue") ? &levelItem
                : item->objectName() == QStringLiteral("previewLevel50") ? &level50Item
                : item->objectName() == QStringLiteral("previewLevel100") ? &level100Item
                : item->objectName() == QStringLiteral("previewMute") ? &muteItem : nullptr;
            if (slot) { require(*slot == nullptr, "ambiguous named real QML level control"); *slot = item; }
        }
        require(statusItem && levelItem && level50Item && level100Item && muteItem, "actual QML status/level controls not found");
        require(levelItem->inherits("QQuickText") && level50Item->inherits("QQuickButton")
            && level100Item->inherits("QQuickButton") && muteItem->inherits("QQuickButton"), "actual QML level control types refused");
        require(::lstat(path.constData(), &metadata) < 0 && errno == ENOENT, "control path already exists");
        listener = ::socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
        require(listener >= 0, "private listener socket failed");
        int enabled = 1; require(::setsockopt(listener, SOL_SOCKET, SO_PASSCRED, &enabled, sizeof(enabled)) == 0, "listener credentials failed");
        sockaddr_un address{}; address.sun_family = AF_UNIX; std::memcpy(address.sun_path, path.constData(), size_t(path.size()) + 1);
        ::umask(0077);
        require(::bind(listener, reinterpret_cast<sockaddr *>(&address), sizeof(address)) == 0
            && ::chmod(path.constData(), 0600) == 0 && ::listen(listener, 1) == 0, "private listener bind/listen failed");
        require(::lstat(path.constData(), &metadata) == 0, "created listener identity unavailable");
        socketDevice = metadata.st_dev; socketInode = metadata.st_ino;
        acceptNotice = std::make_unique<QSocketNotifier>(listener, QSocketNotifier::Read, this);
        QObject::connect(acceptNotice.get(), &QSocketNotifier::activated, this, [this] { safe([this] { accepted(); }); });
        controlWatch.setInterval(10);
        QObject::connect(&controlWatch, &QTimer::timeout, this, [this] { safe([this] { require(now() < controlUntil, "original private control deadline expired"); }); });
        initialized = true;
    }
    ~Component() override {
        controlWatch.stop(); acceptNotice.reset(); readNotice.reset(); writeNotice.reset();
        if (control >= 0) ::close(control);
        if (listener >= 0) ::close(listener);
        struct stat metadata{};
        if (::lstat(controlPath.constData(), &metadata) == 0 && metadata.st_dev == socketDevice && metadata.st_ino == socketInode) ::unlink(controlPath.constData());
        view.reset(); theme.reset(); audio.reset(); appearance.reset();
    }
};

int main(int argc, char **argv) {
    if (argc != 4) return 90;
    const QByteArray appPath(argv[1]), controlPath(argv[2]); const QString appearanceDirectory = QString::fromLocal8Bit(argv[3]);
    QQuickStyle::setStyle(QStringLiteral("Basic"));
    qputenv("QT_QPA_PLATFORM", "offscreen");
    qputenv("QT_QUICK_BACKEND", "software");
    QGuiApplication application(argc, argv); application.setQuitOnLastWindowClosed(false);
    try { Component component(appPath, controlPath, appearanceDirectory); return application.exec(); }
    catch (const std::exception &error) { qCritical("QML_INPUT_COMPONENT_SETUP_FAILURE: %s", error.what()); return 91; }
}
