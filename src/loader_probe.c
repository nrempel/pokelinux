/* SPDX-License-Identifier: MIT */
#include "pixel_runtime.h"
#include <stdlib.h>
#include <dlfcn.h>
#include <hybris/common/binding.h>
#include <stdio.h>
int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: loader-probe /absolute/android/library.so\n");
    return 2;
  }
  const char *root = getenv("PIXEL_RUNTIME");
  if (root && pixel_runtime_start(root)) {
    perror("runtime snapshot");
    return 1;
  }
  void *handle = android_dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  if (!handle) {
    fprintf(stderr, "LOAD_FAILED: %s\n", android_dlerror());
    return 1;
  }
  printf("LOAD_OK: %s\n", argv[1]);
  /* Do not unload: some vendor constructors register process-lifetime state. */
  return 0;
}
