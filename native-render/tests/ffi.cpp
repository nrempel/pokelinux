// Supplemental actual C++ -> Rust payload/lifetime integration, not a render GREEN.
#include "pokelinux_render.h"

#include <array>
#include <atomic>
#include <cerrno>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <fcntl.h>
#include <iostream>
#include <limits>
#include <linux/memfd.h>
#include <memory>
#include <poll.h>
#include <signal.h>
#include <stdexcept>
#include <string>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <thread>
#include <time.h>
#include <type_traits>
#include <unistd.h>
#include <utility>

static_assert(sizeof(void*) == 8);
static_assert(std::is_standard_layout_v<PrShmDescription>);
static_assert(sizeof(PrShmDescription) == 48 && alignof(PrShmDescription) == 8);
static_assert(offsetof(PrShmDescription, drm_format) == 24);
static_assert(offsetof(PrShmDescription, reserved) == 44);
static_assert(std::is_standard_layout_v<PrSnapshotView>);
static_assert(sizeof(PrSnapshotView) == 64 && alignof(PrSnapshotView) == 8);
static_assert(offsetof(PrSnapshotView, pixels) == 48);
static_assert(offsetof(PrSnapshotView, pixel_bytes) == 56);

static constexpr uint32_t WIDTH = 3, HEIGHT = 4, STRIDE = 16, BPP = 4;
static constexpr size_t BYTES = STRIDE * HEIGHT;
static constexpr uint32_t XRGB8888 = uint32_t('X') | uint32_t('R') << 8 |
    uint32_t('2') << 16 | uint32_t('4') << 24;
using Bytes = std::array<uint8_t, BYTES>;

static void require(bool condition, const char* reason) {
    if (!condition)
        throw std::runtime_error(reason);
}

static uint64_t now() {
    timespec ts{};
    require(clock_gettime(CLOCK_BOOTTIME, &ts) == 0, "CLOCK_BOOTTIME");
    return uint64_t(ts.tv_sec) * 1000000000ULL + uint64_t(ts.tv_nsec);
}

struct Deadlines {
    uint64_t work;
    uint64_t close;
};

class Fd {
  public:
    explicit Fd(int fd = -1) : value(fd) {}
    ~Fd() { reset(); }
    Fd(const Fd&) = delete;
    Fd& operator=(const Fd&) = delete;
    Fd(Fd&& other) noexcept : value(std::exchange(other.value, -1)) {}
    Fd& operator=(Fd&& other) noexcept {
        reset(std::exchange(other.value, -1));
        return *this;
    }
    int get() const { return value; }
    void reset(int next = -1) noexcept {
        if (value >= 0)
            ::close(value);
        value = next;
    }
  private:
    int value;
};

struct Pipe {
    Fd read, write;
    Pipe() {
        int ends[2] = {-1, -1};
        require(pipe2(ends, O_CLOEXEC | O_NONBLOCK) == 0, "pipe2");
        read.reset(ends[0]);
        write.reset(ends[1]);
    }
};

class Mapping {
  public:
    Mapping(int fd, int protection) : data(mmap(nullptr, BYTES, protection, MAP_SHARED, fd, 0)) {
        require(data != MAP_FAILED, "mmap shared memfd");
    }
    ~Mapping() { reset(); }
    Mapping(const Mapping&) = delete;
    Mapping& operator=(const Mapping&) = delete;
    uint8_t* bytes() const { return static_cast<uint8_t*>(data); }
    bool retire() noexcept {
        if (data == MAP_FAILED)
            return true;
        void* old = std::exchange(data, MAP_FAILED);
        return munmap(old, BYTES) == 0;
    }
    void reset() noexcept { (void)retire(); }
  private:
    void* data;
};

