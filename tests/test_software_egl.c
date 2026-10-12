/* SPDX-License-Identifier: MIT. Real Mesa software EGL through backend ABI. */
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <ws.h>
static void *resolve(const char *name) {
  return !strcmp(name, "eglGetProcAddress") ? (void *)eglGetProcAddress : NULL;
}
int main(int argc, char **argv) {
  assert(argc == 2);
  void *module = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  if (!module) {
    puts(dlerror());
    return 1;
  }
  struct ws_module *ws = dlsym(module, "ws_module_info");
  assert(ws);
  struct ws_egl_interface iface = {.android_egl_dlsym = resolve};
  ws->init_module(&iface);
  struct _EGLDisplay *mapped = ws->GetDisplay(EGL_DEFAULT_DISPLAY);
  assert(mapped);
  EGLDisplay d =
      eglGetPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA, NULL, NULL);
  assert(d != EGL_NO_DISPLAY);
  EGLint major, minor;
  assert(eglInitialize(d, &major, &minor));
  mapped->dpy = d;
  assert(strstr(ws->eglQueryString(d, EGL_EXTENSIONS, eglQueryString),
                "EGL_KHR_surfaceless_context"));
  assert(eglBindAPI(EGL_OPENGL_ES_API));
  EGLint attrs[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
  EGLContext c = eglCreateContext(d, EGL_NO_CONFIG_KHR, EGL_NO_CONTEXT, attrs);
  assert(c != EGL_NO_CONTEXT);
  assert(eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, c));
  GLuint tex, fbo;
  glGenTextures(1, &tex);
  glBindTexture(GL_TEXTURE_2D, tex);
  glTexStorage2D(GL_TEXTURE_2D, 1, GL_RGBA8, 2, 2);
  glGenFramebuffers(1, &fbo);
  glBindFramebuffer(GL_FRAMEBUFFER, fbo);
  glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                         tex, 0);
  assert(glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE);
  glClearColor(1, 0, 0, 1);
  glClear(GL_COLOR_BUFFER_BIT);
  unsigned char pixel[4] = {0};
  glReadPixels(0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel);
  assert(glGetError() == GL_NO_ERROR && pixel[0] == 255 && pixel[1] == 0 &&
         pixel[2] == 0 && pixel[3] == 255);
  printf("software EGL %d.%d: %s; RGBA readback 255,0,0,255\n", major, minor,
         glGetString(GL_RENDERER));
  glDeleteFramebuffers(1, &fbo);
  glDeleteTextures(1, &tex);
  eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
  eglDestroyContext(d, c);
  eglTerminate(d);
  ws->Terminate(mapped);
  dlclose(module);
  return 0;
}
