#pragma once
#include <QObject>
#include <QVariantMap>
#include <QTimer>
#include "pokelinux_appearance_client.h"
class QSocketNotifier;

// Authenticated public appearance only. Never used for action/session authority.
class PhoneAppearanceClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap snapshot READ snapshot NOTIFY changed)
    Q_PROPERTY(bool available READ available NOTIFY changed)
public:
    explicit PhoneAppearanceClient(const QString &socketPath, QObject *parent = nullptr);
    ~PhoneAppearanceClient() override;
    QVariantMap snapshot() const { return current; }
    bool available() const;
signals:
    void changed();
private:
    void refresh();
    void tick();
    void notified(uint64_t generation, uint32_t event);
    void drain(bool notify = true);
    void execute(const PrAppearanceStepV1 &plan);
    void applyView(const PrAppearanceViewV1 &view, bool notify);
    void closeSocket();
    bool peer();
    QString path;
    QVariantMap current;
    PrAppearanceClient *policy = nullptr;
    uint64_t deliveredChangeSequence = 0;
    int fd = -1;
    qint64 peerPid = 0;
    QSocketNotifier *readNotifier = nullptr, *writeNotifier = nullptr;
    QTimer pollTimer, requestTimer, expiryTimer;
};
