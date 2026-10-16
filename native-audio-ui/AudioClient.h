#pragma once
#include <QObject>
#include <QJsonObject>
#include <QTimer>
#include <QVariantList>
#include <sys/types.h>
class QSocketNotifier;

// Presentation of one root lifecycle actor connection. The actor owns policy
// and its audio-provider connection; Qt never owns or replays a media lease.
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
 bool canChoose() const { return m_online && m_available && m_actorCanChoose && !m_reconnectQueued && m_queued.isEmpty() && (m_pending.isEmpty() || m_pending=="status"); }
 bool canReconnect() const { return !m_reconnectQueued && m_queued.isEmpty() && (m_pending.isEmpty() || m_pending=="status") && (m_fd<0 || (m_online && m_actorCanReconnect)); }
 Q_INVOKABLE void reconnect();
 Q_INVOKABLE void select(const QString &id);
signals:
 void changed();
private:
 enum class Stage { Idle, Connecting, Sending, Receiving };
 enum class Reply { Invalid, Done, More };
 static qint64 boottime();
 bool expired() const;
 bool setDeadline();
 void closeSocket();
 void fail(const QString &message);
 void connectActor();
 bool peer();
 void request(const QJsonObject &message, const QString &kind, qint64 originalUntil=0);
 void sendSelection(const QString &id, qint64 originalUntil, bool wasQueued);
 void dispatchQueued();
 bool queue(const QJsonObject &message);
 void statusRequest();
 void writable();
 void receive();
 Reply accept(const QJsonObject &reply);
 void applyState(const QJsonObject &state);
 QString m_path;
 uid_t m_ownerUid;
 int m_fd=-1;
 pid_t m_peerPid=-1;
 QSocketNotifier *m_read=nullptr,*m_write=nullptr;
 quint64 m_generation=0;
 Stage m_stage=Stage::Idle;
 QTimer m_poll,m_watch;
 QByteArray m_output;
 QString m_pending,m_requested,m_applied,m_queued;
 qint64 m_queuedUntil=0;
 QString m_status=QStringLiteral("Audio disconnected");
 QVariantList m_routes,m_pageRoutes;
 qint64 m_deadline=0;
 int m_offset=0;
 bool m_online=false,m_available=false,m_actorCanChoose=false,m_actorCanReconnect=false,m_closing=false,m_reconnectQueued=false;
};
