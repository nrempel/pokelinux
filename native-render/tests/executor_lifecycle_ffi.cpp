// Actual C++ -> retained Rust worker ABI. Authored owner, no AQ/GL/hardware.
#include "pokelinux_render_executor.h"
#include <cstddef>
#define ABI_SIZE(type, size) static_assert(sizeof(type) == size)
ABI_SIZE(PrSessionRenderEventV1, 408);
ABI_SIZE(PrBootstrapEventV1, 416);
ABI_SIZE(PrBootstrapViewV1, 24);
ABI_SIZE(PrOperationPolicyV1, 40);
ABI_SIZE(PrAdmissionV1, 56);
ABI_SIZE(PrNativeObjectV1, 32);
ABI_SIZE(PrNativeScopeV1, 4160);
ABI_SIZE(PrGrantObservationV1, 56);
ABI_SIZE(PrNativeResultV1, 32);
ABI_SIZE(PrNativeProgressV1, 48);
ABI_SIZE(PrRetirementContextV1, 4312);
ABI_SIZE(PrPollSourceV1, 32);
ABI_SIZE(PrNativeReadyV1, 40);
ABI_SIZE(PrDiscoveredOutputV1, 80);
ABI_SIZE(PrDiscoveryV1, 2576);
ABI_SIZE(PrOwnerOptionsV1, 48);
ABI_SIZE(PrNativeOwnerFunctionsV1, 112);
ABI_SIZE(PrExecutorSnapshotV1, 56);
ABI_SIZE(PrCompletionV1, 4376);
static_assert(offsetof(PrCompletionV1, admission) == 24);
static_assert(offsetof(PrCompletionV1, native) == 80);
static_assert(offsetof(PrCompletionV1, grant) == 112);
static_assert(offsetof(PrCompletionV1, scope) == 168);
static_assert(offsetof(PrCompletionV1, facts) == 4328);
#undef ABI_SIZE
#include <atomic>
#include <cstdio>
#include <cstring>
#include <new>
#include <thread>
#include <time.h>

