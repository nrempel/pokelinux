#pragma once
#include <QObject>
#include <QTimer>
#include <QVariantList>
#include <sys/types.h>
#include "pokelinux_sounds_client.h"
class QSocketNotifier;
// Qt owns native transport/credentials and presentation. Rust owns the local
// request/packet/deadline/page/model engine; the lifecycle actor owns authority.
class AudioClient final : public QObject {
 Q_OBJECT
 Q_PROPERTY(QVariantList routes READ routes NOTIFY changed)
 Q_PROPERTY(QString requested READ requested NOTIFY changed)
 Q_PROPERTY(QString applied READ applied NOTIFY changed)
 Q_PROPERTY(QString status READ status NOTIFY changed)
 Q_PROPERTY(bool canChoose READ canChoose NOTIFY changed)
 Q_PROPERTY(bool canReconnect READ canReconnect NOTIFY changed)
 Q_PROPERTY(bool canPlay READ canPlay NOTIFY changed)
 Q_PROPERTY(bool canStop READ canStop NOTIFY changed)
 Q_PROPERTY(int previewLevelPercent READ previewLevelPercent NOTIFY changed)
 Q_PROPERTY(bool previewMuted READ previewMuted NOTIFY changed)
 Q_PROPERTY(bool canAdjustPreview READ canAdjustPreview NOTIFY changed)
 Q_PROPERTY(bool canTogglePreviewMute READ canTogglePreviewMute NOTIFY changed)
 Q_PROPERTY(bool previewMuteChecked READ previewMuteChecked NOTIFY changed)
 Q_PROPERTY(bool previewMuteLive READ previewMuteLive NOTIFY changed)
public:
 AudioClient(QString path, uid_t ownerUid, QObject *parent=nullptr);
 ~AudioClient() override;
 QVariantList routes() const;
 QString requested() const;
 QString applied() const;
 QString status() const;
 bool canChoose() const;
 bool canReconnect() const;
 bool canPlay() const;
 bool canStop() const;
 int previewLevelPercent() const;
 bool previewMuted() const;
 bool canAdjustPreview() const;
 bool canTogglePreviewMute() const;
 bool previewMuteChecked() const;
 bool previewMuteLive() const;
 Q_INVOKABLE void togglePreviewMute();
 Q_INVOKABLE void setPreviewLevel(int percent, bool muted);
 Q_INVOKABLE void reconnect();
 Q_INVOKABLE void playPreview();
 Q_INVOKABLE void stopPreview();
 Q_INVOKABLE void select(const QString &id);
signals:
 void changed();
private:
 PrSoundsClientViewV1 view() const;
 void closeNative();
 void nativeFailure(uint32_t reason);
 void connectNative();
 bool peer();
 void writable();
 void receive();
 void drive();
 void sync();
 QString m_path;
 uid_t m_ownerUid;
 int m_fd=-1;
 pid_t m_peerPid=-1;
 uint64_t m_nativeEpoch=0;
 QSocketNotifier *m_read=nullptr,*m_write=nullptr;
 QTimer m_poll,m_watch;
 PrSoundsClient *m_client=nullptr;
};
