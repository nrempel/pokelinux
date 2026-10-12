#pragma once
#include <QObject>
#include <QProcess>
#include <QTimer>

// Process adapter only. Rust owns protocol validation and command dispatch.
class PhoneRpc final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
public:
    explicit PhoneRpc(QString executable = QStringLiteral("/usr/bin/omarchy-phone"), QObject *parent = nullptr);
    ~PhoneRpc() override;
    bool busy() const { return active; }
    Q_INVOKABLE bool request(const QString &socket, const QString &frame, const QString &token);
    Q_INVOKABLE QString newId() const;
    Q_INVOKABLE double boottimeMs() const;
signals:
    void busyChanged();
    // Rust CLI status: 0 validated reply, 2 not sent, 3 unknown, 4 service error.
    void completed(int status, QByteArray reply, QString token);
private:
    void readOutput();
    void finish(int status);
    QString executable;
    QString callerToken;
    QProcess process;
    QTimer deadline;
    QByteArray input;
    QByteArray output;
    bool active = false;
    bool failed = false;
};
