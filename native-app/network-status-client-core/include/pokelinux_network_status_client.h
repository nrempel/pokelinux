#ifndef POKELINUX_NETWORK_STATUS_CLIENT_H
#define POKELINUX_NETWORK_STATUS_CLIENT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct PrNetworkStatusClient PrNetworkStatusClient;
enum { PR_NETWORK_STATUS_OK = 0, PR_NETWORK_STATUS_INVALID = 1,
       PR_NETWORK_STATUS_REFUSED = 2 };
typedef struct PrNetworkStatusViewV1 {
    uint32_t available;
    int32_t local_interfaces; /* -1 when unavailable; healthy empty is 0 */
    int32_t address_records; /* -1 when unavailable; not usable connections */
    uint64_t revision; /* 0 when unavailable */
} PrNetworkStatusViewV1;
/* Same-thread calls and free must not overlap. Handles must be live; complete
 * output slots must be aligned, exclusively writable, and disjoint from handle.
 * No caller pointer is retained. Non-OK leaves output unchanged; new requires
 * *out == NULL. Fixed /run/pokelinux-network/snapshot.sock, root:root peer only.
 * No requests/writes, provider selector, commands, thread or worker. tick does
 * bounded nonblocking work; view samples CLOCK_BOOTTIME and expires stale data
 * even when timer delivery is late. Counts confer no connectivity authority. */
int32_t pr_network_status_client_new_v1(PrNetworkStatusClient **out);
void pr_network_status_client_free_v1(PrNetworkStatusClient *client);
int32_t pr_network_status_client_tick_v1(PrNetworkStatusClient *client);
int32_t pr_network_status_client_view_v1(PrNetworkStatusClient *client,
                                        PrNetworkStatusViewV1 *out);
#ifdef __cplusplus
}
static_assert(sizeof(PrNetworkStatusViewV1) == 24, "network status ABI layout");
#endif
#endif
