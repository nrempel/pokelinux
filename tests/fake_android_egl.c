/* SPDX-License-Identifier: MIT
 * Dependency-free ELF fixture. Models the Android ABI only; not GPU proof. */
#include <EGL/egl.h>
#include <EGL/eglext.h>
static int same(const char *a, const char *b) {
  while (*a && *a == *b) {
    a++;
    b++;
  }
  return *a == *b;
}
EGLDisplay eglGetDisplay(EGLNativeDisplayType native) {
  return native == EGL_DEFAULT_DISPLAY ? (void *)0x42 : EGL_NO_DISPLAY;
}
EGLBoolean eglInitialize(EGLDisplay d, EGLint *major, EGLint *minor) {
  if (d != (void *)0x42)
    return EGL_FALSE;
  *major = 1;
  *minor = 5;
  return EGL_TRUE;
}
EGLBoolean eglTerminate(EGLDisplay d) { return d == (void *)0x42; }
EGLint eglGetError(void) { return EGL_SUCCESS; }
const char *eglQueryString(EGLDisplay d, EGLint n) {
  (void)d;
  return n == EGL_EXTENSIONS
             ? "EGL_KHR_surfaceless_context EGL_KHR_no_config_context "
               "EGL_EXT_image_dma_buf_import "
               "EGL_EXT_image_dma_buf_import_modifiers"
             : "fixture";
}
EGLSurface eglCreateWindowSurface(EGLDisplay d, EGLConfig c,
                                  EGLNativeWindowType w, const EGLint *a) {
  (void)d;
  (void)c;
  (void)w;
  (void)a;
  return (void *)0xdead;
}
static EGLBoolean formats(EGLDisplay d, EGLint max, EGLint *f, EGLint *n) {
  (void)d;
  *n = 1;
  if (max > 0)
    *f = 0x34325241;
  return EGL_TRUE;
}
static EGLBoolean modifiers(EGLDisplay d, EGLint f, EGLint max, EGLuint64KHR *m,
                            EGLBoolean *e, EGLint *n) {
  (void)d;
  (void)f;
  *n = 1;
  if (max > 0) {
    *m = 0;
    *e = EGL_FALSE;
  }
  return EGL_TRUE;
}
__eglMustCastToProperFunctionPointerType eglGetProcAddress(const char *n) {
  if (same(n, "eglQueryDmaBufFormatsEXT"))
    return (__eglMustCastToProperFunctionPointerType)formats;
  if (same(n, "eglQueryDmaBufModifiersEXT"))
    return (__eglMustCastToProperFunctionPointerType)modifiers;
  return 0;
}

EGLImageKHR eglCreateImageKHR(EGLDisplay d, EGLContext ctx, EGLenum target,
                              EGLClientBuffer buffer, const EGLint *a) {
  if (d != (void *)0x42 || ctx != EGL_NO_CONTEXT ||
      target != EGL_LINUX_DMA_BUF_EXT || buffer || !a)
    return EGL_NO_IMAGE_KHR;
  int fd = 0, hi = 0;
  for (int i = 0; a[i] != EGL_NONE; i += 2) {
    if (a[i] == EGL_DMA_BUF_PLANE0_FD_EXT)
      fd = a[i + 1];
    if (a[i] == EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT)
      hi = a[i + 1];
  }
  return fd == 17 && hi == 2 ? (void *)0x1234 : EGL_NO_IMAGE_KHR;
}
EGLBoolean eglDestroyImageKHR(EGLDisplay d, EGLImageKHR image) {
  return d == (void *)0x42 && image == (void *)0x1234;
}
