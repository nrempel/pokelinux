#ifndef POKELINUX_RENDER_EXECUTOR_H
#define POKELINUX_RENDER_EXECUTOR_H
#include "pokelinux_render_frame.h"
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* One current ABI, no legacy negotiation. All structs are copied values except
 * explicitly synchronous borrows. Every callback must catch exceptions inside
 * C++, never unwind, never reenter executor control, and obey native custody.
 * The supplier's code must remain loaded until destroy AND actual try_join.
 * Callback success is a trusted adapter report, not independent native evidence. */
#define PR_EXECUTOR_ABI_V1 1u
#define PR_EXECUTOR_MAX_OUTPUTS 32u
#define PR_EXECUTOR_MAX_SOURCES 16u
#define PR_EXECUTOR_MAX_OBJECTS 128u
#define PR_EXECUTOR_MAX_BOOTSTRAP_EVENTS 34u

typedef struct PrExecutor PrExecutor;
typedef struct PrBootstrap PrBootstrap;
typedef struct PrAttempt PrAttempt;
enum {
 PR_EXECUTOR_OK=0, PR_EXECUTOR_INVALID=1, PR_EXECUTOR_BUSY=2,
 PR_EXECUTOR_CAPACITY=3, PR_EXECUTOR_STOPPING=4, PR_EXECUTOR_STALE=5,
 PR_EXECUTOR_EXPIRED=6, PR_EXECUTOR_CLOCK=7, PR_EXECUTOR_EXHAUSTED=8,
 PR_EXECUTOR_NATIVE=9, PR_EXECUTOR_PROTOCOL=10, PR_EXECUTOR_WORKER_PANIC=11,
 PR_EXECUTOR_WAKE=12, PR_EXECUTOR_EMPTY=13
};
enum {
 PR_EXECUTOR_STARTING=0, PR_EXECUTOR_IDLE=1, PR_EXECUTOR_PREPARING=2,
 PR_EXECUTOR_EXECUTING=3, PR_EXECUTOR_RETIRING=4, PR_EXECUTOR_DESTROYING=5,
 PR_EXECUTOR_QUARANTINED=6, PR_EXECUTOR_EXITED=7, PR_EXECUTOR_BOOTSTRAPPING=8
};
typedef struct {
 uint32_t version, event_type;
 uint64_t session_sequence, session_generation, device_lease_id, device_generation, rdev;
 uint32_t active, vt, change_type, connector_id, prop_id, boot_vga, reserved[4];
 uint8_t seat_name[64], device_path[256];
} PrSessionRenderEventV1;
typedef struct {
 PrSessionRenderEventV1 metadata;
 int32_t primary_fd, render_fd; /* -1 absent; otherwise BORROWED */
} PrBootstrapEventV1;
typedef struct {
 uint64_t session_generation;
 uint32_t event_count, reserved;
 const PrBootstrapEventV1 *events; /* nested borrows expire on callback return */
} PrBootstrapViewV1;
typedef struct {
 uint64_t security_generation, output_id, output_generation, scene_sequence, session_generation;
} PrOperationPolicyV1;
typedef struct {
 uint64_t id;
 PrOperationPolicyV1 policy;
 uint64_t control_cookie;
} PrAdmissionV1;
typedef struct {
 uint32_t id, reserved;
 uint64_t generation, output_id, output_generation;
} PrNativeObjectV1;
typedef struct {
 PrOperationPolicyV1 policy;
 uint64_t device_lease_id, device_generation;
 uint32_t object_count, reserved;
 PrNativeObjectV1 objects[PR_EXECUTOR_MAX_OBJECTS]; /* unused tail must be zero */
} PrNativeScopeV1;
typedef struct {
 uint64_t serial;
 PrOperationPolicyV1 policy;
 uint64_t at_boottime_ns;
} PrGrantObservationV1;
enum { PR_NATIVE_NOT_ENTERED=0, PR_NATIVE_UNKNOWN=1, PR_NATIVE_RETURNED=2 };
/* NOT_ENTERED/UNKNOWN require all remaining fields zero. RETURNED requires
 * nonzero operation. Every boolean below is exactly 0/1; reserved fields zero. */
