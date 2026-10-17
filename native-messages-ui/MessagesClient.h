#pragma once
#include <QObject>
#include <QVariantList>
#include <QString>
#include "pokelinux_messages_client.h"
class QThread;
// Rust owns the request/model engine. Qt owns presentation and the native edge.
class MessagesClient final: public QObject {
 Q_OBJECT
 Q_PROPERTY(QVariantList conversations READ conversations NOTIFY changed)
 Q_PROPERTY(QVariantList messages READ messages NOTIFY changed)
 Q_PROPERTY(QString status READ status NOTIFY changed)
public:
 explicit MessagesClient(QString socket,QObject *parent=nullptr);
 ~MessagesClient() override;
 QVariantList conversations()const{return convo;}
 QVariantList messages()const{return texts;}
 QString status()const{return currentStatus;}
 Q_INVOKABLE void select(const QString &id);
 Q_INVOKABLE void nextPage();
 Q_INVOKABLE void firstPage();
 Q_INVOKABLE void nextConversations();
 Q_INVOKABLE void firstConversations();
signals: void changed();
private:
 void loop(const QString &socket);
 void action(uint32_t kind,const PrMessagesIdV1 *id=nullptr);
 void sync();
 PrMessagesClient *core=nullptr;
 QThread *thread=nullptr;
 uint64_t projectedRevision=0;
 QVariantList convo,texts;
 QString currentStatus="Messages unavailable";
};
