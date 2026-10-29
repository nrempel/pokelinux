#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* App-local state only. No socket, credentials, provider lease or authority.
 * Single caller thread, no call/free overlap; no caller pointer is retained.
 * Actual native socket and credential checks precede connected/reply calls. */
typedef struct PrSoundsClient PrSoundsClient;
typedef struct { uint32_t length; uint8_t bytes[64]; } PrSoundsClientIdV1;
typedef struct { uint32_t length; uint8_t bytes[96]; } PrSoundsClientLabelV1;
typedef struct { PrSoundsClientIdV1 id; PrSoundsClientLabelV1 label; } PrSoundsClientRowV1;
typedef struct {
 uint64_t connection_epoch, request_until_boottime_ns, queued_until_boottime_ns;
 uint32_t stage, request_kind, status, online, available, can_choose, can_reconnect;
 uint32_t route_count, requested_present, applied_present, queued_kind, reserved;
 PrSoundsClientIdV1 requested, applied;
 PrSoundsClientRowV1 routes[32];
 uint32_t policy_settled, preview, can_play, can_stop;
 uint32_t selected_level_percent, selected_muted, can_adjust_preview, preview_reserved;
 uint64_t preview_generation;
 uint32_t playback_muted, playback_mute_known, can_toggle_mute;
 uint32_t mute_checked, mute_is_live, mute_reserved;
} PrSoundsClientViewV1;
typedef struct {
 uint64_t connection_epoch, token, original_until_boottime_ns;
 uint32_t length, reserved;
 uint8_t bytes[4096];
} PrSoundsClientPacketV1;
enum { PR_SOUNDS_CLIENT_OK=0, PR_SOUNDS_CLIENT_EMPTY=1, PR_SOUNDS_CLIENT_BUSY=2,
 PR_SOUNDS_CLIENT_REFUSED=3, PR_SOUNDS_CLIENT_EXPIRED=4,
 PR_SOUNDS_CLIENT_INVALID=5, PR_SOUNDS_CLIENT_STALE=6 };
enum { PR_SOUNDS_CLIENT_CLOSED=0, PR_SOUNDS_CLIENT_CONNECTING=1,
 PR_SOUNDS_CLIENT_SENDING=2, PR_SOUNDS_CLIENT_RECEIVING=3, PR_SOUNDS_CLIENT_IDLE=4 };
enum { PR_SOUNDS_CLIENT_NO_REQUEST=0, PR_SOUNDS_CLIENT_CONNECT=1,
 PR_SOUNDS_CLIENT_STATUS=2, PR_SOUNDS_CLIENT_ROUTE=3, PR_SOUNDS_CLIENT_RECONNECT=4,
 PR_SOUNDS_CLIENT_PREVIEW_PLAY=5, PR_SOUNDS_CLIENT_PREVIEW_STOP=6,
 PR_SOUNDS_CLIENT_PREVIEW_MUTE=7 };
enum { PR_SOUNDS_CLIENT_NO_INTENT=0, PR_SOUNDS_CLIENT_ROUTE_INTENT=1,
 PR_SOUNDS_CLIENT_RECONNECT_INTENT=2, PR_SOUNDS_CLIENT_STOP_INTENT=3,
 PR_SOUNDS_CLIENT_PLAY_INTENT=4 };
enum { PR_SOUNDS_CLIENT_SEND_COMPLETE=1, PR_SOUNDS_CLIENT_SEND_WOULD_BLOCK=2,
 PR_SOUNDS_CLIENT_SEND_INTERRUPTED=3, PR_SOUNDS_CLIENT_SEND_FAILED=4 };
/* Logical model/presentation codes, mapped by Qt to the existing exact text. */
enum { PR_SOUNDS_CLIENT_DISCONNECTED=0, PR_SOUNDS_CLIENT_CONNECTING_AUDIO=1,
 PR_SOUNDS_CLIENT_UNAVAILABLE_INSPECT=2, PR_SOUNDS_CLIENT_CHECKING=3,
 PR_SOUNDS_CLIENT_APPLYING=4, PR_SOUNDS_CLIENT_APPLIED=5,
 PR_SOUNDS_CLIENT_INTERRUPTED=6, PR_SOUNDS_CLIENT_CHOOSE_OUTPUT=7,
 PR_SOUNDS_CLIENT_ROUTE_UNAVAILABLE=8, PR_SOUNDS_CLIENT_OUTPUT_REQUESTED=9,
 PR_SOUNDS_CLIENT_DENIED=10, PR_SOUNDS_CLIENT_UNAVAILABLE=11,
 PR_SOUNDS_CLIENT_STATE_UNAVAILABLE=12, PR_SOUNDS_CLIENT_PATH_OR_CLOCK_REFUSED=13,
 PR_SOUNDS_CLIENT_CONNECTION_REFUSED=14, PR_SOUNDS_CLIENT_CONNECTION_EXPIRED=15,
 PR_SOUNDS_CLIENT_ACTOR_UNAVAILABLE=16, PR_SOUNDS_CLIENT_IDENTITY_REFUSED=17,
 PR_SOUNDS_CLIENT_REQUEST_EXPIRED_INSPECT=18, PR_SOUNDS_CLIENT_REQUEST_EXPIRED=19,
 PR_SOUNDS_CLIENT_CLOCK_UNAVAILABLE=20, PR_SOUNDS_CLIENT_REQUEST_REFUSED=21,
 PR_SOUNDS_CLIENT_QUEUED_REFUSED=22, PR_SOUNDS_CLIENT_SEND_REFUSED=23,
 PR_SOUNDS_CLIENT_REPLY_OUTSIDE_REQUEST=24, PR_SOUNDS_CLIENT_REPLY_EXPIRED=25,
 PR_SOUNDS_CLIENT_PACKET_REFUSED=26, PR_SOUNDS_CLIENT_REPLY_REFUSED=27,
 PR_SOUNDS_CLIENT_SELECTED_IDLE=28, PR_SOUNDS_CLIENT_PREVIEW_STARTING=29,
 PR_SOUNDS_CLIENT_PREVIEW_PLAYING=30, PR_SOUNDS_CLIENT_PREVIEW_RETIRING=31,
 PR_SOUNDS_CLIENT_PREVIEW_COMPLETED=32, PR_SOUNDS_CLIENT_PREVIEW_FAILED=33,
 PR_SOUNDS_CLIENT_PREVIEW_CANCELLED=34, PR_SOUNDS_CLIENT_PREVIEW_MUTE_PENDING=35 };
