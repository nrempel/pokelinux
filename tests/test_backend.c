#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <ws.h>
extern struct ws_module ws_module_info;
static int calls;
static EGLBoolean formats(EGLDisplay d, EGLint max, EGLint *f, EGLint *n) {
  assert(d == (void *)0x42);
  assert(max == 1);
  *f = 0x34325241;
  *n = 1;
  calls++;
  return EGL_TRUE;
}
static EGLBoolean modifiers(EGLDisplay d, EGLint f, EGLint max, EGLuint64KHR *m,
                            EGLBoolean *e, EGLint *n) {
  assert(d == (void *)0x42 && f == 0x34325241 && max == 1);
  *m = 0x1234567800000000ULL;
  *e = EGL_TRUE;
  *n = 1;
  calls++;
  return EGL_TRUE;
}
static __eglMustCastToProperFunctionPointerType getproc(const char *n) {
  if (!strcmp(n, "eglQueryDmaBufFormatsEXT"))
    return (__eglMustCastToProperFunctionPointerType)formats;
  if (!strcmp(n, "eglQueryDmaBufModifiersEXT"))
    return (__eglMustCastToProperFunctionPointerType)modifiers;
  return NULL;
}
static void *lookup(const char *n) {
  return !strcmp(n, "eglGetProcAddress") ? (void *)getproc : NULL;
}
static void *missing(const char *n) {
  (void)n;
  return NULL;
}
static const char *query(EGLDisplay d, EGLint n) {
  (void)d;
  assert(n == EGL_EXTENSIONS);
  return "EGL_EXT_image_dma_buf_import";
}
int main(void) {
  struct ws_egl_interface iface = {.android_egl_dlsym = lookup};
  ws_module_info.init_module(&iface);
  struct _EGLDisplay *d = ws_module_info.GetDisplay(EGL_DEFAULT_DISPLAY);
  assert(d);
  assert(d->dpy == EGL_NO_DISPLAY);
  assert(!ws_module_info.GetDisplay((EGLNativeDisplayType)(uintptr_t)1));
  assert(!ws_module_info.CreateWindow((EGLNativeWindowType)(uintptr_t)1, d));
  EGLint a[] = {EGL_WIDTH,
                64,
                EGL_HEIGHT,
                64,
                EGL_LINUX_DRM_FOURCC_EXT,
                0x34325241,
                EGL_DMA_BUF_PLANE0_FD_EXT,
                17,
                EGL_DMA_BUF_PLANE0_PITCH_EXT,
                256,
                EGL_DMA_BUF_PLANE0_OFFSET_EXT,
                4096,
                EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
                1,
                EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
                2,
                EGL_NONE};
  const EGLint *ap = a;
  EGLContext ctx = EGL_NO_CONTEXT;
  EGLenum target = EGL_LINUX_DMA_BUF_EXT;
  EGLClientBuffer buf = NULL;
  ws_module_info.passthroughImageKHR(&ctx, &target, &buf, &ap);
  assert(ap == a && target == EGL_LINUX_DMA_BUF_EXT && ctx == EGL_NO_CONTEXT &&
         buf == NULL && a[7] == 17);
  assert(!strcmp(
      ws_module_info.eglQueryString((void *)0x42, EGL_EXTENSIONS, query),
      "EGL_EXT_image_dma_buf_import"));
  EGLint f, n;
  EGLuint64KHR m;
  EGLBoolean e;
  assert(ws_module_info.queryDmaBufFormatsEXT((void *)0x42, 1, &f, &n) &&
         n == 1);
  assert(
      ws_module_info.queryDmaBufModifiersEXT((void *)0x42, f, 1, &m, &e, &n) &&
      e == EGL_TRUE && m == 0x1234567800000000ULL);
  assert(calls == 2);
  assert(ws_module_info.eglGetProcAddress("unknown") == NULL);
  ws_module_info.Terminate(d);
  iface.android_egl_dlsym = missing;
  ws_module_info.init_module(&iface);
  assert(!ws_module_info.queryDmaBufFormatsEXT((void *)0x42, 1, &f, &n));
  assert(
      !ws_module_info.queryDmaBufModifiersEXT((void *)0x42, f, 1, &m, &e, &n));
  puts("backend contracts passed");
  return 0;
}
