#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Source-only full Messages engine proposal, freeze2. App-local policy only.
 * Qt owns QObject/QML, one QThread, actual socket/credential/SCM_RIGHTS handling,
 * queued invoke delivery and actual QThread::wait. This core owns all decisions.
 * One UI caller and one entered worker may call concurrently. Calls serialize
 * internally. No call/free overlap, retained caller pointer, clock injection,
 * caller-supplied identity/permission, provider lease or service authority.
 * All structures are copied. Reserved fields must be zero. */
typedef struct PrMessagesClient PrMessagesClient;
typedef struct { uint32_t length; uint8_t bytes[32]; } PrMessagesIdV1;
typedef struct {
 uint64_t generation, connection_epoch, request_token, event_token;
 uint64_t original_until_boottime_ns, wake_boottime_ns;
 uint32_t action, packet_length, wait_slice_ms, reserved;
 uint8_t packet[4096];
} PrMessagesStepV1;
typedef struct {
 uint64_t generation, model_revision, connection_epoch, request_token;
 uint64_t original_until_boottime_ns, queued_events;
 uint32_t stage, status, stopping, worker_active;
 uint32_t conversations_length, messages_length, reserved0, reserved1;
 PrMessagesIdV1 selected, conversations_after;
 uint64_t after_ordinal;
 /* Validated original JSON array slices, no trailing NUL. Preserve unknown keys;
  * row validation is in Rust; Qt only converts copied JSON into QVariantList. */
 uint8_t conversations_json[4096], messages_json[4096];
} PrMessagesViewV1;
enum { PR_MESSAGES_OK=0, PR_MESSAGES_EMPTY=1, PR_MESSAGES_WAIT=2,
 PR_MESSAGES_REFUSED=3, PR_MESSAGES_EXPIRED=4, PR_MESSAGES_INVALID=5,
 PR_MESSAGES_STALE=6, PR_MESSAGES_ABORT=7 };
enum { PR_MESSAGES_CONNECT=1, PR_MESSAGES_SEND=2, PR_MESSAGES_RECEIVE=3,
 PR_MESSAGES_NOTIFY=4, PR_MESSAGES_SLEEP=5, PR_MESSAGES_CLOSE=6,
 PR_MESSAGES_EXIT=7 };
enum { PR_MESSAGES_NEW=0, PR_MESSAGES_CONNECTING=1, PR_MESSAGES_SENDING=2,
 PR_MESSAGES_RECEIVING=3, PR_MESSAGES_COOLDOWN=4, PR_MESSAGES_STOPPED=5 };
enum { PR_MESSAGES_UNAVAILABLE=0, PR_MESSAGES_HISTORY=1, PR_MESSAGES_DENIED=2 };
enum { PR_MESSAGES_CONNECTED=1, PR_MESSAGES_CONNECT_FAILED=2,
 PR_MESSAGES_SENT=3, PR_MESSAGES_WOULD_BLOCK=4, PR_MESSAGES_INTERRUPTED=5,
 PR_MESSAGES_IO_FAILED=6, PR_MESSAGES_CLOSED=7,
 PR_MESSAGES_NOTIFY_QUEUED=8, PR_MESSAGES_PATH_REFUSED=9 };
enum { PR_MESSAGES_SELECT=1, PR_MESSAGES_NEXT_MESSAGES=2,
 PR_MESSAGES_FIRST_MESSAGES=3, PR_MESSAGES_NEXT_CONVERSATIONS=4,
 PR_MESSAGES_FIRST_CONVERSATIONS=5 };
/* out must point to null; non-OK never writes output. New state has []/[],
 * unavailable, generation0, no worker/connection/request and no time budget. */
int32_t pr_messages_client_new_v1(PrMessagesClient **out);
/* Refuses until an entered worker exited AND native successful wait was reported.
 * A never-entered handle may be freed. No implicit blocking destructor/join. */
int32_t pr_messages_client_free_v1(PrMessagesClient *client);
/* Exactly one entry for the handle; entry starts initial conversation cycle. */
int32_t pr_messages_client_worker_enter_v1(PrMessagesClient *client);
/* Pure copied view: cannot dispatch, expire, renew or grant anything. */
int32_t pr_messages_client_view_v1(const PrMessagesClient *client, PrMessagesViewV1 *out);
/* SELECT takes an exact32-byte lowercase hex ID. Other actions require id=null.
 * Valid actions increment generation (even first-page on empty); next-page on an
 * empty corresponding list returns EMPTY with no change. Clear exactly the
 * current Qt lists; retain current status. No immediate request or cancellation. */
int32_t pr_messages_client_action_v1(PrMessagesClient *client, uint32_t action,
 const PrMessagesIdV1 *id);