typedef struct {
 uint32_t kind, operation;
 int64_t value;
 int32_t error_number;
 uint32_t succeeded, extra_attempt_refused, reserved;
} PrNativeResultV1;
enum { PR_CLOCK_MONOTONIC=0, PR_CLOCK_REALTIME=1, PR_CLOCK_BOOTTIME=2 };
/* An absent presentation requires all four presentation value fields zero.
 * Present raw sequence/timestamp zero values remain valid in the explicit clock. */
typedef struct {
 uint64_t fence_identity; /* zero absent */
 uint64_t presentation_correlation_id, presentation_sequence, presentation_timestamp_ns;
 uint32_t presentation_clock, has_presentation, resources_retired, reserved;
} PrNativeProgressV1;
typedef struct {
 PrAdmissionV1 admission;
 PrNativeResultV1 native;
 uint32_t has_grant, has_scope;
 PrGrantObservationV1 grant;
 PrNativeScopeV1 scope;
} PrRetirementContextV1;
typedef struct {
 int32_t fd;
 uint32_t reserved;
 uint64_t token, generation;
 int16_t events;
 uint16_t reserved2[3];
} PrPollSourceV1;
typedef struct { PrPollSourceV1 source; int16_t revents; uint16_t reserved[3]; } PrNativeReadyV1;
typedef struct {
 uint64_t output_id, output_generation, device_lease_id, device_generation;
 uint64_t mode_id, mode_generation, modifier;
 uint32_t connector_id, width, height, refresh_millihz, drm_format, flags;
} PrDiscoveredOutputV1;
typedef struct {
 uint64_t session_generation;
 uint32_t output_count, reserved;
 PrDiscoveredOutputV1 outputs[PR_EXECUTOR_MAX_OUTPUTS];
} PrDiscoveryV1;
typedef struct {
 uint32_t abi_version, struct_size;
 uint64_t owner_id; /* nonzero scalar correlation only; no pointer encoding */
 uint64_t reserved[4];
} PrOwnerOptionsV1;
typedef struct {
 uint32_t abi_version, struct_size;
 uint64_t reserved[4];
 /* create is bounded CPU-only: allocate EMPTY holder, no native acquisition.
  * The returned pointer is immediately retained on the Rust worker before
  * bootstrap. No userdata/control/native objects cross at construction. */
 void *(*create)(const PrOwnerOptionsV1 *options);
 /* Only after verified per-entry AND aggregate native retirement, on worker. */
 void (*destroy)(void *owner);
 /* Borrowed FD duplicates stay owned by Rust adapter through verified shutdown.
  * Duplicate explicitly before passing to an API that CONSUMES an FD. Retain
  * native descendants in owner before fallible work, including on failure. */
 int32_t (*bootstrap)(void *owner, const PrBootstrapViewV1 *input, PrDiscoveryV1 *result);
 int32_t (*prepare)(void *owner, const PrFrame *frame, const PrAdmissionV1 *admission);
 /* attempt expires on return; authorize only synchronously at FINAL native
  * display attempt. Never retain token/frame/view pointers or fabricate scope. */
 int32_t (*execute)(void *owner, const PrFrame *frame, const PrAdmissionV1 *admission,
                    PrAttempt *attempt, PrNativeResultV1 *result);
 int32_t (*advance)(void *owner, const PrRetirementContextV1 *context, PrNativeProgressV1 *result);
 int32_t (*poll_sources)(void *owner, PrPollSourceV1 *storage, uint32_t capacity, uint32_t *count);
 int32_t (*dispatch)(void *owner, const PrNativeReadyV1 *ready);
 int32_t (*shutdown_progress)(void *owner, uint32_t *resources_retired);
} PrNativeOwnerFunctionsV1;
typedef struct {
 uint32_t stage, worker_alive, stopping, ready, failed;
 uint32_t occupied_credits, reserved_completion_records, queued_frames;
 uint64_t accepted, retired;
 uint32_t joined, reserved;
} PrExecutorSnapshotV1;
enum { PR_COMPLETION_ATTEMPT=1, PR_COMPLETION_OUTCOME=2, PR_COMPLETION_FENCE=3,
       PR_COMPLETION_PRESENTED=4, PR_COMPLETION_RETIRED=5 };
