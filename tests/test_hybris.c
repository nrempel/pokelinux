/* SPDX-License-Identifier: MIT */
#include <EGL/egl.h>
#include <EGL/eglext.h>
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>
int main(void) {
  assert(!strstr(eglQueryString(EGL_NO_DISPLAY, EGL_EXTENSIONS), "platform_wayland"));
  EGLDisplay d = eglGetDisplay(EGL_DEFAULT_DISPLAY);
  assert(d != EGL_NO_DISPLAY);
  EGLint major, minor, n, f;
  assert(eglInitialize(d, &major, &minor));
  assert(!strstr(eglQueryString(d, EGL_EXTENSIONS), "platform_gbm"));
  PFNEGLQUERYDMABUFFORMATSEXTPROC formats =
      (PFNEGLQUERYDMABUFFORMATSEXTPROC)eglGetProcAddress(
          "eglQueryDmaBufFormatsEXT");
  assert(formats && formats(d, 1, &f, &n) && n == 1 && f == 0x34325241);
  PFNEGLCREATEIMAGEKHRPROC create_image =
      (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
  PFNEGLDESTROYIMAGEKHRPROC destroy_image =
      (PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
  assert(create_image && destroy_image);
  EGLint attrs[] = {EGL_WIDTH,
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
                    0,
                    EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
                    1,
                    EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
                    2,
                    EGL_NONE};
  EGLImageKHR image =
      create_image(d, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attrs);
  assert(image != EGL_NO_IMAGE_KHR && destroy_image(d, image));
  assert(eglCreateWindowSurface(d, NULL, (EGLNativeWindowType)1, NULL) ==
         EGL_NO_SURFACE);
  assert(eglGetError() == EGL_BAD_NATIVE_WINDOW);
  assert(eglTerminate(d));
  puts("real libhybris wrapper integration passed");
  return 0;
}
