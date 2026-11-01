#pragma once
// Private numeric event landmarks only. No Source, focus, policy or output input.
#include <atomic>
#include <bit>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <type_traits>
#include <fcntl.h>
#include <linux/magic.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <time.h>
#include <unistd.h>
namespace startup_events {
inline constexpr char PATH[] = "/run/user/1000/pixel10-profile/startup-events.bin";
inline constexpr char MAGIC[8] = {'P','L','S','E','V','T','0','1'};
inline constexpr uint32_t VERSION=1, CAPACITY=128;
inline constexpr size_t FILE_BYTES=4096;
enum class Kind : uint32_t { WitnessArmed=1, PreReload=2, Reloaded=3, PropsRefreshed=4 };
inline constexpr uint64_t OVERFLOW=1, CLOCK_ERROR=2, RETIRED=4;
struct alignas(64) Header {
    char magic[8];
    uint32_t version, header_bytes, record_bytes, capacity;
    alignas(8) uint64_t committed;
    alignas(8) uint64_t flags;
    uint64_t reserved[3];
};
struct Record { uint64_t ordinal, boottime_ns; uint32_t kind, detail; };
struct Ring {
    Header header;
    Record records[CAPACITY];
    unsigned char padding[FILE_BYTES-sizeof(Header)-sizeof(Record)*CAPACITY];
};
static_assert(std::endian::native==std::endian::little);
static_assert(sizeof(Header)==64 && sizeof(Record)==24 && sizeof(Ring)==FILE_BYTES);
static_assert(std::is_standard_layout_v<Ring> && std::is_trivially_copyable_v<Ring>);
static_assert(std::atomic_ref<uint64_t>::is_always_lock_free);
static_assert(offsetof(Header,committed)%std::atomic_ref<uint64_t>::required_alignment==0);
static_assert(offsetof(Header,flags)%std::atomic_ref<uint64_t>::required_alignment==0);
class Mapping {
    Ring* ring_=nullptr;
  public:
    void openAndPrefault() {
        const int fd=open(PATH,O_RDWR|O_CLOEXEC|O_NOFOLLOW|O_NONBLOCK);
        struct stat st{}; struct statfs fs{};
        const bool valid=fd>=0 && fstat(fd,&st)==0 && S_ISREG(st.st_mode)
            && st.st_uid==1000 && st.st_gid==1000 && (st.st_mode&07777)==0600
            && st.st_nlink==1 && st.st_size==static_cast<off_t>(FILE_BYTES)
            && fstatfs(fd,&fs)==0 && fs.f_type==TMPFS_MAGIC;
        if (!valid) { if(fd>=0)close(fd); throw std::runtime_error("startup event file mismatch"); }
        void* memory=mmap(nullptr,FILE_BYTES,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);
        close(fd);
        if(memory==MAP_FAILED)throw std::runtime_error("startup event mapping unavailable");
        ring_=static_cast<Ring*>(memory);
        const auto& h=ring_->header;
        const bool shape=std::memcmp(h.magic,MAGIC,8)==0 && h.version==VERSION
            && h.header_bytes==sizeof(Header) && h.record_bytes==sizeof(Record)
            && h.capacity==CAPACITY && h.committed==0 && h.flags==0
            && h.reserved[0]==0 && h.reserved[1]==0 && h.reserved[2]==0;
        bool empty=true;
        auto* bytes=static_cast<unsigned char*>(memory);
        for(size_t i=sizeof(Header);i<FILE_BYTES;++i)empty=empty && bytes[i]==0;
        if(!shape || !empty) { closeMapping(); throw std::runtime_error("startup event ABI not fresh"); }
        // Prefault every cache line before any event listener is registered.
        volatile unsigned char* touched=bytes;
        for(size_t i=0;i<FILE_BYTES;i+=64)touched[i]=touched[i];
    }
    // Single compositor event-loop writer. No allocation/formatting/I/O here;
    // the only event-path syscall is CLOCK_BOOTTIME. No wrap or retries.
    void publish(Kind kind,uint32_t detail=0) noexcept {
        if(!ring_)return;
        auto committed=std::atomic_ref<uint64_t>(ring_->header.committed);
        auto flags=std::atomic_ref<uint64_t>(ring_->header.flags);
        if(flags.load(std::memory_order_relaxed)&RETIRED)return;
        const uint64_t count=committed.load(std::memory_order_relaxed);
        if(count>=CAPACITY) { flags.fetch_or(OVERFLOW,std::memory_order_release); return; }
        timespec t{};
        if(clock_gettime(CLOCK_BOOTTIME,&t)!=0 || t.tv_sec<0 || t.tv_nsec<0 || t.tv_nsec>=1000000000L
            || static_cast<uint64_t>(t.tv_sec)>(std::numeric_limits<uint64_t>::max()-static_cast<uint64_t>(t.tv_nsec))/1000000000ULL) {
            flags.fetch_or(CLOCK_ERROR,std::memory_order_release); return;
        }
        const uint64_t stamp=static_cast<uint64_t>(t.tv_sec)*1000000000ULL+static_cast<uint64_t>(t.tv_nsec);
        ring_->records[count]={count+1,stamp,static_cast<uint32_t>(kind),detail};
        committed.store(count+1,std::memory_order_release);
    }
    void retire() noexcept {
        if(ring_)std::atomic_ref<uint64_t>(ring_->header.flags).fetch_or(RETIRED,std::memory_order_release);
    }
    void closeMapping() noexcept {
        if(ring_) { munmap(ring_,FILE_BYTES); ring_=nullptr; }
    }
};
}