static void ready(int fd, short events, uint64_t cutoff) {
    for (;;) {
        const auto current = now();
        require(current < cutoff, "original pipe/pidfd cutoff");
        const auto remaining = cutoff - current;
        const int milliseconds = remaining >= 20000000 ? 20 : int((remaining + 999999) / 1000000);
        pollfd descriptor{fd, events, 0};
        const int count = poll(&descriptor, 1, milliseconds);
        if (count < 0 && errno == EINTR)
            continue;
        require(count >= 0 && !(descriptor.revents & POLLNVAL), "poll owned fd");
        if (count > 0) {
            require(now() < cutoff, "actual poll observed before original cutoff");
            return;
        }
    }
}

static void send_byte(int fd, char value, uint64_t cutoff) {
    for (;;) {
        ready(fd, POLLOUT, cutoff);
        const auto count = write(fd, &value, 1);
        if (count < 0 && (errno == EINTR || errno == EAGAIN))
            continue;
        require(count == 1 && now() < cutoff, "bounded one-byte pipe write");
        return;
    }
}

static char receive_byte(int fd, uint64_t cutoff) {
    for (;;) {
        ready(fd, POLLIN, cutoff);
        char value = 0;
        const auto count = read(fd, &value, 1);
        if (count < 0 && (errno == EINTR || errno == EAGAIN))
            continue;
        require(count == 1 && now() < cutoff, "bounded one-byte pipe read");
        return value;
    }
}

static uint64_t receive_eof(int fd, uint64_t cutoff) {
    for (;;) {
        ready(fd, POLLIN, cutoff);
        char unexpected = 0;
        const auto count = read(fd, &unexpected, 1);
        if (count < 0 && (errno == EINTR || errno == EAGAIN))
            continue;
        require(count == 0, "actual PIPE EOF, no extra protocol bytes");
        const auto observed = now();
        require(observed < cutoff, "actual EOF before original cutoff");
        return observed;
    }
}

static bool path_absent(const std::string& path) {
    struct stat status{};
    if (lstat(path.c_str(), &status) == 0)
        return false;
    require(errno == ENOENT, "owned proc path lookup");
    return true;
}

static bool pidfd_dead(int fd) {
    pollfd descriptor{fd, POLLIN, 0};
    int count;
    do { count = poll(&descriptor, 1, 0); } while (count < 0 && errno == EINTR);
    require(count >= 0, "pidfd death poll");
    return count == 1 && (descriptor.revents & POLLIN);
}

static Bytes original_bytes() {
    Bytes bytes{};
    for (size_t row = 0; row < HEIGHT; ++row)
        for (size_t column = 0; column < STRIDE; ++column)
            bytes[row * STRIDE + column] = column < WIDTH * BPP ?
                uint8_t(17 + row * 23 + column * 3) : uint8_t(0xC0 + row + column);
    return bytes;
}

static PrShmDescription description() {
    return {71, 9, 23, XRGB8888, WIDTH, HEIGHT, STRIDE, BPP, 0};
}

static bool same_description(const PrShmDescription& left, const PrShmDescription& right) {
    return left.resource_id == right.resource_id && left.generation == right.generation &&
        left.commit_sequence == right.commit_sequence && left.drm_format == right.drm_format &&
        left.width == right.width && left.height == right.height && left.stride == right.stride &&
        left.bytes_per_pixel == right.bytes_per_pixel && left.reserved == right.reserved;
}

struct StoreRelease { void operator()(PrSnapshotStore* p) const { pr_snapshot_store_release(p); } };
struct SnapshotRelease { void operator()(PrSnapshot* p) const { pr_snapshot_release(p); } };
using Store = std::unique_ptr<PrSnapshotStore, StoreRelease>;
using Snapshot = std::unique_ptr<PrSnapshot, SnapshotRelease>;

static Snapshot capture(PrSnapshotStore* store, const uint8_t* data) {
    auto desc = description();
    PrSnapshot* raw = nullptr;
    const int status = pr_snapshot_capture(store, &desc, data, BYTES, &raw);
    Snapshot snapshot(raw);
    require(status == PR_OK && snapshot != nullptr, "real C API capture");
    return snapshot;
}

