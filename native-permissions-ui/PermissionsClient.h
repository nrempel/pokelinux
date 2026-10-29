#pragma once
#include <QObject>
#include <QTimer>
#include "pokelinux_permissions_client.h"
// Presentation adapter. Rust owns credentials, requests, deadlines and confirmed state.
class PermissionsClient final : public QObject {
 Q_OBJECT
 Q_PROPERTY(int allowed READ allowed NOTIFY changed)
 Q_PROPERTY(bool canSet READ canSet NOTIFY changed)
 Q_PROPERTY(bool pending READ pending NOTIFY changed)
 Q_PROPERTY(bool unconfirmed READ unconfirmed NOTIFY changed)
public:
 explicit PermissionsClient(const QString &socket, QObject *parent=nullptr);
 ~PermissionsClient() override;
 int allowed() const;
 bool canSet() const;
 bool pending() const;
 bool unconfirmed() const;
 Q_INVOKABLE void choose(bool allowed);
 void setActive(bool active);
 void close();
signals:
 void changed();
private:
 PrPermissionsView read() const;
 void sync();
 PrPermissionsHandle *core=nullptr;
 PrPermissionsView delivered{-1,0,0,0};
 QTimer timer;
};
