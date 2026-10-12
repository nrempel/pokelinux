/* SPDX-License-Identifier: MIT
 * Pixel surfaceless backend for the pinned libhybris ws_module ABI.
 * No GBM/Wayland window-system extension is advertised or emulated.
 */
#include <stdlib.h>
#include <ws.h>
static PFNEGLQUERYDMABUFFORMATSEXTPROC query_formats;
static PFNEGLQUERYDMABUFMODIFIERSEXTPROC query_modifiers;
static void init(struct ws_egl_interface *iface) {
  query_formats = NULL;
  query_modifiers = NULL;
  if (!iface || !iface->android_egl_dlsym)
    return;
  PFNEGLGETPROCADDRESSPROC getproc =
      (PFNEGLGETPROCADDRESSPROC)iface->android_egl_dlsym("eglGetProcAddress");
  if (getproc) {
    query_formats =
        (PFNEGLQUERYDMABUFFORMATSEXTPROC)getproc("eglQueryDmaBufFormatsEXT");
    query_modifiers = (PFNEGLQUERYDMABUFMODIFIERSEXTPROC)getproc(
        "eglQueryDmaBufModifiersEXT");
  }
}
static struct _EGLDisplay *get_display(EGLNativeDisplayType native) {
  if (native != EGL_DEFAULT_DISPLAY)
    return NULL;
  return calloc(1, sizeof(struct _EGLDisplay));
}
static void terminate(struct _EGLDisplay *display) { free(display); }
static EGLNativeWindowType create_window(EGLNativeWindowType window,
                                         struct _EGLDisplay *display) {
  (void)window;
  (void)display;
  return (EGLNativeWindowType)0;
}
static void destroy_window(EGLNativeWindowType window) { (void)window; }
static __eglMustCastToProperFunctionPointerType get_proc(const char *name) {
  (void)name;
  return NULL; /* Let libhybris dispatch to the vendor. */
}
static void image_passthrough(EGLContext *ctx, EGLenum *target,
                              EGLClientBuffer *buffer, const EGLint **attrs) {
  /* Preserve real dma-buf fds and every plane/modifier attribute exactly.
   * Validation and EGL errors belong to the vendor implementation. */
  (void)ctx;
  (void)target;
  (void)buffer;
  (void)attrs;
}
static const char *query_string(EGLDisplay d, EGLint name,
                                const char *(*real)(EGLDisplay, EGLint)) {
  return real(d, name);
}
static EGLBoolean formats(EGLDisplay d, EGLint max, EGLint *out,
                          EGLint *count) {
  return query_formats ? query_formats(d, max, out, count) : EGL_FALSE;
}
static EGLBoolean modifiers(EGLDisplay d, EGLint format, EGLint max,
                            EGLuint64KHR *out, EGLBoolean *external,
                            EGLint *count) {
  return query_modifiers ? query_modifiers(d, format, max, out, external, count)
                         : EGL_FALSE;
}
struct ws_module ws_module_info = {
    .init_module = init,
    .GetDisplay = get_display,
    .Terminate = terminate,
    .CreateWindow = create_window,
    .DestroyWindow = destroy_window,
    .eglGetProcAddress = get_proc,
    .passthroughImageKHR = image_passthrough,
    .eglQueryString = query_string,
    .queryDmaBufFormatsEXT = formats,
    .queryDmaBufModifiersEXT = modifiers,
};
