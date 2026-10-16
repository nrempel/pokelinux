#pragma once
#include <QObject>
#include <QVariantMap>
#include <QTimer>
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
    enum class Stage { Idle, Connecting, Sending, Receiving };
    void refresh();
    void writable();
    void readable();
    void fail();
    void closeSocket();
    void invalidate();
    void armExpiry();
    bool peer();
    QString path;
    QString owner;
    quint64 revision = 0, requestUntil = 0, paletteUntil = 0, connectionGeneration = 0;
    QVariantMap current;
    bool valid = false;
    int fd = -1;
    qint64 peerPid = 0;
    Stage stage = Stage::Idle;
    QSocketNotifier *readNotifier = nullptr, *writeNotifier = nullptr;
    QTimer pollTimer, requestTimer, expiryTimer;
};
