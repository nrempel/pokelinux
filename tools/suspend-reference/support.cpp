#include "platform.h"
#include "WakeupList.h"
namespace reference {int wake_lock_fd=-1,wake_unlock_fd=-1;std::function<bool(int,const std::string&)> before_write;}
namespace android::system::suspend::V1_0 {
WakeupList::WakeupList(size_t capacity):mCapacity(capacity){}
void WakeupList::update(const std::vector<std::string>&){}
void WakeupList::getWakeupStats(std::vector<internal::WakeupInfo>*)const{}
}
