#ifndef POKELINUX_RENDER_FORMAT_H
#define POKELINUX_RENDER_FORMAT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
#define PR_FORMAT_ABI_V1 1u
#define PR_FORMAT_MAX_NATIVE_FORMATS 128u
#define PR_FORMAT_MAX_MODIFIERS 4096u
#define PR_FORMAT_MAX_PLANES 128u
#define PR_FORMAT_MAX_LEGACY_FORMATS 4096u
#define PR_FORMAT_MAX_EXPLICIT_PAIRS 4096u
#define PR_FORMAT_MAX_CRTCS 32u
#define PR_FORMAT_MAX_ENCODERS 128u
#define PR_FORMAT_MOD_INVALID UINT64_C(0x00ffffffffffffff)
enum { PR_FORMAT_OK=0, PR_FORMAT_INVALID=1, PR_FORMAT_CAPACITY=2 };
enum { PR_FORMAT_READ_UNKNOWN=0, PR_FORMAT_READ_OK=1, PR_FORMAT_READ_ERROR=2 };
enum { PR_FORMAT_PROPERTY_UNKNOWN=0, PR_FORMAT_PROPERTY_ABSENT=1,
       PR_FORMAT_PROPERTY_ERROR=2, PR_FORMAT_PROPERTY_PRESENT=3 };
enum { PR_FORMAT_REFUSED=1, PR_FORMAT_CORROBORATED_EXPLICIT=2 };
enum { PR_FORMAT_REASON_NONE=0, PR_FORMAT_REASON_EMPTY_NATIVE=1,
       PR_FORMAT_REASON_EMPTY_FIRST_MODIFIERS=2, PR_FORMAT_REASON_IMPLICIT_UNQUALIFIED=3,
       PR_FORMAT_REASON_QUERY_UNKNOWN=4, PR_FORMAT_REASON_QUERY_ERROR=5,
       PR_FORMAT_REASON_PROPERTY_ABSENT=6, PR_FORMAT_REASON_CAP_UNSUPPORTED=7,
       PR_FORMAT_REASON_INCONSISTENT=8, PR_FORMAT_REASON_NO_ROUTE=9,
       PR_FORMAT_REASON_NO_EXPLICIT_MATCH=10 };
enum { PR_FORMAT_SOURCE_NONE=0, PR_FORMAT_SOURCE_NATIVE=1, PR_FORMAT_SOURCE_CAP=2,
       PR_FORMAT_SOURCE_ROUTE=3, PR_FORMAT_SOURCE_PLANE=4, PR_FORMAT_SOURCE_TYPE=5,
       PR_FORMAT_SOURCE_IN_FORMATS=6, PR_FORMAT_SOURCE_BLOB=7 };
/* Scalar correlation only. It does not authenticate this context as current or
 * replace the outer executor's epoch/availability gate. All IDs nonzero except
 * raw rdev may be zero; reserved is zero. No pointer or epoch is encoded. */
typedef struct {
 uint64_t session_generation, discovery_sequence, device_lease_id, device_generation;
 uint64_t rdev, output_id, output_generation;
 uint32_t connector_id, reserved;
} PrFormatContextV1;
/* Preserve ALL native groups in order, including an empty FIRST modifier list. */
typedef struct { uint32_t drm_format, modifier_first, modifier_count, reserved; } PrFormatNativeV1;
typedef struct { uint32_t drm_format, reserved; uint64_t modifier; } PrFormatPairV1;
typedef struct { uint32_t state; int32_t error_number; uint64_t value; } PrFormatPropertyV1;
/* cap raw value is meaningful only on an observed successful call. */
typedef struct {
 uint32_t state; int32_t return_value, error_number; uint32_t reserved;
 uint64_t value;
} PrFormatCapV1;
/* All fields are observations, never native-precomputed route/pair eligibility.
 * Spans index the whole flat legacy-format and decoded-pair arrays. blob_state
 * describes successful read AND complete bounded decoding, not property presence.
 * UNKNOWN/ERROR fields carry no invented payload values. */
typedef struct {
 uint32_t plane_id, current_crtc_id, possible_crtcs, query_state;
 int32_t query_errno;
 uint32_t legacy_first, legacy_count, pair_first, pair_count, blob_state;
 int32_t blob_errno; uint32_t blob_length;
 PrFormatPropertyV1 type, in_formats;
} PrFormatPlaneV1;
typedef struct {
 uint32_t abi_version, struct_size;
 PrFormatContextV1 context;
 PrFormatCapV1 cap;
 /* route state covers complete raw CRTC/connector-encoder mask collection.
  * Current encoder is separate: ABSENT is known encoder_id0, never query error. */
 uint32_t route_state; int32_t route_errno;
 uint32_t current_encoder_state; int32_t current_encoder_errno;
 uint32_t current_encoder_id, current_crtc_id;
 uint32_t native_format_count, modifier_count, plane_count, legacy_format_count;
 uint32_t explicit_pair_count, crtc_count, encoder_count, reserved_count;
 const PrFormatNativeV1 *native_formats;
 const uint64_t *modifiers;
 const PrFormatPlaneV1 *planes;
 const uint32_t *legacy_formats;
 const PrFormatPairV1 *explicit_pairs;
 const uint32_t *crtc_ids;
 const uint32_t *encoder_possible_crtcs;
 uint64_t reserved[2];
} PrFormatFactsV1;
/* Owned copied value only. A corroborated advertised explicit tuple is NOT
 * backend-operation eligibility, allocation/import permission or a frame grant.
 * Refusal writes no format/modifier choice. plane_id may identify its observed
 * refusal witness; it never claims AQ selected that plane. */
typedef struct {
 uint32_t abi_version, struct_size, decision, reason;
 PrFormatContextV1 context;
 uint32_t plane_id, drm_format, refusal_source, reserved;
 uint64_t modifier;
} PrFormatSelectionV1;
/* No allocation, retained pointer/state/FD/callback, executor lookup or native
 * operation. Synchronous stateless CPU policy. All nonzero-count arrays are
 * live/aligned read-only borrows for the call; zero count requires null. Output
 * is live/aligned and nonaliasing; it is NEVER read before writing. INVALID/CAPACITY leave
 * every output/input byte unchanged. OK overwrites one canonical whole choice OR
 * refusal (all reserved bytes zero), including an already initialized slot.
 * No input order changes: only FIRST format/FIRST modifier is selected. */
int32_t pr_format_select_v1(const PrFormatFactsV1 *facts, PrFormatSelectionV1 *result);
#ifdef __cplusplus
}
#endif
#endif
