#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <string>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>
#include "producer-under-test.inc"
using Producer = CCallbackSpanRing;
static_assert(Producer::FILE_BYTES == 82048);

static uint64_t deadline = 0;
static void check(bool ok, const char* what) {
    if (!ok) { std::fprintf(stderr, "COMPONENT_ASSERT:%s errno=%d\n", what, errno); std::exit(1); }
}
static uint64_t boot() {
    timespec ts{}; check(clock_gettime(CLOCK_BOOTTIME, &ts) == 0, "actual BOOTTIME");
    return uint64_t(ts.tv_sec) * 1000000000ULL + uint64_t(ts.tv_nsec);
}
static void within() { check(boot() < deadline, "original program 5s cutoff"); }
static void createFile(const std::string& path, off_t bytes, mode_t mode = 0600) {
    int fd = open(path.c_str(), O_RDWR | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
    check(fd >= 0, "create exclusive component file");
    check(ftruncate(fd, bytes) == 0 && fchmod(fd, mode) == 0, "size/mode component file");
    check(close(fd) == 0, "close fixture fd");
}
static bool allZero(const std::string& path, off_t expected) {
    int fd = open(path.c_str(), O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    check(fd >= 0, "read component file");
    struct stat st{}; check(fstat(fd, &st) == 0 && st.st_size == expected, "unchanged fixture size");
    unsigned char buf[4096]; off_t total = 0;
    while (total < expected) {
        const auto n = read(fd, buf, sizeof(buf)); check(n > 0, "fixture read");
        for (ssize_t i = 0; i < n; ++i) if (buf[i]) { check(close(fd) == 0, "close nonzero read fd"); return false; }
        total += n;
    }
    check(close(fd) == 0, "close zero read fd"); return true;
}
static void unavailable(Producer& p) {
    int calls = 0; errno = ENOSPC;
    const auto token = p.enter(1, 123, 7); check(errno == ENOSPC && token.span == 0, "unavailable enter/errno");
    ++calls; errno = EAGAIN; p.normalReturn(token);
    check(calls == 1 && errno == EAGAIN, "callback still executes when unavailable");
}
int main(int argc, char** argv) {
    check(argc == 2, "one output directory");
    const std::string base = argv[1]; const uint64_t started = boot(); deadline = started + 5000000000ULL;
    check(getuid() == 1000 && getgid() == 1000, "actual selected component identity");
    std::printf("COMPONENT_ID pid=%ld tid=%ld started=%llu until=%llu\n", long(getpid()), long(syscall(SYS_gettid)),
                (unsigned long long)started, (unsigned long long)deadline);
    std::fflush(stdout);
    const std::string normal = base + "/normal.bin", untouched = base + "/untouched.bin";
    createFile(normal, Producer::FILE_BYTES); createFile(untouched, Producer::FILE_BYTES);
    {
        Producer p; errno = EBUSY; check(p.arm(normal.c_str()), "real mmap arm"); check(errno == EBUSY, "arm preserves errno");
        check(!p.arm(untouched.c_str()) && errno == EBUSY, "arm once only");
        int calls = 0; errno = ENOSPC; auto a = p.enter(1, 0x1234, 7); check(a.span == 1 && errno == ENOSPC, "AQ entry/errno");
        ++calls; errno = EAGAIN; p.normalReturn(a); check(errno == EAGAIN, "normal return preserves callback errno");
        auto t = p.enter(2, 0x5678, 0); check(t.span == 3, "timer entry");
        auto d = p.enter(3, 9, 0); check(d.span == 4, "deferred nested entry");
        ++calls; p.normalReturn(d); p.normalReturn(t); check(calls == 2, "actual component callback count");
        auto noReturn = p.enter(3, 10, 0); check(noReturn.span == 7, "actual unmatched entry");
    }
    check(allZero(untouched, Producer::FILE_BYTES), "second arm untouched"); within();
    std::puts("GROUP normal-return-nesting-errno-and-unmatched-entry PASS");
    {
        Producer p; errno = EBUSY; check(!p.arm((base + "/absent.bin").c_str()) && errno == EBUSY, "missing file refusal/errno");
        unavailable(p); check(!p.arm(untouched.c_str()), "failed arm is sticky");
    }
    const std::string shortFile = base + "/short.bin", wrongMode = base + "/wrong-mode.bin", nonzero = base + "/nonzero.bin";
    createFile(shortFile, 7); createFile(wrongMode, Producer::FILE_BYTES, 0640); createFile(nonzero, Producer::FILE_BYTES);
    int fd = open(nonzero.c_str(), O_RDWR | O_CLOEXEC); check(fd >= 0, "nonzero fixture fd");
    check(pwrite(fd, "x", 1, 0) == 1 && close(fd) == 0, "write preexisting nonzero marker");
    for (const auto& path : {shortFile, wrongMode, nonzero}) {
        Producer p; check(!p.arm(path.c_str()), "invalid fixed file refusal"); unavailable(p);
    }
    check(allZero(shortFile, 7) && allZero(wrongMode, Producer::FILE_BYTES), "refused files unchanged");
    fd = open(nonzero.c_str(), O_RDONLY | O_CLOEXEC); char first = 0;
    check(fd >= 0 && read(fd, &first, 1) == 1 && first == 'x' && close(fd) == 0, "nonzero marker preserved");
    const std::string link = base + "/link.bin", fifo = base + "/wrong.fifo";
    check(symlink("normal.bin", link.c_str()) == 0 && mkfifo(fifo.c_str(), 0600) == 0, "link/FIFO fixtures");
    { Producer p; check(!p.arm(link.c_str()), "symlink refusal"); unavailable(p); }
    { Producer p; check(!p.arm(fifo.c_str()), "nonblocking FIFO refusal"); unavailable(p); }
    within(); std::puts("GROUP fixed-file-unavailable-and-sticky-refusals PASS");
    const std::string rolling = base + "/rolling.bin"; createFile(rolling, Producer::FILE_BYTES);
    {
        Producer p; check(p.arm(rolling.c_str()), "rolling real mmap arm");
        for (uint64_t i = 0; i < 600; ++i) { auto token = p.enter(3, i + 1, 0); check(token.span == 2 * i + 1, "rolling reservation"); p.normalReturn(token); }
    }
    within(); std::puts("GROUP actual-rolling-overflow PASS");
    timespec cpu{}; check(clock_gettime(CLOCK_THREAD_CPUTIME_ID, &cpu) == 0, "actual thread CPU clock");
    const uint64_t done = boot(); check(done < deadline, "completed before original cutoff");
    std::printf("COMPONENT_DONE boottime=%llu thread_cpu=%llu\n", (unsigned long long)done,
                (unsigned long long)(uint64_t(cpu.tv_sec) * 1000000000ULL + uint64_t(cpu.tv_nsec)));
    return 0;
}
