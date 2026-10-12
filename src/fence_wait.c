/* SPDX-License-Identifier: MIT */
#include "fence_wait.h"
#include <errno.h>
#include <poll.h>
#include <stdint.h>
#include <time.h>
static int64_t milliseconds(void) {
  struct timespec t;
  if (clock_gettime(CLOCK_MONOTONIC, &t))
    return -1;
  return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
int pixel_wait_fence(int fd, int timeout_ms) {
  if (fd < 0) {
    errno = EBADF;
    return -1;
  }
  if (timeout_ms < 0) {
    errno = EINVAL;
    return -1;
  }
  int64_t start = milliseconds();
  if (start < 0)
    return -1;
  int64_t deadline = start + timeout_ms;
  int remaining = timeout_ms;
  struct pollfd p = {.fd = fd, .events = POLLIN};
  for (;;) {
    int result = poll(&p, 1, remaining);
    if (result > 0) {
      if (p.revents & POLLNVAL) {
        errno = EBADF;
        return -1;
      }
      if ((p.revents & POLLIN) && !(p.revents & (POLLERR | POLLHUP)))
        return 0;
      errno = EIO;
      return -1;
    }
    if (result == 0) {
      errno = ETIMEDOUT;
      return -1;
    }
    if (errno != EINTR)
      return -1;
    int64_t now = milliseconds();
    if (now < 0)
      return -1;
    if (now >= deadline) {
      errno = ETIMEDOUT;
      return -1;
    }
    remaining = (int)(deadline - now);
  }
}
