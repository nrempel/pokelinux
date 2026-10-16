#ifndef POKELINUX_RENDER_H
#define POKELINUX_RENDER_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Original immutable CPU import payload. No graphics-completion or authority claim.
 * Store handles are control-owned; snapshots may be exclusively transferred or
 * cloned before transfer. Every successful clone is a separately owned handle.
 * Pointer arguments must address the advertised live objects/storage. */
typedef struct PrSnapshotStore PrSnapshotStore;
typedef struct PrSnapshot PrSnapshot;
typedef struct {
    uint64_t resource_id, generation, commit_sequence;
    uint32_t drm_format, width, height, stride, bytes_per_pixel, reserved;
} PrShmDescription;
typedef struct {
    PrShmDescription description;
    const uint8_t *pixels; /* borrow valid until this owned snapshot is released */
    uint64_t pixel_bytes;
} PrSnapshotView;
enum { PR_OK = 0, PR_INVALID = 1, PR_CAPACITY = 2, PR_ALLOCATION = 3 };
/* max_bytes is a bound on distinct live pixel allocations, including all clones.
 * Releasing the store leaves snapshots valid, with their budget still retained. */
PrSnapshotStore *pr_snapshot_store_create(uint64_t max_bytes);
void pr_snapshot_store_release(PrSnapshotStore *store);
uint64_t pr_snapshot_store_used(const PrSnapshotStore *store);
/* Result is set to null before admission. On error producer retains input.
 * Exactly stride*height bytes are copied before success; client storage is never
 * retained. No partial snapshot is published on validation/allocation failure. */
int32_t pr_snapshot_capture(const PrSnapshotStore *store,
                           const PrShmDescription *description,
                           const uint8_t *pixels, uint64_t pixel_bytes,
                           PrSnapshot **result);
PrSnapshot *pr_snapshot_clone(const PrSnapshot *snapshot);
int32_t pr_snapshot_view(const PrSnapshot *snapshot, PrSnapshotView *result);
void pr_snapshot_release(PrSnapshot *snapshot);
#ifdef __cplusplus
}
#endif
#endif
