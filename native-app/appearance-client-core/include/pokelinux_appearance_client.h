#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* App-local appearance policy only; not action/session authority. Exactly one
 * existing Qt event-loop caller; no worker/thread/queue. No retained caller pointer,
 * call/free overlap, injected clock, provider lease, native fd or credentials.
 * Native edge owns socket/notifiers, SO_PEERCRED/SCM_CREDENTIALS, closes every
 * SCM_RIGHTS fd, rejects truncation/ancillary surprises and projects QColor/QML.
 * Every struct is copied. Unused/reserved fields and packet padding are zero. */
typedef struct PrAppearanceClient PrAppearanceClient;
typedef struct { uint16_t r,g,b,a; } PrAppearanceRgba16V1;
typedef struct {
 uint64_t connection_generation,request_token,original_until_boottime_ns;
 uint64_t palette_until_boottime_ns,change_sequence;
 uint32_t stage,valid,has_snapshot,poll_interval_ms;
 uint32_t request_timer_ms,expiry_timer_ms,read_enabled,write_enabled;
 uint8_t owner[16];
 uint32_t revision,base_size;
 double font_scale;
 /* background,foreground,surface,border,accent,muted,urgent,positive,ink */
 PrAppearanceRgba16V1 palette[9];
 /* caption,bodySmall,body,subtitle,title,heading,display,displayLarge */
 uint32_t font_tokens[8];
 /* controlGap,panelPadding,panelGap,rowGap,controlPaddingX,controlPaddingY */
 uint32_t spacing_tokens[6];
} PrAppearanceViewV1;
typedef struct {
 uint64_t connection_generation,request_token,effect_token;
 uint64_t original_until_boottime_ns;
 uint32_t action,packet_length;
 uint8_t packet[32];
} PrAppearanceStepV1;
enum { PR_APPEARANCE_OK=0,PR_APPEARANCE_EMPTY=1,PR_APPEARANCE_WAIT=2,
 PR_APPEARANCE_REFUSED=3,PR_APPEARANCE_EXPIRED=4,PR_APPEARANCE_INVALID=5,
 PR_APPEARANCE_STALE=6 };
enum { PR_APPEARANCE_IDLE=0,PR_APPEARANCE_CONNECTING=1,
 PR_APPEARANCE_SENDING=2,PR_APPEARANCE_RECEIVING=3,
 PR_APPEARANCE_CLOSING=4,PR_APPEARANCE_STOPPED=5 };
enum { PR_APPEARANCE_CONNECT=1,PR_APPEARANCE_FINISH_CONNECT=2,
 PR_APPEARANCE_SEND=3,PR_APPEARANCE_RECEIVE=4,PR_APPEARANCE_CLOSE=5 };
enum { PR_APPEARANCE_READ_READY=1,PR_APPEARANCE_WRITE_READY=2 };
enum { PR_APPEARANCE_CONNECTED=1,PR_APPEARANCE_CONNECT_PENDING=2,
 PR_APPEARANCE_CONNECT_FAILED_CLOSED=3,PR_APPEARANCE_SENT=4,
 PR_APPEARANCE_WOULD_BLOCK=5,PR_APPEARANCE_INTERRUPTED=6,
 PR_APPEARANCE_IO_FAILED=7,PR_APPEARANCE_CLOSED=8 };
/* out must point to null. New state is idle, invalid, no snapshot/deadlines;
 * poll_interval_ms250. Constructor never needs a successful clock sample. */
int32_t pr_appearance_client_new_v1(PrAppearanceClient **out);
/* Caller first retires its own actual socket/notifiers. No native ownership or
 * thread is inside this handle and destruction never blocks or fabricates close. */
void pr_appearance_client_free_v1(PrAppearanceClient *client);
/* Qt preserves its existing periodic250ms timer (start once, not reset on replies)
 * and initial zero-delay refresh. Idle refresh opens one original500ms exchange;
 * active refresh returns WAIT without renewing. Cached expiry is independent. */
