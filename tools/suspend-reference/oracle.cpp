// Our executable boundary harness; fetched Android suspend algorithms are unchanged.
#include "platform.h"
#include "SystemSuspend.h"
#include "SystemSuspendAidl.h"
#include <future>
#include <filesystem>
#include <stdexcept>
#include <sys/stat.h>
#include <sys/wait.h>
#include <signal.h>
#include <sys/prctl.h>
#undef open
using android::base::unique_fd;
using android::system::suspend::V1_0::SystemSuspend;
using android::system::suspend::V1_0::SleepTimeConfig;
using android::system::suspend::V1_0::SuspendControlService;
using android::system::suspend::V1_0::SuspendControlServiceInternal;
using aidl::android::system::suspend::SystemSuspendAidl;
using aidl::android::system::suspend::IWakeLock;
using aidl::android::system::suspend::WakeLockType;
static void require(bool value,const char* message){if(!value)throw std::runtime_error(message);}
static bool ready(int fd,int ms=150){pollfd p{fd,POLLIN,0};return ::poll(&p,1,ms)>0&&(p.revents&POLLIN);}
static std::string packet(int fd){require(ready(fd,2000),"packet timeout");char b[64];auto n=::read(fd,b,sizeof b);require(n>0,"packet EOF");return {b,static_cast<size_t>(n)};}
static void send(int fd,const std::string& x){require(::write(fd,x.data(),x.size())==static_cast<ssize_t>(x.size()),"packet write failed");}
static bool same(int a,int b){struct stat x{},y{};return fstat(a,&x)==0&&fstat(b,&y)==0&&x.st_dev==y.st_dev&&x.st_ino==y.st_ino;}
template<class F>static void until(F f,const char* message){auto end=std::chrono::steady_clock::now()+2s;while(!f()){require(std::chrono::steady_clock::now()<end,message);std::this_thread::sleep_for(1ms);}}
class PeerToken:public android::IBinder {int fd;public:explicit PeerToken(int f):fd(f){}int pingBinder()override{char b;auto n=recv(fd,&b,1,MSG_PEEK|MSG_DONTWAIT);return n==0?-32:android::OK;}};
struct Child {
    pid_t pid=-1;unique_fd socket;
    Child(){int pair[2];require(socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,pair)==0,"child socketpair");pid=fork();require(pid>=0,"fork");if(pid==0){prctl(PR_SET_PDEATHSIG,SIGKILL);close(pair[0]);char b;while(read(pair[1],&b,1)>0){};_exit(0);}close(pair[1]);socket.reset(pair[0]);}
    void terminate(){if(pid>0){require(kill(pid,SIGKILL)==0,"owned child kill");int status=0;require(waitpid(pid,&status,0)==pid,"owned child reap");pid=-1;}}
    ~Child(){if(pid>0){kill(pid,SIGKILL);waitpid(pid,nullptr,0);}}
};
struct Fixture {
    std::filesystem::path root;
    unique_fd count_peer,state_peer,lock_file,unlock_file,reasons_file,time_file;
    int count_fd=-1,state_fd=-1;
    android::sp<SuspendControlService> control;
    android::sp<SuspendControlServiceInternal> internal;
    android::sp<SystemSuspend> system;
    std::shared_ptr<SystemSuspendAidl> service;
    Fixture(std::string path,bool fail_state=false):root(std::move(path)),control(new SuspendControlService()),internal(new SuspendControlServiceInternal()) {
        require(std::filesystem::create_directory(root),"fresh owned directory required");
        auto file=[&](const char* name,const char* contents){auto p=(root/name).string();int fd=::open(p.c_str(),O_CREAT|O_EXCL|O_RDWR|O_CLOEXEC,0600);require(fd>=0,"temporary file");send(fd,contents);lseek(fd,0,SEEK_SET);return unique_fd(fd);};
        lock_file=file("wake-lock","");unlock_file=file("wake-unlock","");reasons_file=file("reasons","fixture\n");time_file=file("time","0 0\n");
        reference::wake_lock_fd=lock_file.get();reference::wake_unlock_fd=unlock_file.get();
        int count[2],state[2];require(socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,count)==0,"count pair");count_fd=count[0];count_peer.reset(count[1]);
        unique_fd state_owner;
        if(fail_state){auto p=(root/"failed-state").string();int initial=::open(p.c_str(),O_CREAT|O_EXCL|O_RDWR|O_CLOEXEC,0600);require(initial>=0,"state fixture");close(initial);state_owner.reset(::open(p.c_str(),O_RDONLY|O_CLOEXEC));state_fd=state_owner.get();}
        else {require(socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,state)==0,"state pair");state_fd=state[0];state_owner.reset(state[0]);state_peer.reset(state[1]);}
        SleepTimeConfig config{10ms,100ms,2.0,0,0ms,true,false,0ms};
        system=new SystemSuspend(unique_fd(count[0]),std::move(state_owner),unique_fd(-1),8,unique_fd(-1),unique_fd(dup(reasons_file.get())),unique_fd(dup(time_file.get())),config,control,internal,true);
        service=std::make_shared<SystemSuspendAidl>(system.get());
    }
    std::shared_ptr<IWakeLock> acquire(){std::shared_ptr<IWakeLock> out;require(service->acquireWakeLock(WakeLockType::PARTIAL,"synthetic",&out).isOk()&&out,"original acquire");return out;}
    void enable(android::sp<android::IBinder> token=android::sp<android::IBinder>(new android::IBinder())){require(system->enableAutosuspend(token),"enable failed");}
    void cycle(const std::string& count="7"){send(count_peer.get(),count);require(packet(count_peer.get())==count,"count not committed unchanged");require(packet(state_peer.get())=="mem","missing mem write");}
    ~Fixture(){
        if(system.get()){
            system->disableAutosuspend();
            char b='0';::write(count_peer.get(),&b,1);shutdown(count_peer.get(),SHUT_RDWR);
            service.reset();system=nullptr; // Original destructor observes worker stop after FD unblock.
        }
        reference::before_write={};std::filesystem::remove_all(root);
    }
};
int main(int argc,char** argv){
    require(argc==3,"case and fresh private directory required");std::string mode=argv[1];
    if(mode=="dead-controller"||mode=="dead-client"){
        Child child;Fixture f(argv[2]);
        if(mode=="dead-controller"){
            auto token=android::sp<android::IBinder>(new PeerToken(child.socket.get()));f.enable(token);child.terminate();send(f.count_peer.get(),"7");require(!ready(f.count_peer.get())&&!ready(f.state_peer.get()),"dead controller allowed commit");
        }else{
            auto lock=f.acquire();f.enable();send(f.count_peer.get(),"7");require(!ready(f.count_peer.get()),"live client did not block");child.terminate();require(ready(child.socket.get()),"client EOF not observed");char b;require(read(child.socket.get(),&b,1)==0,"client lifetime EOF");lock.reset();require(packet(f.count_peer.get())=="7"&&packet(f.state_peer.get())=="mem","destruction did not unblock");require(f.control->released==1,"destruction released more than once");
        }
    }else if(mode=="ordering"){
        Fixture f(argv[2]);f.enable();f.cycle();require(!f.system->enableAutosuspend(android::sp<android::IBinder>(new android::IBinder())),"duplicate enable succeeded");
    }else if(mode=="idempotent-release"){
        Fixture f(argv[2]);auto lock=f.acquire();lock->release();lock->release();lock.reset();require(f.control->acquired==1&&f.control->released==1,"original release accounting not once");f.enable();f.cycle();
    }else if(mode=="destructor-release"){
        Fixture f(argv[2]);auto lock=f.acquire();f.enable();send(f.count_peer.get(),"7");require(!ready(f.count_peer.get()),"held wake lock allowed commit");lock.reset();require(packet(f.count_peer.get())=="7"&&packet(f.state_peer.get())=="mem","destructor did not unblock");
    }else if(mode=="concurrent-acquire"){
        Fixture f(argv[2]);std::mutex m;std::condition_variable cv;bool commit=false,proceed=false,abort=false,acquired=false;std::vector<std::string> trace;
        reference::before_write=[&](int fd,const std::string& value){std::unique_lock guard(m);if(fd==f.count_fd){trace.push_back("commit");commit=true;cv.notify_all();cv.wait(guard,[&]{return proceed;});}if(same(fd,f.lock_file.get())&&value=="userspace-abort"){trace.push_back("abort-lock");abort=true;cv.notify_all();}if(fd==f.state_fd)trace.push_back("mem");if(same(fd,f.unlock_file.get())&&value=="userspace-abort")trace.push_back("abort-unlock");return true;};
        f.enable();send(f.count_peer.get(),"7");{std::unique_lock g(m);require(cv.wait_for(g,2s,[&]{return commit;}),"commit gate timeout");}
        std::shared_ptr<IWakeLock> lock;std::thread acquire([&]{lock=f.acquire();std::lock_guard g(m);acquired=true;cv.notify_all();});
        {std::unique_lock g(m);require(cv.wait_for(g,2s,[&]{return abort;}),"missing abort-before-mutex");require(!acquired,"acquire completed inside commit critical section");proceed=true;cv.notify_all();}
        require(packet(f.count_peer.get())=="7"&&packet(f.state_peer.get())=="mem","protected commit incomplete");acquire.join();require(acquired,"acquire not completed after commit");
        {std::lock_guard g(m);auto a=std::find(trace.begin(),trace.end(),"abort-lock"),s=std::find(trace.begin(),trace.end(),"mem"),u=std::find(trace.begin(),trace.end(),"abort-unlock");require(a<s&&s<u,"abort/commit/unlock ordering changed");}
        reference::before_write={};lock.reset();
    }else if(mode=="count-reject"){
        Fixture f(argv[2]);std::atomic<bool> reject{true},seen{false};reference::before_write=[&](int fd,const std::string&){if(fd==f.count_fd&&reject.exchange(false)){seen=true;errno=EINVAL;return false;}return true;};f.enable();send(f.count_peer.get(),"7");until([&]{return seen.load();},"reject not reached");require(!ready(f.state_peer.get()),"failed count commit wrote mem");f.cycle("8");
    }else if(mode=="state-write-failure"){
        Fixture f(argv[2],true);f.enable();send(f.count_peer.get(),"7");require(packet(f.count_peer.get())=="7","count commit missing");until([&]{return f.control->notifications.load()>0;},"failure callback missing");require(!f.control->last_success,"failed state reported success");require(f.system->getSleepTime()==20ms,"original failed-suspend backoff not applied");
    }else throw std::runtime_error("unknown case");
    std::cout<<mode<<"\tPASS\n";
}
