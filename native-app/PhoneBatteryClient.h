#pragma once
#include <QObject>
#include <QString>
#include <QTimer>
#include <QVariant>
#include "pokelinux_battery_client.h"

// Public coarse battery state only; Rust owns transport, validation and expiry.
class PhoneBatteryClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(QVariant capacityPercent READ capacityPercent NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
public:
    explicit PhoneBatteryClient(QObject *parent = nullptr);
    ~PhoneBatteryClient() override;
    bool available() const;
    QVariant capacityPercent() const;
    QString status() const;
signals:
    void changed();
private:
    PrBatteryViewV1 readView() const;
    void tick();
    PrBatteryClient *client = nullptr;
    PrBatteryViewV1 delivered{};
    QTimer timer;
};
