#include "PhoneBatteryClient.h"

namespace {
PrBatteryViewV1 emptyView() {
    PrBatteryViewV1 view{};
    view.capacity_percent = -1;
    return view;
}
}

PhoneBatteryClient::PhoneBatteryClient(QObject *parent) : QObject(parent), delivered(emptyView()) {
    if (pr_battery_client_new_v1(&client) != PR_BATTERY_OK || !client) return;
    timer.setInterval(100);
    connect(&timer, &QTimer::timeout, this, &PhoneBatteryClient::tick);
    timer.start();
    QTimer::singleShot(0, this, &PhoneBatteryClient::tick);
}

PhoneBatteryClient::~PhoneBatteryClient() {
    timer.stop();
    pr_battery_client_free_v1(client);
}

PrBatteryViewV1 PhoneBatteryClient::readView() const {
    auto view = emptyView();
    if (client) pr_battery_client_view_v1(client, &view);
    return view;
}

bool PhoneBatteryClient::available() const { return readView().available != 0; }
QVariant PhoneBatteryClient::capacityPercent() const {
    const auto view = readView();
    return view.available ? QVariant(view.capacity_percent) : QVariant();
}
QString PhoneBatteryClient::status() const {
    switch (readView().status) {
    case PR_BATTERY_UNKNOWN: return QStringLiteral("unknown");
    case PR_BATTERY_CHARGING: return QStringLiteral("charging");
    case PR_BATTERY_DISCHARGING: return QStringLiteral("discharging");
    case PR_BATTERY_NOT_CHARGING: return QStringLiteral("not_charging");
    case PR_BATTERY_FULL: return QStringLiteral("full");
    default: return {};
    }
}

void PhoneBatteryClient::tick() {
    if (!client) return;
    pr_battery_client_tick_v1(client);
    const auto view = readView();
    const bool changedView = delivered.available != view.available
        || delivered.capacity_percent != view.capacity_percent || delivered.status != view.status;
    delivered = view;
    // Copied state is visible before a slot can reenter or destroy this object.
    if (changedView) emit changed();
}