/* One entered worker calls step/report/received/exited. step only produces a
 * copied plan. Repeated CONNECT/SEND/RECEIVE/SLEEP plans keep the same
 * token/deadline/wake. NOTIFY is one-shot: first step writes one plan; until
 * its acknowledgement, later step returns WAIT without writing output. Retain
 * its acknowledgement slot independently of the UI FIFO, even after delivery.
 * Stop cancels this wait so CLOSE/EXIT never waits for notification ACK.
 * Native nonblocking
 * edge acts on plan, polls no more than wait_slice_ms20, then reports actual
 * results. SLEEP has <=10ms slice, original retry/cycle wake never renewed.
 * Initial CONNECT has no clock admission check or service request deadline:
 * even a BOOTTIME EIO must allow the existing actual credential-bound connect
 * control to accept, then refuse SEND until a real successful clock sample.
 * CONNECT_FAILED acknowledges only an already-closed failed native socket.
 * Credentialed native CONNECTED starts
 * one1s BOOTTIME exchange; established channels start the next1s exchange after
 * the original250ms cooldown. Failed connects wait100ms before retry.
 * No operation is authorized by a copied packet. Issuing CONNECT creates a
 * native-close obligation even before CONNECTED is reported. Stop cannot yield
 * EXIT or accept worker_exited until that obligation receives CLOSED (or an
 * accepted already-closed CONNECT_FAILED before Stop). The
 * mechanical edge remains authoritative over its actual fd: after an already
 * entered native operation returns, close any fd before CLOSED or final EXIT,
 * including when its late CONNECTED/SENT report is REFUSED by stopped policy. */
int32_t pr_messages_client_step_v1(PrMessagesClient *client, PrMessagesStepV1 *out);
/* Echo the plan's connection_epoch/request_token/event_token exactly. Unused
 * token fields remain0. SENT requires exact packet length; partial send fails.
 * WOULD_BLOCK/INTERRUPTED consume nothing and cannot extend a deadline.
 * NOTIFY_QUEUED acknowledges the separately retained issued plan once, even
 * if GUI deliver already consumed its event as valid or stale. Generation
 * changes/FIFO removal cannot reject that ACK or renew the original cooldown.
 * Duplicate ACK is STALE. Stop makes late NOTIFY_QUEUED/CONNECTED/SENT REFUSED;
 * such refusal never relinquishes native fd custody or the close obligation.
 * CONNECTED is allowed only after real SO_PEERCRED uid0; packet receipt still
 * requires exact PID/uid0 SCM_CREDENTIALS, no truncation/ancillary surprises,
 * and closure of every received SCM_RIGHTS fd in the native edge. */
int32_t pr_messages_client_report_v1(PrMessagesClient *client,
 const PrMessagesStepV1 *plan, uint32_t outcome, uint32_t actual_sent_bytes);
/* One already-authenticated full <=4096-byte SEQPACKET, only in RECEIVE.
 * Check real BOOTTIME after receive, before retaining copied bytes. Empty/error,
 * clock failure and deadline equality enqueue unavailable then close the channel.
 * Valid timely bytes enqueue exactly one token without parsing on worker.
 * The existing FIFO is retained: copied payload/generation live in Rust; Qt's
 * queued lambda carries only event_token. No callback owns another request.
 * Every exchange, including failure, toggles conversation/messages cycle once.
 * Completion fixes wake=completion BOOTTIME+250ms, exposed in NOTIFY and SLEEP.
 * Delivery, ACK and CLOSE cannot renew wake; if they finish after it, the next
 * cycle starts without a new250ms wait. No repeated NOTIFY after ACK or delivery. */
int32_t pr_messages_client_received_v1(PrMessagesClient *client,
 const PrMessagesStepV1 *plan, const uint8_t *bytes, uint32_t length);
/* UI consumes exactly once, FIFO; any consumed/unknown/out-of-order token is
 * STALE without model changes. GUI delivery may precede the worker NOTIFY_QUEUED report; consume the event
 * without erasing its separate pending acknowledgement. A matching token from an old action generation
 * is consumed as STALE. No BOOTTIME reread here: current Qt accepts a timely
 * received queued reply even when GUI delivery is delayed. No authority claim.
 * Validation and model/auto-selection update are atomic. */
int32_t pr_messages_client_deliver_v1(PrMessagesClient *client, uint64_t event_token);
/* UI stop starts a single500ms BOOTTIME ownership budget; no replacement
 * deadline. Clock failure permits only the next native20ms wait attempt.
 * Worker must CLOSE then EXIT for any issued CONNECT or established fd.
 * Outstanding notification ACK never blocks this path; Stop removes queued
 * events and invalidates late ACK/delivery. Future actions/deliveries refuse.
 * Repeated Stop is idempotent and cannot recreate its original ownership budget. */
int32_t pr_messages_client_stop_v1(PrMessagesClient *client);
/* Only after actual native loop exit and descriptor close; still not a join.
 * Refuses before close obligations receive CLOSED or an accepted already-closed
 * CONNECT_FAILED before Stop.
 * Native fd must also be closed by the edge even if an earlier report refused. */
int32_t pr_messages_client_worker_exited_v1(PrMessagesClient *client);
/* UI reports actual QThread::wait(20ms) return, including TLS completion.
 * actual_completed is0/1 only. Successful actual wait+worker exit ->OK even
 * if that true return is observed after500ms; check actual completion first,
 * preserving the current Qt destructor. A false result within
 * original budget ->WAIT; deadline/clock uncertainty ->ABORT. false never
 * authorizes free even when the worker-exited flag is true. */
int32_t pr_messages_client_join_result_v1(PrMessagesClient *client, uint32_t actual_completed);
#ifdef __cplusplus
}
#endif
