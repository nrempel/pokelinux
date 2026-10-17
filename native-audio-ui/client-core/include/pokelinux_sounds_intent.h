#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Queue-only client state. The native edge retains all transport authority. */
typedef struct PrSoundsQueue PrSoundsQueue;
typedef struct { uint32_t length; uint8_t bytes[64]; } PrSoundsRouteV1;
typedef struct {
 uint32_t transport_online, status_complete, available, can_choose, can_reconnect, route_count;
 PrSoundsRouteV1 routes[32];
} PrSoundsFreshV1;
typedef struct {
 uint64_t token, original_until_boottime_ns;
 uint32_t kind, ttl_ms;
 PrSoundsRouteV1 route;
} PrSoundsCommandV1;
enum { PR_SOUNDS_ROUTE=1, PR_SOUNDS_RECONNECT=2 };
enum { PR_SOUNDS_OK=0, PR_SOUNDS_EMPTY=1, PR_SOUNDS_BUSY=2, PR_SOUNDS_REFUSED=3,
 PR_SOUNDS_EXPIRED=4, PR_SOUNDS_INVALID=5, PR_SOUNDS_STALE=6 };
/* Single Qt event-thread use; the Rust handle owns its one copied intent. */
int32_t pr_sounds_queue_new_v1(PrSoundsQueue **out);
void pr_sounds_queue_free_v1(PrSoundsQueue *queue);
int32_t pr_sounds_queue_offer_v1(PrSoundsQueue *queue, uint32_t kind, const PrSoundsRouteV1 *route);
/* Exactly 0/1; negative means invalid input. Read-only, never dispatches. */
int32_t pr_sounds_queue_pending_v1(const PrSoundsQueue *queue);
/* Called after native credential/JSON checks; status_complete=0 cannot dispatch.
   A ready result is a copied plan, never a send or permission grant. No mutation
   of *out on non-OK. Repeated preparation cannot renew the original deadline. */
int32_t pr_sounds_queue_prepare_v1(PrSoundsQueue *queue, const PrSoundsFreshV1 *fresh, PrSoundsCommandV1 *out);
/* Native edge reports only an actually completed send, not EAGAIN/EINTR. */
int32_t pr_sounds_queue_sent_v1(PrSoundsQueue *queue, uint64_t token);
void pr_sounds_queue_cancel_v1(PrSoundsQueue *queue);
#ifdef __cplusplus
}
#endif
