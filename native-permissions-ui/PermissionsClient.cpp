#include "PermissionsClient.h"
#include <QFile>
#include <cstddef>
#include <cstdlib>
static_assert(sizeof(PrPermissionsView)==16 && alignof(PrPermissionsView)==4);
static_assert(offsetof(PrPermissionsView,unconfirmed)==12);
PermissionsClient::PermissionsClient(const QString &socket,QObject *parent):QObject(parent){
 const auto path=QFile::encodeName(socket);
 if(pr_permissions_new_v1(reinterpret_cast<const uint8_t*>(path.constData()),size_t(path.size()),&core)!=PR_PERMISSIONS_OK)return;
 timer.setInterval(20);
 connect(&timer,&QTimer::timeout,this,&PermissionsClient::sync);
 timer.start();
}
PermissionsClient::~PermissionsClient(){close();}
void PermissionsClient::close(){
 timer.stop();
 if(core){if(pr_permissions_free_v1(core)!=PR_PERMISSIONS_OK)std::abort();core=nullptr;}
 sync();
}
PrPermissionsView PermissionsClient::read() const{
 PrPermissionsView v{-1,0,0,0};
 if(core && pr_permissions_view_v1(core,&v)!=PR_PERMISSIONS_OK)std::abort();
 return v;
}
int PermissionsClient::allowed() const{return read().allowed;}
bool PermissionsClient::canSet() const{return read().can_set!=0;}
bool PermissionsClient::pending() const{return read().pending!=0;}
bool PermissionsClient::unconfirmed() const{return read().unconfirmed!=0;}
void PermissionsClient::setActive(bool active){
 if(core && pr_permissions_active_v1(core,active?1:0)!=PR_PERMISSIONS_OK)std::abort();
 sync();
}
void PermissionsClient::choose(bool allowed){
 if(core)pr_permissions_choose_v1(core,allowed?1:0);
 sync();
}
void PermissionsClient::sync(){
 if(core && pr_permissions_tick_v1(core)!=PR_PERMISSIONS_OK)std::abort();
 const auto v=read();
 if(v.allowed!=delivered.allowed || v.can_set!=delivered.can_set || v.pending!=delivered.pending || v.unconfirmed!=delivered.unconfirmed){delivered=v;emit changed();}
}
