#ifndef POKELINUX_SETTINGS_CLIENT_H
#define POKELINUX_SETTINGS_CLIENT_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct PrSettingsClient PrSettingsClient;
enum { PR_SETTINGS_OK=0, PR_SETTINGS_INVALID=1, PR_SETTINGS_WAIT=2, PR_SETTINGS_ABORT=3, PR_SETTINGS_REFUSED=4 };
enum { PR_SETTINGS_UNAVAILABLE=0, PR_SETTINGS_READY=1, PR_SETTINGS_DENIED=2, PR_SETTINGS_PENDING=3, PR_SETTINGS_UNCERTAIN=4, PR_SETTINGS_STOPPED=5, PR_SETTINGS_CONTROLS_UNAVAILABLE=6, PR_SETTINGS_NOT_SENT=7, PR_SETTINGS_UNCONFIRMED_UNAVAILABLE=8 };
typedef struct {
 uint64_t revision, until_ns;
 uint32_t connected, available, pending, can_set, status;
 int32_t requested_percent, observed_percent, actual_percent;
} PrSettingsView;
/* Live aligned pointers, disjoint output storage, no free/call overlap.
 * Non-OK leaves output unchanged. Path bytes are copied; no pointer retained.
 * One explicit Set; no replay. View samples CLOCK_BOOTTIME on every call.
 * Choice outcomes survive readonly recovery independently of current controls.
 * NOT_SENT proves no full Set send; UNCONFIRMED_UNAVAILABLE records a peer
 * Unavailable after send and makes no claim about device effects.
 * Stop captures one original 200ms budget; join returns WAIT/OK/ABORT.
 * Free refuses until actual join; the caller must abort on an unresolved join.
 */
int32_t pr_settings_client_new_v1(const uint8_t*,size_t,PrSettingsClient**);
int32_t pr_settings_client_view_v1(PrSettingsClient*,PrSettingsView*);
int32_t pr_settings_client_set_v1(PrSettingsClient*,uint32_t);
int32_t pr_settings_client_stop_v1(PrSettingsClient*);
int32_t pr_settings_client_join_v1(PrSettingsClient*);
int32_t pr_settings_client_free_v1(PrSettingsClient*);
#ifdef __cplusplus
}
#endif
#endif
