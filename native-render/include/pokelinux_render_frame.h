#ifndef POKELINUX_RENDER_FRAME_H
#define POKELINUX_RENDER_FRAME_H
/* Immutable CPU frame value custody. No output or rendering authority is granted.
 * This header declares only implemented frame functions. */
#include "pokelinux_render.h"
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct PrFrame PrFrame;
#define PR_FRAME_ABI_V1 1u
#define PR_FRAME_MAX_DRAWS 256u
#define PR_FRAME_MAX_RESOURCES 256u
#define PR_FRAME_MAX_RECTS 4096u /* SUM damage + final damage + draw scissors */
#define PR_FRAME_MAX_METADATA_BYTES 1048576u

enum {
    PR_FRAME_OK = 0, PR_FRAME_INVALID = 1, PR_FRAME_CAPACITY = 2,
    PR_FRAME_ALLOCATION = 3
};
enum { PR_DRAW_CLEAR = 0, PR_DRAW_RECT = 1, PR_DRAW_SHM = 2 };
enum { PR_FRAME_COMMIT = 1u, PR_FRAME_FULL_REPAINT = 2u };
enum { PR_TARGET_SHADER_CM_AVAILABLE = 1u };
enum {
    PR_MAT_NEAREST = 1u, PR_MAT_BLEND = 2u, PR_MAT_RGBX = 4u,
    PR_MAT_TINT = 8u, PR_MAT_DISCARD_ACTIVE = 16u,
    PR_MAT_DISCARD_OPAQUE = 32u, PR_MAT_DISCARD_ALPHA = 64u
};
enum { PR_CM_ENABLED = 1u, PR_CM_TONEMAP = 2u, PR_CM_SDR_MOD = 4u };

typedef struct { int32_t x1, y1, x2, y2; } PrFrameRectV1;
typedef struct { double x, y, width, height, rotation; } PrFrameBoxV1;
typedef struct {
    uint64_t output_id, output_generation;
    uint32_t width, height, transformed_width, transformed_height;
    uint32_t output_transform, flags;
    double scale;
    float target_projection[9], scale_matrix[9];
} PrFrameTargetV1;
typedef struct {
    uint32_t flags, source_tf, target_tf, tonemap_mode;
    float src_tf_min, src_tf_max, dst_tf_min, dst_tf_max;
    float src_ref_luminance, dst_ref_luminance, max_luminance, dst_max_luminance;
    float convert_matrix[9], dst_primaries_xyz[9]; /* already column-major */
    float sdr_saturation, sdr_brightness;
} PrFrameCMV1;
typedef struct {
    float alpha, rounding_px, rounding_power, discard_opacity;
    uint32_t flags, wrap_x, wrap_y, reserved;
    float tint_rgb[3];
    PrFrameCMV1 cm;
} PrFrameMaterialV1;
typedef struct {
    uint32_t kind, resource_index, scissor_first, scissor_count;
    PrFrameBoxV1 box, rounding_box;
    float projection[9]; /* row-major; preserve final FramePlan71e convention */
    float uv[4], converted_color[4], color_srgb[4];
    PrFrameMaterialV1 material;
    uint64_t surface_id, commit_sequence; /* provenance / immutable import seq */
} PrFrameDrawV1;
typedef struct {
    PrShmDescription description;
    const PrSnapshot *snapshot; /* synchronous capture/native-call borrow; never release */
} PrFrameResourceBorrowV1;
typedef struct {
    uint32_t abi_version, struct_size, flags, reserved;
    uint64_t scene_sequence, security_generation;
    PrFrameTargetV1 target;
    uint32_t damage_first, damage_count, final_damage_first, final_damage_count;
    uint32_t draw_count, resource_count, rect_count, reserved2;
    const PrFrameDrawV1 *draws;
    const PrFrameResourceBorrowV1 *resources;
    const PrFrameRectV1 *rects;
} PrFrameBorrowV1;

/* Copies only bounded value arrays and clones Rust Snapshot/Arc VALUES. Input
 * handle slots and C++ plan remain owned by caller on success AND failure.
 * No pixel recopy, borrowed input pointer, C++ object or closure is retained. */
int32_t pr_frame_capture_v1(const PrFrameBorrowV1 *input, PrFrame **result);
void pr_frame_release(PrFrame *frame);

/* Synchronous visitor for an owned immutable frame.
 * visitor and stack_token are borrowed only for this immediate call and never
 * retained/queued. The view and every nested pointer expire before this function
 * returns. Do not release/transfer frame during visitor, retain pointers, or
 * throw/unwind. Clone a borrowed snapshot explicitly to obtain an owned handle.
 * The visitor's int32 status is returned unchanged; this mints no output authority. */
typedef int32_t (*PrFrameViewFnV1)(const PrFrameBorrowV1 *view, void *stack_token);
int32_t pr_frame_with_view_v1(const PrFrame *frame, PrFrameViewFnV1 visitor,
                              void *stack_token);

#ifdef __cplusplus
}
#endif
#endif