static void check_snapshot(const PrSnapshot* snapshot, const Bytes& expected) {
    PrSnapshotView view{};
    require(pr_snapshot_view(snapshot, &view) == PR_OK, "real C API view");
    require(same_description(view.description, description()), "snapshot description retained");
    require(view.pixels && view.pixel_bytes == BYTES, "snapshot view exact byte length");
    require(std::memcmp(view.pixels, expected.data(), BYTES) == 0, "original bytes including row padding retained");
}

class Producer {
  public:
    Producer(int memfd, const Deadlines& deadlines) : cutoffs(deadlines) {
        const pid_t parent = getpid();
        pid = fork();
        require(pid >= 0, "fork real producer");
        if (pid == 0) {
            command.write.reset();
            status.read.reset();
            int result = 17;
            try {
                require(prctl(PR_SET_PDEATHSIG, SIGKILL) == 0 && getppid() == parent, "producer parent custody");
                Mapping memory(memfd, PROT_READ | PROT_WRITE);
                const auto initial = original_bytes();
                std::memcpy(memory.bytes(), initial.data(), BYTES);
                send_byte(status.write.get(), 'R', cutoffs.work);
                require(receive_byte(command.read.get(), cutoffs.work) == 'M', "producer mutation command");
                for (size_t i = 0; i < BYTES; ++i)
                    memory.bytes()[i] = uint8_t(initial[i] ^ 0x5A);
                send_byte(status.write.get(), 'M', cutoffs.work);
                require(receive_byte(command.read.get(), cutoffs.work) == 'X', "producer release command");
                require(memory.retire(), "producer unmaps original storage");
                require(::close(memfd) == 0, "producer closes original memfd");
                send_byte(status.write.get(), 'X', cutoffs.work);
                result = 0;
            } catch (...) { result = 17; }
            command.read.reset();
            status.write.reset();
            _exit(result);
        }
        command.read.reset();
        status.write.reset();
        pidfd.reset(int(syscall(SYS_pidfd_open, pid, 0)));
        if (pidfd.get() < 0) {
            force_cleanup();
            throw std::runtime_error("pidfd_open real producer");
        }
    }
    ~Producer() { force_cleanup(); }
    Producer(const Producer&) = delete;
    Producer& operator=(const Producer&) = delete;
    void receive(char expected) { require(receive_byte(status.read.get(), cutoffs.work) == expected, "producer status"); }
    void send(char value) { send_byte(command.write.get(), value, cutoffs.work); }
    void finish() {
        command.write.reset();
        eof_at = receive_eof(status.read.get(), cutoffs.close);
        ready(pidfd.get(), POLLIN, cutoffs.close);
        int status_value = 0;
        pid_t waited;
        do { waited = waitpid(pid, &status_value, WNOHANG); } while (waited < 0 && errno == EINTR);
        require(waited == pid, "exact direct parent wait after ready pidfd");
        reaped = true;
        waited_at = now();
        require(WIFEXITED(status_value) && WEXITSTATUS(status_value) == 0, "actual producer exit0");
        require(waited_at < cutoffs.close && pidfd_dead(pidfd.get()), "producer actual wait before cutoff and pidfd dead");
        require(path_absent("/proc/" + std::to_string(pid)), "producer proc absent after wait");
        int extra = 0;
        errno = 0;
        require(waitpid(pid, &extra, WNOHANG) == -1 && errno == ECHILD, "producer exact child ECHILD");
        std::cout << "{\"event\":\"owned_producer_closure\",\"pid\":" << pid
            << ",\"actual_wait_exit\":0,\"waited_at_boottime_ns\":" << waited_at
            << ",\"pipe_EOF_at_boottime_ns\":" << eof_at
            << ",\"original_work_until_ns\":" << cutoffs.work << ",\"original_close_until_ns\":" << cutoffs.close
            << ",\"pidfd_dead\":true,\"proc_absent\":true,\"exact_ECHILD\":true,\"forced_cleanup\":false}\n";
    }
  private:
    Pipe command, status;
    Deadlines cutoffs;
    Fd pidfd;
    pid_t pid = -1;
    bool reaped = false;
    uint64_t waited_at = 0, eof_at = 0;
    void force_cleanup() noexcept {
        if (pid <= 0 || reaped)
            return;
        // Original fixed cutoff is never extended. Even an exceptional late
        // terminal wait is recorded as a failure; never abandon an owned child.
        command.write.reset();
        const int killed = kill(pid, SIGKILL); // retained unreaped direct child; PID cannot be reused
        int value = 0;
        pid_t waited;
        do { waited = waitpid(pid, &value, 0); } while (waited < 0 && errno == EINTR);
        reaped = waited == pid;
        timespec ts{};
        const bool clock_ok = clock_gettime(CLOCK_BOOTTIME, &ts) == 0;
        const uint64_t observed = clock_ok ? uint64_t(ts.tv_sec) * 1000000000ULL + uint64_t(ts.tv_nsec) : 0;
        std::cout << "{\"event\":\"producer_forced_failure_cleanup\",\"pid\":" << pid
            << ",\"kill_result\":" << killed << ",\"actual_direct_wait\":" << (reaped ? "true" : "false")
            << ",\"waited_at_boottime_ns\":" << observed << ",\"original_close_until_ns\":" << cutoffs.close
            << ",\"wait_before_original_cutoff\":" << (clock_ok && observed < cutoffs.close ? "true" : "false") << "}\n";
    }
};

