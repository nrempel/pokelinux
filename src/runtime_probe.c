/* SPDX-License-Identifier: MIT */
#include "pixel_runtime.h"
#include <hybris/common/binding.h>
#include <EGL/egl.h>
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
  if (argc != 4 || (strcmp(argv[2], "--load") && strcmp(argv[2], "--smoke") &&
                    strcmp(argv[2], "--egl"))) {
    fprintf(stderr, "usage: runtime-probe ROOT --load LIBRARY | --smoke "
                    "FIXTURE | --egl initialize\n");
    return 2;
  }
  setbuf(stdout, NULL);
  if (pixel_runtime_start(argv[1])) {
    perror("runtime snapshot");
    return 1;
  }
  if (strcmp(argv[2], "--egl")) {
    void *h = android_dlopen(argv[3], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
      fprintf(stderr, "LOAD_FAILED: %s\n", android_dlerror());
      return 1;
    }
    printf("LOAD_OK %s\n", argv[3]);
    if (!strcmp(argv[2], "--smoke")) {
      int (*smoke)(void) = android_dlsym(h, "stock_runtime_smoke");
      if (!smoke) {
        fprintf(stderr, "missing smoke entry point\n");
        return 1;
      }
      int result = smoke();
      printf("STOCK_ABI_SMOKE=%d\n", result);
      return result ? 1 : 0;
    }
    return 0;
  }
  if (strcmp(argv[3], "initialize"))
    return 2;
  const char *client = eglQueryString(EGL_NO_DISPLAY, EGL_EXTENSIONS);
  printf("CLIENT_EXTENSIONS=%s\n", client ? client : "(null)");
  if (!client)
    return 1;
  EGLDisplay d = eglGetDisplay(EGL_DEFAULT_DISPLAY);
  printf("DISPLAY=%p ERROR=0x%x\n", d, eglGetError());
  if (d == EGL_NO_DISPLAY)
    return 1;
  EGLint major = 0, minor = 0;
  EGLBoolean ok = eglInitialize(d, &major, &minor);
  EGLint error = eglGetError();
  printf("INITIALIZE=%u VERSION=%d.%d ERROR=0x%x\n", ok, major, minor, error);
  if (!ok) {
    puts("Rendering is NOT verified. Inspect the syscall trace; an EGL error "
         "alone does not prove the only missing component is hardware.");
    return 77;
  }
  printf("DISPLAY_EXTENSIONS=%s\n", eglQueryString(d, EGL_EXTENSIONS));
  eglTerminate(d);
  return 0;
}
