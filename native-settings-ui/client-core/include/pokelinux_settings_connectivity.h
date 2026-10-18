#ifndef POKELINUX_SETTINGS_CONNECTIVITY_H
#define POKELINUX_SETTINGS_CONNECTIVITY_H
#include "pokelinux_settings_client.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct PrSettingsConnectivityClient PrSettingsConnectivityClient;
typedef struct {
 uint64_t observed_at_ns, until_ns;
 uint32_t transport_connected, available, pending, can_control, status;
 int32_t configured; /* -1 unknown, 0 disabled, 1 enabled; never Internet access. */
} PrSettingsConnectivityView;
/* Return and status values use PR_SETTINGS_* from the shared client header.
 * Live aligned pointers; disjoint output storage; no free/call overlap.
 * Non-OK leaves output unchanged. New copies path bytes and retains no pointer.
 * Capture deadline once at the originating gesture; zero means failure.
 * Set receives that ORIGINAL absolute CLOCK_BOOTTIME deadline. OK means local
 * admission, not a network acknowledgement. No implicit mutation retry.
 * View expires facts on every call; readonly facts cannot confirm old choices.
 * Stop withdraws locally; a previously committed send may still reach the actor.
 * Stop is neither Disconnect nor remote cleanup. One original200ms join budget;
 * free refuses until actual join. Abort on unresolved join; never detach.
 */
uint64_t pr_settings_connectivity_deadline_v1(void);
int32_t pr_settings_connectivity_new_v1(const uint8_t*,size_t,PrSettingsConnectivityClient**);
int32_t pr_settings_connectivity_view_v1(PrSettingsConnectivityClient*,PrSettingsConnectivityView*);
int32_t pr_settings_connectivity_set_v1(PrSettingsConnectivityClient*,uint32_t,uint64_t);
int32_t pr_settings_connectivity_stop_v1(PrSettingsConnectivityClient*);
int32_t pr_settings_connectivity_join_v1(PrSettingsConnectivityClient*);
int32_t pr_settings_connectivity_free_v1(PrSettingsConnectivityClient*);
#ifdef __cplusplus
}
#endif
#endif
