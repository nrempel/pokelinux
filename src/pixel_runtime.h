/* SPDX-License-Identifier: MIT */
#ifndef PIXEL_RUNTIME_H
#define PIXEL_RUNTIME_H
/* Initialize once before starting threads or loading Android libraries.
 * Reads an immutable factory property snapshot; starts no Android services.
 * Files remain unchanged. Returns 0 on success, -1 with errno on failure. */
int pixel_runtime_start(const char *root);
/* Exposed for ABI tests and embedding. Unknown symbols return NULL. */
void *pixel_runtime_hook(const char *name, const char *requester);
#endif
