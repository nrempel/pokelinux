#pragma once
#include <QObject>
#include <QTimer>
#include <QVariant>
#include "pokelinux_network_status_client.h"

// Public coarse counts only. Rust owns the socket, validation and expiry.
class PhoneNetworkStatusClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(QVariant localInterfaces READ localInterfaces NOTIFY changed)
    Q_PROPERTY(QVariant addressRecords READ addressRecords NOTIFY changed)
public:
    explicit PhoneNetworkStatusClient(QObject *parent = nullptr);
    ~PhoneNetworkStatusClient() override;
    bool available() const;
    QVariant localInterfaces() const;
    QVariant addressRecords() const;
signals:
    void changed();
private:
    PrNetworkStatusViewV1 readView() const;
    void tick();
    PrNetworkStatusClient *client = nullptr;
    PrNetworkStatusViewV1 delivered{};
    QTimer timer;
};