static void group(const char* name, const Deadlines& cutoffs) {
    const auto completed = now();
    require(completed < cutoffs.close, "group completion before original cutoff");
    std::cout << "{\"group\":\"" << name << "\",\"passed\":true,\"completed_boottime_ns\":" << completed
        << ",\"original_work_until_ns\":" << cutoffs.work << ",\"original_close_until_ns\":" << cutoffs.close << "}\n";
}

static Snapshot producer_copy(Store& store, const Deadlines& cutoffs, const Bytes& expected) {
    Fd memfd(int(syscall(SYS_memfd_create, "pokelinux-ffi-owned-producer", MFD_CLOEXEC)));
    require(memfd.get() >= 0 && ftruncate(memfd.get(), BYTES) == 0, "real bounded memfd");
    Producer producer(memfd.get(), cutoffs);
    Mapping input(memfd.get(), PROT_READ);
    producer.receive('R');
    require(std::memcmp(input.bytes(), expected.data(), BYTES) == 0, "producer initial pattern includes padding");
    auto snapshot = capture(store.get(), input.bytes());
    producer.send('M');
    producer.receive('M');
    for (size_t i = 0; i < BYTES; ++i)
        require(input.bytes()[i] == uint8_t(expected[i] ^ 0x5A), "real producer mutation observed");
    check_snapshot(snapshot.get(), expected);
    producer.send('X');
    producer.receive('X');
    require(input.retire(), "parent unmaps original shared storage");
    memfd.reset();
    producer.finish();
    check_snapshot(snapshot.get(), expected);
    require(pr_snapshot_store_used(store.get()) == BYTES, "producer copy charges one allocation");
    return snapshot;
}

static void join_worker(std::thread& worker, const std::atomic<pid_t>& tid, uint64_t cutoff) {
    // std::thread has no timed join. Observe this exact worker's kernel task
    // disappearance under the fixed cutoff, then join the already-exited thread.
    while (now() < cutoff) {
        const pid_t value = tid.load(std::memory_order_acquire);
        if (value > 0 && path_absent("/proc/self/task/" + std::to_string(value))) {
            worker.join();
            require(now() < cutoff, "actual std::thread join before original cutoff");
            return;
        }
        poll(nullptr, 0, 1);
    }
    // Do not detach a late worker or manufacture a new grace deadline.
    worker.join();
    throw std::runtime_error("worker actual terminal join exceeded original cutoff");
}

