#pragma once
#include <QObject>
#include <QLocalSocket>
#include <QTimer>
#include <QVariantList>
#include <QJsonObject>
#include <sys/types.h>

// One authenticated connection owns one media lease. The event loop owns all IO.
class AudioClient final : public QObject {
 Q_OBJECT
 Q_PROPERTY(QVariantList routes READ routes NOTIFY changed)
 Q_PROPERTY(QString requested READ requested NOTIFY changed)
 Q_PROPERTY(QString applied READ applied NOTIFY changed)
 Q_PROPERTY(QString status READ status NOTIFY changed)
 Q_PROPERTY(bool canChoose READ canChoose NOTIFY changed)
 Q_PROPERTY(bool canReconnect READ canReconnect NOTIFY changed)
public:
 AudioClient(QString path, uid_t ownerUid, QObject *parent=nullptr);
 ~AudioClient() override;
 QVariantList routes() const { return m_routes; }
 QString requested() const { return m_requested; }
 QString applied() const { return m_applied; }
 QString status() const { return m_status; }
 bool canChoose() const { return m_online && m_available && m_pending.isEmpty(); }
 bool canReconnect() const { return !m_online && m_socket.state()==QLocalSocket::UnconnectedState; }
 Q_INVOKABLE void reconnect();
 Q_INVOKABLE void select(const QString &id);
signals:
 void changed();
private:
 static qint64 boottime();
 void fail(const QString &message);
 void request(const QJsonObject &message, const QString &kind);
 void receive();
 bool accept(const QJsonObject &reply);
 bool expired() const;
 QString m_path;
 uid_t m_ownerUid;
 QLocalSocket m_socket;
 QTimer m_poll, m_watch;
 QByteArray m_input;
 QString m_pending, m_requested, m_applied;
 QString m_status=QStringLiteral("Audio disconnected");
 QVariantList m_routes;
 qint64 m_deadline=0, m_lease=-1, m_intent=-1, m_epoch=-1, m_revision=-1;
 bool m_online=false, m_available=false, m_closing=false;
};