int32_t pr_appearance_client_refresh_v1(PrAppearanceClient *client);
/* Native notifier callback forwards the immutable Rust generation captured when
 * its actual fd/notifier was installed. Core rejects old/wrong readiness before
 * it can select a current-channel operation. One accepted READ or WRITE delivery
 * is consumed once; no stored permission, event queue or C++ generation check.
 * READ observes current enabled socket EOF/unsolicited packets even while idle;
 * only a receiving exchange can admit payload publication. Wrong-kind readiness
 * returns REFUSED; stale generation returns STALE without changing current work. */
int32_t pr_appearance_client_ready_v1(PrAppearanceClient *client,
 uint64_t connection_generation,uint32_t event);
/* Real BOOTTIME admission immediately before native effects. Exactly one issued
 * plan: repeated step returns WAIT without writing out until report/received.
 * EMPTY/WAIT/EXPIRED/INVALID/STALE do not write an output. Counter increments are
 * checked; no wrap/reuse. CLOSE/Stop remains native-edge responsibility on refusal.
 * Initial CONNECT and immediate SEND after CONNECTED/idle refresh can be issued
 * directly. FINISH_CONNECT needs actual WRITE readiness; RECEIVE needs actual
 * READ readiness. WOULD_BLOCK/INTERRUPTED wait for new actual notifier delivery;
 * a repeated step cannot spin native recvmsg/connect/send calls.
 * Request is exactly {"op":"snapshot"}; 17 bytes, no trailing NUL. */
int32_t pr_appearance_client_step_v1(PrAppearanceClient *client,PrAppearanceStepV1 *out);
/* Echo entire exact plan. CONNECT_PENDING is only initial connect/EINPROGRESS;
 * FINISH_CONNECT uses actual SO_ERROR and same root SO_PEERCRED before CONNECTED.
 * SENT requires exact packet length. WOULD_BLOCK/INTERRUPTED use sent_bytes0,
 * consume only that effect token, keep request identity/deadline unchanged.
 * CONNECT_FAILED_CLOSED means actual native fd already closed. IO_FAILED requests
 * CLOSE. CLOSED requires actual fd/notifier retirement; never claims remote undo.
 * Old/duplicate/altered plans return STALE with no effect on a current channel. */
int32_t pr_appearance_client_report_v1(PrAppearanceClient *client,
 const PrAppearanceStepV1 *plan,uint32_t outcome,uint32_t sent_bytes);
/* Exactly one already authenticated 1..4096-byte SEQPACKET. Native edge has
 * checked peer PID/uid0/gid0 and packet exact credentials and closed/rejected rights.
 * Core checks real BOOTTIME before parsing and again immediately before commit.
 * Schema3 validation and normalized cache ownership are entirely Rust.
 * Malformed/refused: REFUSED; original RPC or published lease expired: EXPIRED.
 * Both close/invalidate but retain last accepted snapshot/owner/revision.
 * Stale plan is rejected without closing another connection. */
int32_t pr_appearance_client_received_v1(PrAppearanceClient *client,
 const PrAppearanceStepV1 *plan,const uint8_t *bytes,uint32_t length);
/* Actual timer delivery; samples BOOTTIME and updates independent expiry/request
 * decisions and timer delays (ceil ms, INT32_MAX clamp). Early delivery cannot
 * renew either original deadline. Clock failure closes/invalidate. */
int32_t pr_appearance_client_tick_v1(PrAppearanceClient *client);
/* Destructor/explicit shutdown only; disables future polling and requests CLOSE.
 * Late events cannot reconnect or republish. No new authority or Stop RPC. */
int32_t pr_appearance_client_stop_v1(PrAppearanceClient *client);
/* Pure copied observation. Timer values were computed at the latest policy call;
 * Qt converts these explicit values to QTimer delivery without a C++ deadline.
 * valid is cached policy state; it is not a fresh availability assertion. */
int32_t pr_appearance_client_view_v1(const PrAppearanceClient *client,PrAppearanceViewV1 *out);
/* Pure real-BOOTTIME getter. Writes0/1; false on clock failure or now>=lease.
 * Does not invalidate, renew, emit changes, dispatch or mutate any state. */
int32_t pr_appearance_client_available_v1(const PrAppearanceClient *client,uint32_t *out);
#ifdef __cplusplus
}
#endif
