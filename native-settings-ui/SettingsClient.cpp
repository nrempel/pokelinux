#include "SettingsClient.h"
#include <QFile>
#include <chrono>
#include <cstdlib>
#include <thread>
#include <cstddef>

static_assert(sizeof(PrSettingsView) == 48 && alignof(PrSettingsView) == 8);
static_assert(offsetof(PrSettingsView, observed_percent) == 40);
namespace {
PrSettingsView emptyView() {
    PrSettingsView view{};
    view.requested_percent = view.observed_percent = view.actual_percent = -1;
    return view;
}
bool same(const PrSettingsView &a, const PrSettingsView &b) {
    return a.revision == b.revision && a.until_ns == b.until_ns
        && a.connected == b.connected && a.available == b.available
        && a.pending == b.pending && a.can_set == b.can_set && a.status == b.status
        && a.requested_percent == b.requested_percent
        && a.observed_percent == b.observed_percent && a.actual_percent == b.actual_percent;
}
}
SettingsClient::SettingsClient(const QString &socket, QObject *parent)
    : QObject(parent), delivered(emptyView()) {
    const auto path = QFile::encodeName(socket);
    if (pr_settings_client_new_v1(reinterpret_cast<const uint8_t *>(path.constData()),
                                  size_t(path.size()), &core) != PR_SETTINGS_OK) return;
    timer.setInterval(100);
    connect(&timer, &QTimer::timeout, this, &SettingsClient::sync);
    timer.start();
    QTimer::singleShot(0, this, &SettingsClient::sync);
}
SettingsClient::~SettingsClient() {
    timer.stop();
    if (!core) return;
    if (pr_settings_client_stop_v1(core) != PR_SETTINGS_OK) std::abort();
    for (;;) {
        const auto result = pr_settings_client_join_v1(core);
        if (result == PR_SETTINGS_OK) break;
        if (result != PR_SETTINGS_WAIT) std::abort();
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    if (pr_settings_client_free_v1(core) != PR_SETTINGS_OK) std::abort();
}
PrSettingsView SettingsClient::readView() const {
    auto view = emptyView();
    if (core) pr_settings_client_view_v1(core, &view);
    return view;
}
bool SettingsClient::available() const { return readView().available != 0; }
bool SettingsClient::pending() const { return readView().pending != 0; }
bool SettingsClient::canSet() const { return readView().can_set != 0; }
QVariant SettingsClient::confirmedPercent() const {
    const auto view = readView();
    return view.available && view.observed_percent >= 0 ? QVariant(view.observed_percent) : QVariant();
}
QVariant SettingsClient::actualPercent() const {
    const auto view = readView();
    return view.available && view.actual_percent >= 0 ? QVariant(view.actual_percent) : QVariant();
}
QString SettingsClient::status() const {
    switch (readView().status) {
    case PR_SETTINGS_READY: return QStringLiteral("Brightness ready");
    case PR_SETTINGS_DENIED: return QStringLiteral("Brightness access denied");
    case PR_SETTINGS_PENDING: return QStringLiteral("Brightness change pending");
    case PR_SETTINGS_UNCERTAIN: return QStringLiteral("Brightness change unconfirmed");
    case PR_SETTINGS_STOPPED: return QStringLiteral("Brightness stopped");
    case PR_SETTINGS_CONTROLS_UNAVAILABLE: return QStringLiteral("Brightness controls unavailable");
    default: return QStringLiteral("Brightness unavailable");
    }
}
void SettingsClient::setPercent(uint32_t percent) {
    if (core) pr_settings_client_set_v1(core, percent);
    sync();
}
void SettingsClient::sync() {
    const auto view = readView();
    const bool updated = !same(delivered, view);
    delivered = view;
    // No object access after signal delivery: a receiver may destroy the adapter.
    if (updated) emit changed();
}
