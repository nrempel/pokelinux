#pragma once
#include <QObject>
#include <QTimer>
#include <QVariant>
#include "pokelinux_settings_connectivity.h"
// Rust owns transport, permission, freshness, intent deadlines and the worker.
class SettingsConnectivityClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(bool pending READ pending NOTIFY changed)
    Q_PROPERTY(bool canControl READ canControl NOTIFY changed)
    Q_PROPERTY(QVariant configured READ configured NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
public:
    explicit SettingsConnectivityClient(const QString &socket, QObject *parent = nullptr);
    ~SettingsConnectivityClient() override;
    bool available() const;
    bool pending() const;
    bool canControl() const;
    QVariant configured() const;
    QString status() const;
    Q_INVOKABLE void setConnected(bool connected);
    void beginClose();
signals:
    void changed();
    void closed();
private:
    PrSettingsConnectivityView readView() const;
    void sync();
    void finishClose();
    PrSettingsConnectivityClient *core = nullptr;
    PrSettingsConnectivityView delivered{};
    QTimer refresh;
    QTimer joinTimer;
    bool closing = false;
    uint64_t closeUntil = 0;
};
