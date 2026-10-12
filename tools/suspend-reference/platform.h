// Our host platform adapter. Suspend algorithms remain in fetched source.
#pragma once
#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdarg>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <fcntl.h>
#include <functional>
#include <filesystem>
#include <fstream>
#include <future>
#include <locale>
#include <map>
#include <memory>
#include <mutex>
#include <poll.h>
#include <sstream>
#include <string>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>
#include <vector>
#include <iostream>
using namespace std::chrono_literals;
#define REQUIRES(...)
#define GUARDED_BY(...)
#define ACQUIRED_AFTER(...)
#define EXCLUSIVE_LOCKS_REQUIRED(...)
#define LOG(...) std::cerr
#define PLOG(...) std::cerr
#define ATRACE_TAG_POWER 0
#define ATRACE_INSTANT_FOR_TRACK(...) ((void)0)
#define ANDROID_SYSTEM_SYSTEM_SUSPEND_CONTROL_SERVICE_H
#define ANDROID_SYSTEM_SUSPEND_WAKE_LOCK_ENTRY_LIST_H
namespace reference {
extern int wake_lock_fd, wake_unlock_fd;
extern std::function<bool(int,const std::string&)> before_write;
inline int open(const char* path,int flags) {
    if(std::strcmp(path,"/sys/power/wake_lock")==0)return ::dup(wake_lock_fd);
    if(std::strcmp(path,"/sys/power/wake_unlock")==0)return ::dup(wake_unlock_fd);
    if(std::strncmp(path,"/sys/",5)==0||std::strncmp(path,"/dev/",5)==0)std::abort();
    return ::open(path,flags);
}
}
namespace android {
class RefBase {public:virtual ~RefBase()=default;};
template<class T> class sp {
    std::shared_ptr<T> p;
public:
    sp()=default;sp(std::nullptr_t){};sp(T* value):p(value,[](T* x){delete static_cast<RefBase*>(x);}){}
    template<class... A> static sp make(A&&... a){return sp(new T(std::forward<A>(a)...));}
    T* get()const{return p.get();}T* operator->()const{return p.get();}
    bool operator==(const sp& other)const{return get()==other.get();}
};
using status_t=int;constexpr status_t OK=0;
class IBinder:public RefBase {public:virtual status_t pingBinder(){return OK;}};
class IPCThreadState {public:static IPCThreadState* self(){static IPCThreadState s;return &s;}int getCallingPid(){return ::getpid();}};
namespace base {
class unique_fd {int fd=-1;public:unique_fd()=default;explicit unique_fd(int n):fd(n){}~unique_fd(){reset();}unique_fd(unique_fd&& x):fd(x.release()){}unique_fd& operator=(unique_fd&& x){reset(x.release());return *this;}unique_fd(const unique_fd&)=delete;void reset(int n=-1){if(fd>=0)::close(fd);fd=n;}int release(){int n=fd;fd=-1;return n;}int get()const{return fd;}operator int()const{return fd;}};
inline bool WriteStringToFd(const std::string& value,int fd){if(reference::before_write&&!reference::before_write(fd,value))return false;size_t p=0;while(p<value.size()){auto n=::write(fd,value.data()+p,value.size()-p);if(n<0&&errno==EINTR)continue;if(n<=0)return false;p+=n;}return true;}
inline bool ReadFdToString(int fd,std::string* out){out->clear();char b[4096];while(true){auto n=::read(fd,b,sizeof b);if(n<0&&errno==EINTR)continue;if(n<0)return false;if(n==0)return true;out->append(b,n);if(out->size()>65536)return false;}}
inline bool WriteStringToFile(const std::string&,const char* path){if(std::strncmp(path,"/sys/",5)==0)std::abort();return false;}
inline std::string Trim(const std::string& x){auto lo=x.find_first_not_of(" \r\n\t");return lo==std::string::npos?"":x.substr(lo,x.find_last_not_of(" \r\n\t")-lo+1);}
inline std::string StringPrintf(const char* f,...){char b[4096];va_list a;va_start(a,f);vsnprintf(b,sizeof b,f,a);va_end(a);return b;}
template<class T>bool ParseInt(const std::string& x,T* out){try{*out=static_cast<T>(std::stoll(x));return true;}catch(...){return false;}}
template<class T>bool ParseUint(const std::string& x,T* out){try{*out=static_cast<T>(std::stoull(x));return true;}catch(...){return false;}}
class CachedProperty {public:explicit CachedProperty(const char*){}const char* Get(const char*){return "";}};
struct ScopedLockAssertion {explicit ScopedLockAssertion(std::mutex&){} };
struct Error {template<class T>Error& operator<<(const T&){return *this;}};
template<class T>class Result {T value{};public:Result(T x):value(std::move(x)){}Result(const Error&){} };
}
namespace system::suspend::internal {
struct SuspendInfo {
    int64_t breakEvenMillis=0,earlyRecoveryBadSuspends=0,newBadSuspends=0,maxBackoffContinuations=0;
    int64_t suspendAttemptCount=0,sleepTimeMillis=0,suspendOverheadTimeMillis=0,suspendTimeMillis=0;
    int64_t failedSuspendCount=0,failedSuspendOverheadTimeMillis=0,shortSuspendCount=0,shortSuspendTimeMillis=0;
    int64_t newBackoffCount=0,backoffContinueCount=0;
    std::vector<int64_t> consecutiveBadSuspendBins,suspendDurationMillisBins;
};
struct WakeLockInfo {bool isKernelWakelock=false;int64_t activeCount=0,totalTime=0;std::string name;};
struct WakeupInfo {};
struct ISuspendControlServiceInternal {static constexpr int WAKE_LOCK_INFO_ACTIVE_COUNT=1,WAKE_LOCK_INFO_TOTAL_TIME=2;};
}
namespace system::suspend::V1_0 {
class SystemSuspend;
class SuspendControlService:public RefBase {public:std::atomic<int> notifications{0},acquired{0},released{0};std::atomic<bool> last_success{false};void notifyWakeup(bool success,const std::vector<std::string>&){last_success=success;notifications++;}void notifyWakelock(const std::string&,bool held){if(held)acquired++;else released++;}};
class SuspendControlServiceInternal:public RefBase {public:void setSuspendService(SystemSuspend*){} };
class WakeLockEntryList {public:WakeLockEntryList(size_t,base::unique_fd){}void updateOnAcquire(const std::string&,int){}void updateOnRelease(const std::string&,int){}void updateNow(){}void getWakeLockStats(int,std::vector<internal::WakeLockInfo>*)const{} };
}
}
using ::android::system::suspend::internal::WakeLockInfo;
namespace suspend_service::flags {inline bool suspend_backoff_observability(){return false;}inline bool extended_suspend_info(){return false;}inline bool enable_pm_fs_abort(){return false;}}
using AIBinder=void;
inline AIBinder* AServiceManager_checkService(const char*){return nullptr;}
inline AIBinder* AStatus_fromExceptionCode(int){return nullptr;}
constexpr int EX_ILLEGAL_ARGUMENT=-3;
namespace ndk {
class SpAIBinder {public:explicit SpAIBinder(AIBinder*){} };
class ScopedAStatus {public:ScopedAStatus()=default;explicit ScopedAStatus(AIBinder*){}static ScopedAStatus ok(){return {};}bool isOk()const{return true;} };
class SharedRefBase {public:template<class T,class... A>static std::shared_ptr<T> make(A&&... a){return std::make_shared<T>(std::forward<A>(a)...);} };
}
namespace aidl::android::system::suspend {
enum class WakeLockType {PARTIAL=0,FULL=1};
class IWakeLock {public:virtual~IWakeLock()=default;virtual ndk::ScopedAStatus release()=0;};
class BnWakeLock:public IWakeLock {};
class ISystemSuspend {public:static inline std::string descriptor="android.system.suspend.ISystemSuspend";static std::shared_ptr<ISystemSuspend> fromBinder(const ndk::SpAIBinder&){return nullptr;}virtual ndk::ScopedAStatus acquireWakeLock(WakeLockType,const std::string&,std::shared_ptr<IWakeLock>*)=0;};
class BnSystemSuspend:public ISystemSuspend {};
}
#define open reference::open