static void transferred_clone(Store& store, Snapshot& original, const Bytes& expected, const Deadlines& cutoffs) {
    Snapshot clone(pr_snapshot_clone(original.get()));
    require(clone != nullptr && pr_snapshot_store_used(store.get()) == BYTES, "explicit clone shares one budget charge");
    Pipe gate, completion;
    std::atomic<pid_t> tid{0};
    std::thread worker([owned = std::move(clone), input = std::move(gate.read), output = std::move(completion.write),
                        expected, &tid, cutoffs]() mutable {
        tid.store(pid_t(syscall(SYS_gettid)), std::memory_order_release);
        char result = 'F';
        try {
            require(receive_byte(input.get(), cutoffs.work) == 'V', "clone worker view command");
            check_snapshot(owned.get(), expected);
            owned.reset();
            result = 'P';
        } catch (...) { owned.reset(); }
        try { send_byte(output.get(), result, cutoffs.work); } catch (...) {}
        input.reset();
        output.reset();
    });
    try {
        original.reset();
        store.reset();
        send_byte(gate.write.get(), 'V', cutoffs.work); // only after both original owners are gone
        gate.write.reset();
        require(receive_byte(completion.read.get(), cutoffs.work) == 'P', "transferred clone survives original and store");
        const auto eof_at = receive_eof(completion.read.get(), cutoffs.close);
        join_worker(worker, tid, cutoffs.close);
        const auto joined_at = now();
        std::cout << "{\"event\":\"owned_thread_closure\",\"tid\":" << tid.load()
            << ",\"actual_join\":true,\"task_absent_before_join\":true,\"joined_at_boottime_ns\":" << joined_at
            << ",\"pipe_EOF_at_boottime_ns\":" << eof_at << ",\"original_work_until_ns\":" << cutoffs.work
            << ",\"original_close_until_ns\":" << cutoffs.close << "}\n";
    } catch (...) {
        gate.write.reset(); // wakes a waiting worker; its own original work deadline also remains fixed
        if (worker.joinable())
            join_worker(worker, tid, cutoffs.close);
        throw;
    }
}

static void quota(const Bytes& expected) {
    Store store(pr_snapshot_store_create(BYTES * 2));
    require(store != nullptr, "quota store");
    auto first = capture(store.get(), expected.data());
    Snapshot one(pr_snapshot_clone(first.get())), two(pr_snapshot_clone(first.get()));
    require(one && two && pr_snapshot_store_used(store.get()) == BYTES, "clones charged exactly once");
    auto second = capture(store.get(), expected.data());
    require(pr_snapshot_store_used(store.get()) == BYTES * 2, "two distinct allocations charged");
    first.reset();
    one.reset();
    require(pr_snapshot_store_used(store.get()) == BYTES * 2, "last clone retains allocation quota");
    auto desc = description();
    PrSnapshot* refused = second.get(); // live sentinel; keep separately owned second handle
    const int status = pr_snapshot_capture(store.get(), &desc, expected.data(), BYTES, &refused);
    if (refused && refused != second.get())
        pr_snapshot_release(refused);
    require(status == PR_CAPACITY && refused == nullptr, "quota refusal has no partial output");
    require(pr_snapshot_store_used(store.get()) == BYTES * 2, "capacity refusal preserves quota");
    check_snapshot(two.get(), expected);
    check_snapshot(second.get(), expected);
    two.reset();
    require(pr_snapshot_store_used(store.get()) == BYTES, "last clone releases one charge");
    auto third = capture(store.get(), expected.data());
    require(pr_snapshot_store_used(store.get()) == BYTES * 2, "released quota is reusable");
    second.reset();
    third.reset();
    require(pr_snapshot_store_used(store.get()) == 0, "all allocation budget returned");
}

