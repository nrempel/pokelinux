/* SPDX-License-Identifier: MIT */
#ifndef PIXEL_FENCE_WAIT_H
#define PIXEL_FENCE_WAIT_H
/* Returns 0 when readable, -1 with errno on timeout or failure. */
int pixel_wait_fence(int fd, int timeout_ms);
#endif
