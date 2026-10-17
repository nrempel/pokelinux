#pragma once
#include <QObject>
#include <QTimer>
#include <QVariant>
#include "pokelinux_settings_client.h"
// Rust owns transport, validation, state, deadlines and the owned worker.
class SettingsClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(bool pending READ pending NOTIFY changed)
    Q_PROPERTY(bool canSet READ canSet NOTIFY changed)
    Q_PROPERTY(QVariant confirmedPercent READ confirmedPercent NOTIFY changed)
    Q_PROPERTY(QVariant actualPercent READ actualPercent NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
public:
    explicit SettingsClient(const QString &socket, QObject *parent = nullptr);
    ~SettingsClient() override;
    bool available() const;
    bool pending() const;
    bool canSet() const;
    QVariant confirmedPercent() const;
    QVariant actualPercent() const;
    QString status() const;
    Q_INVOKABLE void setPercent(uint32_t percent);
signals:
    void changed();
private:
    PrSettingsView readView() const;
    void sync();
    PrSettingsClient *core = nullptr;
    PrSettingsView delivered{};
    QTimer timer;
};
