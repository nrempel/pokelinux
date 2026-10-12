#include "PhonePower.h"
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusVariant>
#include <utility>

namespace {
const QString service = QStringLiteral("org.freedesktop.login1");
const QString path = QStringLiteral("/org/freedesktop/login1");
const QString interface = QStringLiteral("org.freedesktop.login1.Manager");
}
PhonePower::PhonePower(QDBusConnection connection, QObject *parent)
    : QObject(parent), bus(std::move(connection)),
      owner(service, bus, QDBusServiceWatcher::WatchForOwnerChange, this) {
    connect(&owner, &QDBusServiceWatcher::serviceOwnerChanged, this,
        [this](const QString &, const QString &, const QString &) { invalidate(); refresh(); });
    const bool subscribed = bus.connect(service, path, interface, QStringLiteral("PrepareForSleep"),
        QStringLiteral("b"), this, SLOT(prepareForSleep(bool)));
    connectionCheck.setInterval(500);
    connect(&connectionCheck, &QTimer::timeout, this, [this] {
        if (!bus.isConnected() && known) invalidate();
    });
    connectionCheck.start();
    if (subscribed) refresh();
}
void PhonePower::invalidate() {
    ++revision;
    known = false;
    sleeping = true;
    emit changed();
}
void PhonePower::refresh() {
    const auto requested = ++revision;
    auto message = QDBusMessage::createMethodCall(service, path,
        QStringLiteral("org.freedesktop.DBus.Properties"), QStringLiteral("Get"));
    message << interface << QStringLiteral("PreparingForSleep");
    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(message, 1500), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, requested](QDBusPendingCallWatcher *done) {
        const QDBusPendingReply<QDBusVariant> reply = *done;
        done->deleteLater();
        if (requested != revision) return; // A newer signal or owner supersedes this read.
        if (reply.isError() || reply.value().variant().metaType().id() != QMetaType::Bool) {
            invalidate(); return;
        }
        known = true;
        sleeping = reply.value().variant().toBool();
        emit changed();
    });
}
void PhonePower::prepareForSleep(bool preparing) {
    ++revision;
    known = true;
    sleeping = preparing;
    emit changed();
}