typedef struct {
 uint32_t kind, resources_retired;
 int32_t error;
 uint32_t has_grant, has_scope, reserved;
 PrAdmissionV1 admission;
 PrNativeResultV1 native;
 PrGrantObservationV1 grant;
 PrNativeScopeV1 scope;
 PrNativeProgressV1 facts;
} PrCompletionV1;

/* Capture copies all metadata and DUPLICATES each borrowed FD CLOEXEC. Original
 * descriptors stay caller-owned on every path. result must be an empty slot. */
int32_t pr_bootstrap_capture_v1(uint64_t session_generation, const PrBootstrapEventV1 *events,
                              uint32_t count, PrBootstrap **result);
void pr_bootstrap_release(PrBootstrap *input);
/* Validate pointers, empty output, table/version/options BEFORE consuming input.
 * After that validation input is consumed/null even if startup/spawn fails.
 * Copies the function table/options by VALUE; retains no borrowed table pointer.
 * All executor calls are creating-control-thread-only, serialized/nonreentrant.
 * Wrong-thread calls refuse BEFORE inner-state access, ownership changes or
 * output writes. Release must first read its outer handle slot for lookup.
 * All pointed objects must be live/aligned, valid for the call, nonaliasing. */
int32_t pr_executor_create_v1(const PrNativeOwnerFunctionsV1 *functions,
                            const PrOwnerOptionsV1 *options, uint64_t security_generation,
                            PrBootstrap **input, uint64_t startup_not_after_boottime_ns,
                            PrExecutor **result);
/* Both FDs are borrowed. Register BOTH before waiting; on startup readability
 * call snapshot/discovery/submit/join to observe original BOOTTIME expiry.
 * Unregister BOTH before release. Timer includes suspend; no wake guarantee. */
int32_t pr_executor_watch_fds(PrExecutor *owner, int32_t *completion_fd, int32_t *startup_fd);
int32_t pr_executor_snapshot_v1(PrExecutor *owner, PrExecutorSnapshotV1 *result);
int32_t pr_executor_discovery_v1(PrExecutor *owner, PrDiscoveryV1 *result);
/* Consumes/nulls frame ONLY on accepted admission. EVERY refusal leaves caller
 * frame handle unchanged. Ordinary refusals preserve earlier authority; terminal
 * serial exhaustion fail-closes it. result written on success only. */
int32_t pr_executor_try_submit_v1(PrExecutor *owner, PrFrame **frame, const PrOperationPolicyV1 *policy,
                                uint64_t not_after_boottime_ns, uint64_t control_cookie,
                                PrAdmissionV1 *result);
int32_t pr_executor_invalidate(PrExecutor *owner, uint64_t security_generation);
int32_t pr_executor_stop(PrExecutor *owner);
int32_t pr_executor_next_completion_v1(PrExecutor *owner, PrCompletionV1 *result);
/* Nonblocking; Busy retains all ownership while native work is live. */
int32_t pr_executor_try_join(PrExecutor *owner);
/* Null is already released. Busy retains slot until actual try_join. Never
 * detaches or invokes blocking Drop on an alive/unjoined worker. */
int32_t pr_executor_try_release(PrExecutor **owner);
/* Synchronous worker-only token, consumed once including invalid scope. Copy of
 * grant is historical observation, never reuse authority. Complete actual
 * old/new routing must be authenticated by native backend, not these IDs alone. */
int32_t pr_attempt_authorize_v1(PrAttempt *attempt, const PrNativeScopeV1 *scope,
                              PrGrantObservationV1 *result);
#ifdef __cplusplus
}
#endif
#endif