static void invalid_admissions(const Bytes& expected) {
    Store store(pr_snapshot_store_create(BYTES));
    Store sentinel_store(pr_snapshot_store_create(BYTES));
    require(store && sentinel_store, "invalid-admission stores");
    auto sentinel = capture(sentinel_store.get(), expected.data());
    const auto base = description();
    std::array<PrShmDescription, 10> cases{};
    cases.fill(base);
    cases[0].resource_id = 0;
    cases[1].generation = 0;
    cases[2].commit_sequence = 0;
    cases[3].reserved = 1;
    cases[4].drm_format = 0;
    cases[5].width = 0;
    cases[6].height = 0;
    cases[7].stride = WIDTH * BPP - 1;
    cases[8].bytes_per_pixel = 0;
    cases[9].width = std::numeric_limits<uint32_t>::max();
    auto reject = [&](const PrShmDescription& desc, const uint8_t* bytes, uint64_t length) {
        PrSnapshot* out = sentinel.get();
        const int status = pr_snapshot_capture(store.get(), &desc, bytes, length, &out);
        if (out && out != sentinel.get())
            pr_snapshot_release(out);
        require(status == PR_INVALID && out == nullptr, "invalid admission clears result without success");
        require(pr_snapshot_store_used(store.get()) == 0, "invalid admission consumes no quota");
        check_snapshot(sentinel.get(), expected);
    };
    for (const auto& desc : cases)
        reject(desc, expected.data(), BYTES);
    reject(base, expected.data(), BYTES - 1);
    std::array<uint8_t, BYTES + 1> longer{};
    std::memcpy(longer.data(), expected.data(), BYTES);
    reject(base, longer.data(), longer.size());
    auto valid = capture(store.get(), expected.data());
    require(pr_snapshot_store_used(store.get()) == BYTES, "valid capture still succeeds after all invalid admissions");
    valid.reset();
    require(pr_snapshot_store_used(store.get()) == 0, "valid final release returns budget");
}

int main() {
    const char* current_group = "native_C_ABI_and_store_admission";
    try {
        require(signal(SIGPIPE, SIG_IGN) != SIG_ERR, "ignore SIGPIPE for owned failure cleanup");
        const auto started = now();
        const Deadlines cutoffs{started + 5000000000ULL, started + 8000000000ULL};
        require(pr_snapshot_store_create(0) == nullptr, "zero budget refused");
        require(pr_snapshot_store_create(std::numeric_limits<uint64_t>::max()) == nullptr, "unrepresentable budget refused");
        Store store(pr_snapshot_store_create(BYTES));
        require(store && pr_snapshot_store_used(store.get()) == 0, "real empty store");
        group(current_group, cutoffs);
        const auto expected = original_bytes();
        current_group = "shared_memfd_producer_immutable_after_storage_retired";
        auto original = producer_copy(store, cutoffs, expected);
        group(current_group, cutoffs);
        current_group = "explicit_clone_transferred_thread_outlives_original_and_store";
        transferred_clone(store, original, expected, cutoffs);
        group(current_group, cutoffs);
        current_group = "clone_quota_once_capacity_refusal_and_final_release";
        require(now() < cutoffs.work, "quota group original work cutoff");
        quota(expected);
        group(current_group, cutoffs);
        current_group = "invalid_shape_length_identity_reserved_no_partial_output";
        require(now() < cutoffs.work, "invalid group original work cutoff");
        invalid_admissions(expected);
        require(now() < cutoffs.close, "all groups completed before original closure cutoff");
        group(current_group, cutoffs);
        return 0;
    } catch (const std::exception& error) {
        // All reasons above are fixed strings, with no pointer, pixel or private-data output.
        std::cout << "{\"group\":\"" << current_group << "\",\"passed\":false,\"error\":\"" << error.what() << "\"}\n";
        return 1;
    }
}