static std::atomic<bool> bad{false}, created{false}, destroyed{false};
static const auto control_thread = std::this_thread::get_id();
struct Owner { std::thread::id thread; };
static uint64_t now() {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0) return 0;
    return uint64_t(ts.tv_sec) * 1000000000ULL + uint64_t(ts.tv_nsec);
}
static bool worker(Owner *p) noexcept {
    const bool ok = p && p->thread == std::this_thread::get_id() && p->thread != control_thread;
    if (!ok) bad = true;
    return ok;
}
static void *create(const PrOwnerOptionsV1 *options) noexcept {
    if (!options || options->owner_id != 42 || std::this_thread::get_id() == control_thread) {
        bad = true; return nullptr;
    }
    created = true;
    return new (std::nothrow) Owner{std::this_thread::get_id()};
}
static void destroy(void *p) noexcept {
    auto *o = static_cast<Owner*>(p);
    if (!worker(o)) return;
    delete o; destroyed = true;
}
static int32_t bootstrap(void *p, const PrBootstrapViewV1 *input, PrDiscoveryV1 *out) noexcept {
    if (!worker(static_cast<Owner*>(p)) || !input || !out || input->event_count != 2 ||
        input->session_generation != 29 || input->events[0].metadata.event_type != 1 ||
        input->events[1].metadata.event_type != 3) return PR_EXECUTOR_NATIVE;
    *out = {}; out->session_generation = 29; return PR_EXECUTOR_OK;
}
static int32_t prepare(void*, const PrFrame*, const PrAdmissionV1*) noexcept { return PR_EXECUTOR_INVALID; }
static int32_t execute(void*, const PrFrame*, const PrAdmissionV1*, PrAttempt*, PrNativeResultV1*) noexcept { return PR_EXECUTOR_INVALID; }
static int32_t advance(void*, const PrRetirementContextV1*, PrNativeProgressV1*) noexcept { return PR_EXECUTOR_INVALID; }
static int32_t sources(void *p, PrPollSourceV1*, uint32_t capacity, uint32_t *count) noexcept {
    if (!worker(static_cast<Owner*>(p)) || capacity != PR_EXECUTOR_MAX_SOURCES || !count) return PR_EXECUTOR_NATIVE;
    *count = 0; return PR_EXECUTOR_OK;
}
static int32_t dispatch(void*, const PrNativeReadyV1*) noexcept { return PR_EXECUTOR_INVALID; }
static int32_t shutdown_progress(void *p, uint32_t *retired) noexcept {
    if (!worker(static_cast<Owner*>(p)) || !retired) return PR_EXECUTOR_NATIVE;
    *retired = 1; return PR_EXECUTOR_OK;
}
static void *poison_create(const PrOwnerOptionsV1*) noexcept { bad = true; return nullptr; }
static void poison_destroy(void*) noexcept { bad = true; }
#define CHECK(x) do { if (!(x)) { std::fprintf(stderr, "FAIL lifecycle line %d: %s\n", __LINE__, #x); return 1; } } while (false)
int main() {
    PrBootstrapEventV1 events[2]{};
    for (unsigned i=0; i<2; ++i) {
        events[i].metadata.version=1; events[i].metadata.event_type=i==0 ? 1 : 3;
        events[i].metadata.session_sequence=i+1; events[i].metadata.session_generation=29;
        events[i].primary_fd=-1; events[i].render_fd=-1;
    }
    std::memcpy(events[0].metadata.seat_name, "seat0", 6);
    PrBootstrap *input=nullptr;
    CHECK(pr_bootstrap_capture_v1(29, events, 2, &input)==PR_EXECUTOR_OK && input);
    PrNativeOwnerFunctionsV1 functions{};
    functions.abi_version=1; functions.struct_size=sizeof(functions);
    functions.create=create; functions.destroy=destroy; functions.bootstrap=bootstrap;
    functions.prepare=prepare; functions.execute=execute; functions.advance=advance;
    functions.poll_sources=sources; functions.dispatch=dispatch; functions.shutdown_progress=shutdown_progress;
    PrOwnerOptionsV1 options{}; options.abi_version=1; options.struct_size=sizeof(options); options.owner_id=42;
    PrExecutor *owner=nullptr;
    const uint64_t until=now()+2000000000ULL;
    CHECK(pr_executor_create_v1(&functions, &options, 1, &input, until, &owner)==PR_EXECUTOR_OK);
    CHECK(!input && owner);
    functions.create=poison_create; functions.destroy=poison_destroy; options.owner_id=999;
    PrExecutor *original=owner;
    CHECK(pr_executor_try_release(&owner)==PR_EXECUTOR_BUSY && owner==original);
    PrExecutorSnapshotV1 snapshot{};
    snapshot.accepted=0xabcdef;
    int32_t foreign=0;
    std::thread wrong([&] { foreign=pr_executor_snapshot_v1(owner, &snapshot); }); wrong.join();
    CHECK(foreign==PR_EXECUTOR_INVALID && snapshot.accepted==0xabcdef);
    int32_t completion=-1, startup=-1;
    CHECK(pr_executor_watch_fds(owner,&completion,&startup)==PR_EXECUTOR_OK && completion>=0 && startup>=0 && completion!=startup);
    PrDiscoveryV1 discovery{};
    int32_t status=PR_EXECUTOR_EMPTY;
    do { status=pr_executor_discovery_v1(owner,&discovery); std::this_thread::yield(); }
    while ((status==PR_EXECUTOR_EMPTY || status==PR_EXECUTOR_BUSY) && now()<until);
    CHECK(status==PR_EXECUTOR_OK && discovery.session_generation==29 && discovery.output_count==0);
    CHECK(pr_executor_snapshot_v1(owner,&snapshot)==PR_EXECUTOR_OK && snapshot.ready && !snapshot.stopping && !snapshot.joined);
    CHECK(pr_executor_stop(owner)==PR_EXECUTOR_OK);
    do { status=pr_executor_try_join(owner); std::this_thread::yield(); }
    while (status==PR_EXECUTOR_BUSY && now()<until);
    CHECK(status==PR_EXECUTOR_OK && created && destroyed && !bad);
    CHECK(pr_executor_snapshot_v1(owner,&snapshot)==PR_EXECUTOR_OK && snapshot.joined && !snapshot.worker_alive);
    // Borrowed watches were never registered; no registration remains to remove.
    CHECK(pr_executor_try_release(&owner)==PR_EXECUTOR_OK && !owner);
    CHECK(pr_executor_try_release(&owner)==PR_EXECUTOR_OK);
    pr_bootstrap_release(input);
    std::puts("EXECUTOR_LIFECYCLE_FFI_PASS");
    std::puts("EXECUTOR_FFI_ALL_PASS");
}
