#ifndef POKELINUX_REAL_SHM_CAPTURE_FIXTURE_H
#define POKELINUX_REAL_SHM_CAPTURE_FIXTURE_H
#include <stdint.h>
#define CAPTURE_WIDTH 5U
#define CAPTURE_HEIGHT 3U
#define CAPTURE_STRIDE 32U
#define CAPTURE_OFFSET 64U
#define CAPTURE_BYTES (CAPTURE_STRIDE * CAPTURE_HEIGHT)
#define CAPTURE_POOL_BYTES (CAPTURE_OFFSET + CAPTURE_BYTES)
static inline uint8_t capture_expected_byte(uint32_t i, uint8_t seed) {
    const uint32_t y = i / CAPTURE_STRIDE;
    const uint32_t row = i % CAPTURE_STRIDE;
    if (row >= CAPTURE_WIDTH * 4U)
        return (uint8_t)(seed ^ 0xA5U ^ row ^ (y * 13U));
    const uint32_t x = row / 4U;
    switch (row % 4U) {
        case 0: return (uint8_t)(seed + x * 17U + y * 3U);
        case 1: return (uint8_t)(seed ^ (x * 29U + y * 11U));
        case 2: return (uint8_t)(255U - seed - x * 7U - y * 19U);
        default: return UINT8_C(255); // valid premultiplied ARGB: opaque alpha
    }
}
#endif
