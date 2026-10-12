// Fixture OS/logging glue only. Original queue, EventFlag and native-handle code stay unchanged.
#include <cutils/ashmem.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include <cstdio>
#include <cstdlib>
#include <string>
extern "C" int ashmem_create_region(const char* name,size_t size) {
    int fd=memfd_create(name,MFD_CLOEXEC|MFD_ALLOW_SEALING);
    if(fd<0)return -1;
    if(ftruncate(fd,size)!=0 || fcntl(fd,F_ADD_SEALS,F_SEAL_GROW|F_SEAL_SHRINK)!=0){close(fd);return -1;}
    return fd;
}
extern "C" int ashmem_get_size_region(int fd) {struct stat s;return fstat(fd,&s)==0?static_cast<int>(s.st_size):-1;}
namespace android {
int64_t elapsedRealtimeNano(){timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return static_cast<int64_t>(t.tv_sec)*1000000000+t.tv_nsec;}
int64_t elapsedRealtime(){return elapsedRealtimeNano()/1000000;}
namespace hardware::details {
void check(bool expected){if(!expected)abort();}
void check(bool expected,const char* text){if(!expected){fprintf(stderr,"FMQ check: %s\n",text);abort();}}
void logError(const std::string& text){fprintf(stderr,"FMQ: %s\n",text.c_str());}
void logWarning(const std::string& text){fprintf(stderr,"FMQ warning: %s\n",text.c_str());}
void logDebug(const std::string&){}
void errorWriteLog(int,const char*){}
}}
