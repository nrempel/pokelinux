#include "AudioClient.h"
#include <QSocketNotifier>
#include <cerrno>
#include <cstring>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
namespace {
QString routeText(const PrSoundsClientIdV1 &id){return QString::fromLatin1(reinterpret_cast<const char*>(id.bytes),int(id.length));}
constexpr const char *statuses[]={
 "Audio disconnected","Connecting to audio","Audio unavailable; reconnect to inspect","Checking audio","Applying output","Output applied","Media interrupted","Choose an output","Media route unavailable","Output requested","Media route denied","Audio unavailable","Audio command state unavailable","Audio actor path or clock refused","Audio actor connection refused","Audio actor connection expired","Audio actor unavailable; reconnect to inspect","Audio actor identity refused","Audio request expired; reconnect to inspect","Audio request expired","Audio clock unavailable","Audio request refused","Audio queued command refused","Audio request could not be sent","Audio reply outside its request","Audio reply expired","Audio actor packet refused","Audio reply refused","Output selected; preview idle","Preview starting","Preview playing","Preview retiring","Preview completed","Preview failed","Preview cancelled; reconnect to inspect","Updating preview mute"
};
}
PrSoundsClientViewV1 AudioClient::view() const {
 PrSoundsClientViewV1 v{};
 if(pr_sounds_client_view_v1(m_client,&v)!=PR_SOUNDS_CLIENT_OK)v.status=PR_SOUNDS_CLIENT_STATE_UNAVAILABLE;
 return v;
}
QVariantList AudioClient::routes() const {
 const auto v=view();QVariantList result;
 for(uint32_t i=0;i<v.route_count;++i){const auto &row=v.routes[i];result.append(QVariantMap{{"id",routeText(row.id)},{"label",QString::fromUtf8(reinterpret_cast<const char*>(row.label.bytes),int(row.label.length))}});}
 return result;
}
QString AudioClient::requested() const {const auto v=view();return v.requested_present?routeText(v.requested):QString{};}
QString AudioClient::applied() const {const auto v=view();return v.applied_present?routeText(v.applied):QString{};}
QString AudioClient::status() const {const auto code=view().status;return QString::fromLatin1(statuses[code<sizeof(statuses)/sizeof(*statuses)?code:uint32_t(PR_SOUNDS_CLIENT_STATE_UNAVAILABLE)]);}
bool AudioClient::canChoose() const {return view().can_choose!=0;}
bool AudioClient::canReconnect() const {return view().can_reconnect!=0;}
bool AudioClient::canPlay() const {return view().can_play!=0;}
bool AudioClient::canStop() const {return view().can_stop!=0;}
int AudioClient::previewLevelPercent() const {return int(view().selected_level_percent);}
bool AudioClient::previewMuted() const {return view().selected_muted!=0;}
bool AudioClient::canAdjustPreview() const {return view().can_adjust_preview!=0;}
bool AudioClient::canTogglePreviewMute() const {return view().can_toggle_mute!=0;}
bool AudioClient::previewMuteChecked() const {return view().mute_checked!=0;}
bool AudioClient::previewMuteLive() const {return view().mute_is_live!=0;}
void AudioClient::togglePreviewMute(){pr_sounds_client_preview_toggle_mute_v1(m_client);drive();}
void AudioClient::setPreviewLevel(int percent,bool muted){pr_sounds_client_preview_level_v1(m_client,uint32_t(percent),uint32_t(muted));sync();}
AudioClient::AudioClient(QString path,uid_t ownerUid,QObject *parent):QObject(parent),m_path(std::move(path)),m_ownerUid(ownerUid) {
 if(pr_sounds_client_new_v1(&m_client)!=PR_SOUNDS_CLIENT_OK)return;
 m_poll.setInterval(250);m_watch.setInterval(25);
 connect(&m_poll,&QTimer::timeout,this,[this]{if(view().online&&view().request_kind==PR_SOUNDS_CLIENT_NO_REQUEST){pr_sounds_client_refresh_v1(m_client);drive();}});
 connect(&m_watch,&QTimer::timeout,this,[this]{if(pr_sounds_client_tick_v1(m_client)!=PR_SOUNDS_CLIENT_OK)sync();});
 QTimer::singleShot(0,this,[this]{pr_sounds_client_start_v1(m_client);drive();});
}
AudioClient::~AudioClient(){m_poll.stop();m_watch.stop();closeNative();pr_sounds_client_free_v1(m_client);}
void AudioClient::closeNative() {
 for(auto **notifier:{&m_read,&m_write}){if(*notifier){(*notifier)->setEnabled(false);(*notifier)->deleteLater();*notifier=nullptr;}}
 if(m_fd>=0)::close(m_fd);
 m_fd=-1;m_peerPid=-1;m_nativeEpoch=0;
}
void AudioClient::nativeFailure(uint32_t reason){pr_sounds_client_disconnected_v1(m_client,view().connection_epoch,reason);sync();}
void AudioClient::sync() {
 const auto v=view();
 if(v.stage==PR_SOUNDS_CLIENT_CLOSED)closeNative();
 if(v.online){if(!m_poll.isActive())m_poll.start();}else m_poll.stop();
 if(v.request_kind!=PR_SOUNDS_CLIENT_NO_REQUEST){if(!m_watch.isActive())m_watch.start();}else m_watch.stop();
 if(m_read)m_read->setEnabled(v.online!=0);
 if(m_write)m_write->setEnabled(v.stage==PR_SOUNDS_CLIENT_CONNECTING||v.stage==PR_SOUNDS_CLIENT_SENDING);
 emit changed();
}
bool AudioClient::peer() {
 ucred who{};socklen_t size=sizeof(who);
 if(::getsockopt(m_fd,SOL_SOCKET,SO_PEERCRED,&who,&size)||size!=sizeof(who)||who.uid!=m_ownerUid||who.gid!=0||who.pid<=0)return false;
 m_peerPid=who.pid;return true;
}
void AudioClient::connectNative() {
 pr_sounds_client_tick_v1(m_client);
 const auto v=view();if(v.stage==PR_SOUNDS_CLIENT_CLOSED){sync();return;}
 if(m_fd>=0||v.stage!=PR_SOUNDS_CLIENT_CONNECTING)return;
 const auto path=m_path.toLocal8Bit();sockaddr_un address{};address.sun_family=AF_UNIX;
 if(!m_path.startsWith('/')||path.isEmpty()||path.contains('\0')||path.size()>=int(sizeof(address.sun_path))){nativeFailure(PR_SOUNDS_CLIENT_PATH_OR_CLOCK_REFUSED);return;}
 std::memcpy(address.sun_path,path.constData(),size_t(path.size())+1);
 m_nativeEpoch=v.connection_epoch;m_fd=::socket(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK|SOCK_CLOEXEC,0);int enabled=1;
 if(m_fd<0||::setsockopt(m_fd,SOL_SOCKET,SO_PASSCRED,&enabled,sizeof(enabled))){nativeFailure(PR_SOUNDS_CLIENT_CONNECTION_REFUSED);return;}
 const auto epoch=m_nativeEpoch;const auto fd=m_fd;
 m_read=new QSocketNotifier(m_fd,QSocketNotifier::Read,this);m_write=new QSocketNotifier(m_fd,QSocketNotifier::Write,this);m_read->setEnabled(false);m_write->setEnabled(false);
 connect(m_read,&QSocketNotifier::activated,this,[this,epoch,fd]{if(epoch==m_nativeEpoch&&fd==m_fd)receive();});
 connect(m_write,&QSocketNotifier::activated,this,[this,epoch,fd]{if(epoch==m_nativeEpoch&&fd==m_fd)writable();});
 const auto connected=::connect(m_fd,reinterpret_cast<sockaddr*>(&address),sizeof(address));const auto error=errno;
 if(connected<0&&error!=EINPROGRESS){nativeFailure(PR_SOUNDS_CLIENT_ACTOR_UNAVAILABLE);return;}
 if(connected==0)writable();else sync();
}
void AudioClient::drive(){if(view().stage==PR_SOUNDS_CLIENT_CONNECTING)connectNative();else if(view().stage==PR_SOUNDS_CLIENT_SENDING)writable();else sync();}
void AudioClient::reconnect(){pr_sounds_client_reconnect_v1(m_client);drive();}
void AudioClient::playPreview(){pr_sounds_client_preview_play_v1(m_client);drive();}
void AudioClient::stopPreview(){pr_sounds_client_preview_stop_v1(m_client);drive();}
void AudioClient::select(const QString &id) {
 const auto bytes=id.toUtf8();PrSoundsClientIdV1 route{};
 if(bytes.isEmpty()||bytes.size()>int(sizeof(route.bytes)))return;
 route.length=uint32_t(bytes.size());std::memcpy(route.bytes,bytes.constData(),size_t(bytes.size()));
 pr_sounds_client_choose_v1(m_client,&route);drive();
}
void AudioClient::writable() {
 if(m_fd<0)return;
 if(view().stage==PR_SOUNDS_CLIENT_CONNECTING){
  int error=0;socklen_t size=sizeof(error);
  if(::getsockopt(m_fd,SOL_SOCKET,SO_ERROR,&error,&size)||size!=sizeof(error)||error||!peer()){nativeFailure(PR_SOUNDS_CLIENT_IDENTITY_REFUSED);return;}
  if(pr_sounds_client_connected_v1(m_client,m_nativeEpoch)!=PR_SOUNDS_CLIENT_OK){sync();return;}
 }
 PrSoundsClientPacketV1 packet{};
 if(pr_sounds_client_prepare_v1(m_client,&packet)!=PR_SOUNDS_CLIENT_OK){sync();return;}
 // No Qt signal/reentrant callback is emitted between prepare, actual send and
 // its report. Rust retains the original token/deadline through EAGAIN/EINTR.
 const auto count=::send(m_fd,packet.bytes,packet.length,MSG_DONTWAIT|MSG_NOSIGNAL);const auto error=errno;
 const uint32_t outcome=count>=0?PR_SOUNDS_CLIENT_SEND_COMPLETE:error==EAGAIN?PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK:error==EINTR?PR_SOUNDS_CLIENT_SEND_INTERRUPTED:PR_SOUNDS_CLIENT_SEND_FAILED;
 pr_sounds_client_sent_v1(m_client,packet.connection_epoch,packet.token,outcome,count>=0?uint32_t(count):0);sync();
}
void AudioClient::receive() {
 if(m_fd<0)return;
 pr_sounds_client_tick_v1(m_client);
 const auto v=view();if(v.stage==PR_SOUNDS_CLIENT_CLOSED){sync();return;}
 if(v.stage!=PR_SOUNDS_CLIENT_RECEIVING){nativeFailure(PR_SOUNDS_CLIENT_REPLY_OUTSIDE_REQUEST);return;}
 char bytes[4096];alignas(cmsghdr)char control[CMSG_SPACE(sizeof(ucred))+CMSG_SPACE(16*sizeof(int))]{};
 iovec io{bytes,sizeof(bytes)};msghdr message{};message.msg_iov=&io;message.msg_iovlen=1;message.msg_control=control;message.msg_controllen=sizeof(control);
 const auto count=::recvmsg(m_fd,&message,MSG_DONTWAIT|MSG_CMSG_CLOEXEC);const auto error=errno;
 if(count<0&&(error==EAGAIN||error==EINTR)){pr_sounds_client_tick_v1(m_client);sync();return;}
 bool trusted=false,seen=false,invalid=message.msg_flags&(MSG_TRUNC|MSG_CTRUNC);
 for(auto *c=CMSG_FIRSTHDR(&message);count>=0&&c;c=CMSG_NXTHDR(&message,c)){
  if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_CREDENTIALS&&c->cmsg_len==CMSG_LEN(sizeof(ucred))&&!seen){ucred who{};std::memcpy(&who,CMSG_DATA(c),sizeof(who));seen=true;trusted=who.uid==m_ownerUid&&who.gid==0&&who.pid==m_peerPid;}
  else {invalid=true;if(c->cmsg_level==SOL_SOCKET&&c->cmsg_type==SCM_RIGHTS&&c->cmsg_len>=CMSG_LEN(0)){const auto handles=(c->cmsg_len-CMSG_LEN(0))/sizeof(int);const auto *fds=reinterpret_cast<const int*>(CMSG_DATA(c));for(size_t i=0;i<handles;++i)::close(fds[i]);}}
 }
 if(count<=0||!trusted||invalid){nativeFailure(PR_SOUNDS_CLIENT_PACKET_REFUSED);return;}
 pr_sounds_client_reply_v1(m_client,m_nativeEpoch,reinterpret_cast<const uint8_t*>(bytes),uint32_t(count));drive();
}
