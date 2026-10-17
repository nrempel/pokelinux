#ifndef POKELINUX_BATTERY_CLIENT_H
#define POKELINUX_BATTERY_CLIENT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct PrBatteryClient PrBatteryClient;
enum { PR_BATTERY_OK = 0, PR_BATTERY_INVALID = 1, PR_BATTERY_REFUSED = 2 };
enum { PR_BATTERY_NO_STATUS = 0, PR_BATTERY_UNKNOWN = 1, PR_BATTERY_CHARGING = 2,
       PR_BATTERY_DISCHARGING = 3, PR_BATTERY_NOT_CHARGING = 4, PR_BATTERY_FULL = 5 };
typedef struct PrBatteryViewV1 {
    uint32_t available;
    int32_t capacity_percent; /* -1 when unavailable */
    uint32_t status;
    uint64_t revision; /* 0 when unavailable */
} PrBatteryViewV1;
/* Calls and free must not overlap. Handles must be live; complete output slots
 * must be aligned, exclusively writable, and disjoint from the handle. No caller
 * pointer is retained. Non-OK leaves output unchanged. new requires *out == NULL.
 * Endpoint is fixed; this interface cannot send commands or select a provider.
 * tick performs bounded nonblocking work. view samples CLOCK_BOOTTIME and clears
 * expired state even if the event loop has not delivered its timer. */
int32_t pr_battery_client_new_v1(PrBatteryClient **out);
void pr_battery_client_free_v1(PrBatteryClient *client); /* NULL allowed */
int32_t pr_battery_client_tick_v1(PrBatteryClient *client);
int32_t pr_battery_client_view_v1(PrBatteryClient *client, PrBatteryViewV1 *out);
#ifdef __cplusplus
}
static_assert(sizeof(PrBatteryViewV1) == 24, "battery ABI layout");
#endif
#endif