enum { PR_SOUNDS_CLIENT_PREVIEW_IDLE_STATE=0, PR_SOUNDS_CLIENT_PREVIEW_STARTING_STATE=1,
 PR_SOUNDS_CLIENT_PREVIEW_PLAYING_STATE=2, PR_SOUNDS_CLIENT_PREVIEW_RETIRING_STATE=3,
 PR_SOUNDS_CLIENT_PREVIEW_COMPLETED_STATE=4, PR_SOUNDS_CLIENT_PREVIEW_FAILED_STATE=5 };
/* Output pointee must initially be null. Non-OK never writes an output. */
int32_t pr_sounds_client_new_v1(PrSoundsClient **out);
void pr_sounds_client_free_v1(PrSoundsClient *client);
/* Initial connect only. Later offline connect is an explicit reconnect action. */
int32_t pr_sounds_client_start_v1(PrSoundsClient *client);
int32_t pr_sounds_client_connected_v1(PrSoundsClient *client, uint64_t epoch);
int32_t pr_sounds_client_choose_v1(PrSoundsClient *client, const PrSoundsClientIdV1 *route);
int32_t pr_sounds_client_reconnect_v1(PrSoundsClient *client);
/* Atomic local next-preview choice: percent 0..100 and muted 0/1. Defaults 100/0.
 * Invalid pairs never partially update. Active preview, pending effects or queued
 * intents return BUSY; a read-only status may continue under its original deadline.
 * Choices are available offline and during read-only status; a pending connection
 * returns BUSY. Selected values survive connection changes for this client lifetime;
 * a new client starts 100/0.
 * No socket operation, effect intent or request/token/deadline mutation. */
int32_t pr_sounds_client_preview_level_v1(PrSoundsClient *client, uint32_t percent, uint32_t muted);
/* One explicit mute button choice. Idle changes only the local next-preview
 * choice. Playing freezes this connection's observed generation, inverse of its
 * confirmed feed policy and original one-second deadline. It never queues behind
 * status/effects or changes next-preview choices while Playing. Confirmation is
 * digital feeder policy, not immediate sink silence. Stop while this effect is
 * in flight closes local channel state and must close the native socket. */
int32_t pr_sounds_client_preview_toggle_mute_v1(PrSoundsClient *client);
/* Explicit Play with a settled selected logical route may wait behind one
 * read-only status. Its final complete state must still authorize the same route
 * before the first Play attempt. The route, selected_level_percent, selected_muted
 * and original one-second BOOTTIME deadline are frozen at the explicit choice.
 * Each prepare only reduces ttl_ms; no completion, failure or reconnect replays
 * Play. Stop cancels a waiting Play by closing local channel state. */
int32_t pr_sounds_client_preview_play_v1(PrSoundsClient *client);
/* Cleanup Stop may wait behind one read-only status under its original deadline.
 * Stop during an in-flight Play closes local channel state: native must close
 * the socket to withdraw ownership; this does not assert remote quiet/reap. */
int32_t pr_sounds_client_preview_stop_v1(PrSoundsClient *client);
/* Idle read-only poll only; never interrupts or extends an active request. */
int32_t pr_sounds_client_refresh_v1(PrSoundsClient *client);
/* Copied packet only, no I/O. Reprepare cannot renew token/deadline. */
int32_t pr_sounds_client_prepare_v1(PrSoundsClient *client, PrSoundsClientPacketV1 *out);
/* Reports one actual send result for the latest prepared packet. A complete
 * send must equal its length; partial/error closes without claiming remote undo.
 * WOULD_BLOCK/INTERRUPTED require sent_bytes=0 and do not consume the token. */
int32_t pr_sounds_client_sent_v1(PrSoundsClient *client, uint64_t epoch,
 uint64_t token, uint32_t outcome, uint32_t sent_bytes);
/* Only exact bytes from one already authenticated <=4096-byte SEQPACKET. */
int32_t pr_sounds_client_reply_v1(PrSoundsClient *client, uint64_t epoch,
 const uint8_t *bytes, uint32_t length);
/* Native edge supplies one of the explicitly enumerated failure codes13..27.
 * Old epochs never close the current channel. No arbitrary text/status setter. */
int32_t pr_sounds_client_disconnected_v1(PrSoundsClient *client, uint64_t epoch, uint32_t reason);
/* Explicit watch delivery; no synthetic time. All effect/reply paths also check
 * real BOOTTIME before dispatch, including incomplete-status expiry. */
int32_t pr_sounds_client_tick_v1(PrSoundsClient *client);
/* Pure copied observation: no clock transition, send, permission or renewal. */
int32_t pr_sounds_client_view_v1(const PrSoundsClient *client, PrSoundsClientViewV1 *out);
#ifdef __cplusplus
}
#endif
