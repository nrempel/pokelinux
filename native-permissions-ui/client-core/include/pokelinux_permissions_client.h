#ifndef POKELINUX_PERMISSIONS_CLIENT_H
#define POKELINUX_PERMISSIONS_CLIENT_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* All calls are serialized on one GUI thread; never reuse a freed handle. */
typedef struct PrPermissionsHandle PrPermissionsHandle;
typedef struct { int32_t allowed; uint32_t pending, can_set, unconfirmed; } PrPermissionsView;
enum { PR_PERMISSIONS_OK=0, PR_PERMISSIONS_INVALID=1, PR_PERMISSIONS_REFUSED=2 };
int32_t pr_permissions_new_v1(const uint8_t *, size_t, PrPermissionsHandle **);
int32_t pr_permissions_active_v1(PrPermissionsHandle *, uint32_t);
int32_t pr_permissions_tick_v1(PrPermissionsHandle *);
int32_t pr_permissions_view_v1(PrPermissionsHandle *, PrPermissionsView *);
int32_t pr_permissions_choose_v1(PrPermissionsHandle *, uint32_t);
int32_t pr_permissions_free_v1(PrPermissionsHandle *);
#ifdef __cplusplus
}
#endif
#endif
