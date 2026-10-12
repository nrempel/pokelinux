/* SPDX-License-Identifier: MIT
 * Explicit-device diagnostic: allocates/renders but never modesets or presents.
 * Do not run on a phone until a separate device-test step is authorized.
 */
#include "pixel_runtime.h"
#include "fence_wait.h"
#include <stdlib.h>
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <drm_fourcc.h>
#include <drm_mode.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <xf86drm.h>
static int extension(const char *s, const char *name) {
  if (!s)
    return 0;
  size_t n = strlen(name);
  for (const char *p = s; (p = strstr(p, name)); p += n)
    if ((p == s || p[-1] == ' ') && (p[n] == ' ' || p[n] == '\0'))
      return 1;
  return 0;
}
static void usage(void) {
  puts("usage: pixel-dmabuf-probe --drm /dev/dri/cardN\nAllocates one 64x64 "
       "linear buffer; imports, renders, reads back and waits for a native "
       "fence. Never changes display modes.");
}
int main(int argc, char **argv) {
  if (argc == 2 && !strcmp(argv[1], "--help")) {
    usage();
    return 0;
  }
  if (argc != 3 || strcmp(argv[1], "--drm")) {
    usage();
    return 2;
  }
  const char *runtime_root = getenv("PIXEL_RUNTIME");
  if (runtime_root && pixel_runtime_start(runtime_root)) {
    perror("runtime snapshot");
    return 1;
  }
  int result = 1, fd = -1, dmabuf = -1, fence = -1;
  struct drm_mode_create_dumb bo = {.width = 64, .height = 64, .bpp = 32};
  EGLDisplay display = EGL_NO_DISPLAY;
  EGLContext ctx = EGL_NO_CONTEXT;
  EGLImageKHR image = EGL_NO_IMAGE_KHR;
  EGLSyncKHR sync = EGL_NO_SYNC_KHR;
  GLuint rb = 0, fbo = 0;
  PFNEGLDESTROYIMAGEKHRPROC destroy_image = NULL;
  PFNEGLDESTROYSYNCKHRPROC destroy_sync = NULL;
  fd = open(argv[2], O_RDWR | O_CLOEXEC | O_NOFOLLOW);
  if (fd < 0) {
    perror("open DRM node");
    goto done;
  }
  struct stat st;
  if (fstat(fd, &st) || !S_ISCHR(st.st_mode)) {
    fprintf(stderr, "DRM node must be a character device\n");
    goto done;
  }
  if (drmIoctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, &bo)) {
    perror("CREATE_DUMB");
    goto done;
  }
  if (drmPrimeHandleToFD(fd, bo.handle, DRM_CLOEXEC | DRM_RDWR, &dmabuf)) {
    perror("PRIME export");
    goto done;
  }
  display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
  EGLint major, minor;
  if (display == EGL_NO_DISPLAY || !eglInitialize(display, &major, &minor)) {
    fprintf(stderr, "EGL initialization failed: 0x%x\n", eglGetError());
    goto done;
  }
  const char *ext = eglQueryString(display, EGL_EXTENSIONS);
  const char *required[] = {
      "EGL_EXT_image_dma_buf_import", "EGL_KHR_surfaceless_context",
      "EGL_KHR_no_config_context", "EGL_ANDROID_native_fence_sync"};
  for (unsigned i = 0; i < sizeof(required) / sizeof(required[0]); i++)
    if (!extension(ext, required[i])) {
      fprintf(stderr, "Missing %s\n", required[i]);
      goto done;
    }
  PFNEGLCREATEIMAGEKHRPROC create_image =
      (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
  destroy_image =
      (PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
  PFNGLEGLIMAGETARGETRENDERBUFFERSTORAGEOESPROC attach =
      (PFNGLEGLIMAGETARGETRENDERBUFFERSTORAGEOESPROC)eglGetProcAddress(
          "glEGLImageTargetRenderbufferStorageOES");
  PFNEGLCREATESYNCKHRPROC create_sync =
      (PFNEGLCREATESYNCKHRPROC)eglGetProcAddress("eglCreateSyncKHR");
  destroy_sync =
      (PFNEGLDESTROYSYNCKHRPROC)eglGetProcAddress("eglDestroySyncKHR");
  PFNEGLDUPNATIVEFENCEFDANDROIDPROC dup_fence =
      (PFNEGLDUPNATIVEFENCEFDANDROIDPROC)eglGetProcAddress(
          "eglDupNativeFenceFDANDROID");
  if (!create_image || !destroy_image || !attach || !create_sync ||
      !destroy_sync || !dup_fence) {
    fprintf(stderr, "Missing required EGL procedure\n");
    goto done;
  }
  EGLint ca[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
  if (!eglBindAPI(EGL_OPENGL_ES_API)) {
    fprintf(stderr, "GLES API bind failed\n");
    goto done;
  }
  ctx = eglCreateContext(display, EGL_NO_CONFIG_KHR, EGL_NO_CONTEXT, ca);
  if (ctx == EGL_NO_CONTEXT ||
      !eglMakeCurrent(display, EGL_NO_SURFACE, EGL_NO_SURFACE, ctx)) {
    fprintf(stderr, "GLES3 surfaceless context failed\n");
    goto done;
  }
  EGLint attrs[] = {EGL_WIDTH,
                    64,
                    EGL_HEIGHT,
                    64,
                    EGL_LINUX_DRM_FOURCC_EXT,
                    DRM_FORMAT_ABGR8888,
                    EGL_DMA_BUF_PLANE0_FD_EXT,
                    dmabuf,
                    EGL_DMA_BUF_PLANE0_OFFSET_EXT,
                    0,
                    EGL_DMA_BUF_PLANE0_PITCH_EXT,
                    (EGLint)bo.pitch,
                    EGL_NONE};
  image =
      create_image(display, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attrs);
  if (image == EGL_NO_IMAGE_KHR) {
    fprintf(stderr, "dma-buf import failed: 0x%x\n", eglGetError());
    goto done;
  }
  glGenRenderbuffers(1, &rb);
  glBindRenderbuffer(GL_RENDERBUFFER, rb);
  attach(GL_RENDERBUFFER, image);
  glGenFramebuffers(1, &fbo);
  glBindFramebuffer(GL_FRAMEBUFFER, fbo);
  glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0,
                            GL_RENDERBUFFER, rb);
  if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
    fprintf(stderr, "Imported buffer is not a complete render target\n");
    goto done;
  }
  glClearColor(1, 0, 0, 1);
  glClear(GL_COLOR_BUFFER_BIT);
  EGLint fa[] = {EGL_SYNC_NATIVE_FENCE_FD_ANDROID,
                 EGL_NO_NATIVE_FENCE_FD_ANDROID, EGL_NONE};
  sync = create_sync(display, EGL_SYNC_NATIVE_FENCE_ANDROID, fa);
  if (sync == EGL_NO_SYNC_KHR) {
    fprintf(stderr, "Fence creation failed\n");
    goto done;
  }
  glFlush();
  fence = dup_fence(display, sync);
  if (fence < 0) {
    fprintf(stderr, "Fence export failed\n");
    goto done;
  }
  if (pixel_wait_fence(fence, 5000)) {
    fprintf(stderr, "Fence did not complete cleanly\n");
    goto done;
  }
  unsigned char pixel[4] = {0};
  glReadPixels(0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel);
  if (glGetError() != GL_NO_ERROR || pixel[0] != 255 || pixel[1] || pixel[2] ||
      pixel[3] != 255) {
    fprintf(stderr, "Pixel readback mismatch\n");
    goto done;
  }
  printf("PASS: linear ABGR8888 dma-buf, pitch=%u, GLES3 render/readback, "
         "native fence. Scanout NOT tested.\n",
         bo.pitch);
  result = 0;
done:
  if (fence >= 0)
    close(fence);
  if (fbo)
    glDeleteFramebuffers(1, &fbo);
  if (rb)
    glDeleteRenderbuffers(1, &rb);
  if (sync != EGL_NO_SYNC_KHR && destroy_sync)
    destroy_sync(display, sync);
  if (image != EGL_NO_IMAGE_KHR && destroy_image)
    destroy_image(display, image);
  if (ctx != EGL_NO_CONTEXT) {
    eglMakeCurrent(display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    eglDestroyContext(display, ctx);
  }
  if (display != EGL_NO_DISPLAY)
    eglTerminate(display);
  if (dmabuf >= 0)
    close(dmabuf);
  if (bo.handle) {
    struct drm_mode_destroy_dumb d = {.handle = bo.handle};
    if (drmIoctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &d))
      perror("DESTROY_DUMB");
  }
  if (fd >= 0)
    close(fd);
  return result;
}
