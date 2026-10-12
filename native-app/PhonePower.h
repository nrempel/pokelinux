#pragma once
#include <QObject>
#include <QDBusConnection>
#include <QDBusServiceWatcher>
#include <QTimer>

// Read-only host lifecycle observer. It never requests or inhibits sleep.
class PhonePower final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(bool preparing READ preparing NOTIFY changed)
public:
    explicit PhonePower(QDBusConnection connection = QDBusConnection::systemBus(), QObject *parent = nullptr);
    bool available() const { return known; }
    bool preparing() const { return sleeping; }
signals:
    void changed();
private slots:
    void prepareForSleep(bool preparing);
private:
    void refresh();
    void invalidate();
    QDBusConnection bus;
    QDBusServiceWatcher owner;
    QTimer connectionCheck;
    quint64 revision = 0;
    bool known = false;
    bool sleeping = true;
};
