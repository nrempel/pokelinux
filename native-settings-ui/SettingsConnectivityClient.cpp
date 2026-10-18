#include "SettingsConnectivityClient.h"
#include <QFile>
#include <cstddef>
#include <cstdlib>
#include <limits>
#include <time.h>

static_assert(sizeof(PrSettingsConnectivityView) == 40 && alignof(PrSettingsConnectivityView) == 8);
static_assert(offsetof(PrSettingsConnectivityView, configured) == 36);
namespace {
PrSettingsConnectivityView emptyView() {
    PrSettingsConnectivityView view{};
    view.configured = -1;
    return view;
}
bool same(const PrSettingsConnectivityView &a, const PrSettingsConnectivityView &b) {
    return a.observed_at_ns == b.observed_at_ns && a.until_ns == b.until_ns
        && a.transport_connected == b.transport_connected && a.available == b.available
        && a.pending == b.pending && a.can_control == b.can_control && a.status == b.status
        && a.configured == b.configured;
}
uint64_t boottime() {
    timespec value{};
    if (::clock_gettime(CLOCK_BOOTTIME, &value) != 0 || value.tv_sec < 0) std::abort();
    return uint64_t(value.tv_sec) * 1000000000ULL + uint64_t(value.tv_nsec);
}
}
SettingsConnectivityClient::SettingsConnectivityClient(const QString &socket, QObject *parent)
    : QObject(parent), delivered(emptyView()) {
    joinTimer.setInterval(1);
    joinTimer.setTimerType(Qt::PreciseTimer);
    connect(&joinTimer, &QTimer::timeout, this, &SettingsConnectivityClient::finishClose);
    const auto path = QFile::encodeName(socket);
    if (pr_settings_connectivity_new_v1(reinterpret_cast<const uint8_t *>(path.constData()),
                                        size_t(path.size()), &core) != PR_SETTINGS_OK) return;
    refresh.setInterval(100);
    connect(&refresh, &QTimer::timeout, this, &SettingsConnectivityClient::sync);
    refresh.start();
    QTimer::singleShot(0, this, &SettingsConnectivityClient::sync);
}
SettingsConnectivityClient::~SettingsConnectivityClient() {
    refresh.stop();
    joinTimer.stop();
    // A retained Rust worker must never detach during QObject destruction.
    if (core) std::abort();
}
PrSettingsConnectivityView SettingsConnectivityClient::readView() const {
    auto view = emptyView();
    if (core) pr_settings_connectivity_view_v1(core, &view);
    else if (closing) view.status = PR_SETTINGS_STOPPED;
    return view;
}
bool SettingsConnectivityClient::available() const { return readView().available != 0; }
bool SettingsConnectivityClient::pending() const { return readView().pending != 0; }
bool SettingsConnectivityClient::canControl() const { return !closing && readView().can_control != 0; }
QVariant SettingsConnectivityClient::configured() const {
    const auto view = readView();
    return view.available && view.configured >= 0 ? QVariant(view.configured != 0) : QVariant();
}
QString SettingsConnectivityClient::status() const {
    switch (readView().status) {
    case PR_SETTINGS_READY: return QStringLiteral("Connection ready");
    case PR_SETTINGS_DENIED: return QStringLiteral("Connection access denied");
    case PR_SETTINGS_PENDING: return QStringLiteral("Connection change pending");
    case PR_SETTINGS_UNCERTAIN: return QStringLiteral("Connection change unconfirmed");
    case PR_SETTINGS_NOT_SENT: return QStringLiteral("Connection choice not sent");
    case PR_SETTINGS_STOPPED: return QStringLiteral("Connection client stopped");
    case PR_SETTINGS_CONTROLS_UNAVAILABLE: return QStringLiteral("Connection controls unavailable");
    default: return QStringLiteral("Connection unavailable");
    }
}
void SettingsConnectivityClient::setConnected(bool connected) {
    // Capture at this click's origin, before any deferred work or transport.
    const uint64_t originalUntil = pr_settings_connectivity_deadline_v1();
    if (closing || !core || originalUntil == 0) return;
    // Rust owns admission and confirmation; never retry or renew this click.
    pr_settings_connectivity_set_v1(core, connected ? 1U : 0U, originalUntil);
    sync();
}
void SettingsConnectivityClient::sync() {
    if (closing) return;
    const auto view = readView();
    const bool updated = !same(delivered, view);
    delivered = view;
    if (updated) emit changed(); // No object access after signal delivery.
}
void SettingsConnectivityClient::beginClose() {
    if (closing) return;
    closing = true;
    refresh.stop();
    if (!core) {
        QTimer::singleShot(0, this, [this] { emit closed(); });
        return;
    }
    const uint64_t started = boottime();
    if (started > std::numeric_limits<uint64_t>::max() - 200000000ULL) std::abort();
    closeUntil = started + 200000000ULL;
    if (pr_settings_connectivity_stop_v1(core) != PR_SETTINGS_OK) std::abort();
    joinTimer.start();
    QTimer::singleShot(0, this, &SettingsConnectivityClient::finishClose);
    emit changed(); // All state/timer setup precedes possible object destruction.
}
void SettingsConnectivityClient::finishClose() {
    if (!core) return;
    if (!closing || boottime() >= closeUntil) std::abort();
    const auto result = pr_settings_connectivity_join_v1(core);
    if (result == PR_SETTINGS_WAIT) return;
    if (result != PR_SETTINGS_OK || boottime() >= closeUntil) std::abort();
    if (pr_settings_connectivity_free_v1(core) != PR_SETTINGS_OK) std::abort();
    core = nullptr;
    joinTimer.stop();
    if (boottime() >= closeUntil) std::abort();
    emit closed(); // Last operation; the owner may now leave the event loop.
}
