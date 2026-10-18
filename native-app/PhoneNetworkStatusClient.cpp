#include "PhoneNetworkStatusClient.h"

namespace {
PrNetworkStatusViewV1 emptyView() {
    PrNetworkStatusViewV1 view{};
    view.local_interfaces = -1;
    view.address_records = -1;
    return view;
}
}

PhoneNetworkStatusClient::PhoneNetworkStatusClient(QObject *parent) : QObject(parent), delivered(emptyView()) {
    if (pr_network_status_client_new_v1(&client) != PR_NETWORK_STATUS_OK || !client) return;
    timer.setInterval(100);
    connect(&timer, &QTimer::timeout, this, &PhoneNetworkStatusClient::tick);
    timer.start();
    QTimer::singleShot(0, this, &PhoneNetworkStatusClient::tick);
}

PhoneNetworkStatusClient::~PhoneNetworkStatusClient() {
    timer.stop();
    pr_network_status_client_free_v1(client);
}

PrNetworkStatusViewV1 PhoneNetworkStatusClient::readView() const {
    auto view = emptyView();
    if (client) pr_network_status_client_view_v1(client, &view);
    return view;
}

bool PhoneNetworkStatusClient::available() const { return readView().available != 0; }
QVariant PhoneNetworkStatusClient::localInterfaces() const {
    const auto view = readView();
    return view.available ? QVariant(view.local_interfaces) : QVariant();
}
QVariant PhoneNetworkStatusClient::addressRecords() const {
    const auto view = readView();
    return view.available ? QVariant(view.address_records) : QVariant();
}

void PhoneNetworkStatusClient::tick() {
    if (!client) return;
    pr_network_status_client_tick_v1(client);
    const auto view = readView();
    const bool changedView = delivered.available != view.available
        || delivered.local_interfaces != view.local_interfaces || delivered.address_records != view.address_records;
    delivered = view;
    if (changedView) emit changed();
}
