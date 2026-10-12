/* SPDX-License-Identifier: MIT */
#define _GNU_SOURCE
#include "pixel_runtime.h"
#include <hybris/common/hooks.h>
#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
/* Exported by libhybris; omitted from its public binding header. */
extern void *android_dlopen_ext(const char *, int, const void *);

struct property {
  struct property *next;
  char *name;
  char *value;
};
static struct property *properties;
static char *runtime_root;
static bool ready;

static void discard(struct property *p) {
  while (p) {
    struct property *next = p->next;
    free(p->name);
    free(p->value);
    free(p);
    p = next;
  }
}
static struct property *lookup(struct property *p, const char *name) {
  if (!name)
    return NULL;
  for (; p; p = p->next)
    if (!strcmp(p->name, name))
      return p;
  return NULL;
}
static int read_file(struct property **list, const char *root,
                     const char *suffix) {
  char *path = NULL;
  if (asprintf(&path, "%s%s", root, suffix) < 0)
    return -1;
  FILE *f = fopen(path, "re");
  free(path);
  if (!f)
    return -1;
  char *line = NULL;
  size_t cap = 0;
  ssize_t len;
  int result = 0;
  while ((len = getline(&line, &cap, f)) >= 0) {
    if (len > 65536) {
      errno = EOVERFLOW;
      result = -1;
      break;
    }
    while (len && (line[len - 1] == '\n' || line[len - 1] == '\r'))
      line[--len] = 0;
    char *name = line;
    while (isspace((unsigned char)*name))
      name++;
    if (!*name || *name == '#')
      continue;
    char *value = strchr(name, '=');
    if (!value) {
      errno = EINVAL;
      result = -1;
      break;
    }
    *value++ = 0;
    if (!*name) {
      errno = EINVAL;
      result = -1;
      break;
    }
    for (char *c = name; *c; c++)
      if (!(isalnum((unsigned char)*c) || *c == '.' || *c == '_' ||
            *c == '-')) {
        errno = EINVAL;
        result = -1;
        break;
      }
    if (result)
      break;
    struct property *p = lookup(*list, name);
    char *copy = strdup(value);
    if (!copy) {
      result = -1;
      break;
    }
    if (p) {
      free(p->value);
      p->value = copy;
      continue;
    }
    p = calloc(1, sizeof(*p));
    if (!p) {
      free(copy);
      result = -1;
      break;
    }
    p->name = strdup(name);
    p->value = copy;
    if (!p->name) {
      free(p->value);
      free(p);
      result = -1;
      break;
    }
    p->next = *list;
    *list = p;
  }
  if (ferror(f))
    result = -1;
  int saved = errno;
  free(line);
  fclose(f);
  errno = saved;
  return result;
}
static const struct property *property_find(const char *name) {
  return lookup(properties, name);
}
static int property_get(const char *name, char *value) {
  const struct property *p = property_find(name);
  value[0] = 0;
  if (!p)
    return 0;
  size_t n = strlen(p->value);
  if (n >= 92) {
    errno = EOVERFLOW;
    return 0;
  }
  memcpy(value, p->value, n + 1);
  return (int)n;
}
static uint32_t property_serial(const struct property *p) {
  if (!p)
    return 0;
  size_t len = strlen(p->value);
  return ((uint32_t)(len > 255 ? 255 : len) << 24) | 2;
}
static void property_callback(const struct property *p,
                              void (*fn)(void *, const char *, const char *,
                                         uint32_t),
                              void *cookie) {
  if (p && fn)
    fn(cookie, p->name, p->value, property_serial(p));
}
static int property_read(const struct property *p, char *name, char *value) {
  if (!p) {
    if (name)
      name[0] = 0;
    if (value)
      value[0] = 0;
    errno = EINVAL;
    return 0;
  }
  if (name) {
    size_t n = strlen(p->name);
    if (n > 31)
      n = 31;
    memcpy(name, p->name, n);
    name[n] = 0;
  }
  if (!value)
    return 0;
  return property_get(p->name, value);
}
static int property_foreach(void (*fn)(const struct property *, void *),
                            void *cookie) {
  if (!fn) {
    errno = EINVAL;
    return -1;
  }
  for (struct property *p = properties; p; p = p->next)
    fn(p, cookie);
  return 0;
}
static int property_init(void) { return ready ? 0 : -1; }
static uint32_t area_serial(void) { return 2; }
static int property_set(const char *name, const char *value) {
  (void)name;
  (void)value;
  errno = EROFS;
  return -1;
}
static int property_update(void *p, const char *v, unsigned n) {
  (void)p;
  (void)v;
  (void)n;
  errno = EROFS;
  return -1;
}
static int property_add(const char *n, unsigned nl, const char *v,
                        unsigned vl) {
  (void)n;
  (void)nl;
  (void)v;
  (void)vl;
  errno = EROFS;
  return -1;
}
static bool property_wait(const struct property *p, uint32_t old,
                          uint32_t *next, const struct timespec *timeout) {
  uint32_t serial = p ? property_serial(p) : area_serial();
  if (next)
    *next = serial;
  if (serial != old)
    return true;
  /* The snapshot never changes. Honor a bounded wait; reject an unbounded
   * service-style wait because this provider cannot receive external updates.
   */
  if (!timeout) {
    errno = ENOTSUP;
    return false;
  }
  struct timespec remain = *timeout;
  while (nanosleep(&remain, &remain) < 0)
    if (errno != EINTR)
      return false;
  return false;
}
static uint32_t property_wait_any(uint32_t old) {
  (void)old;
  errno = ENOTSUP;
  return area_serial();
}
static const struct property *property_find_nth(unsigned n) {
  struct property *p = properties;
  while (p && n--)
    p = p->next;
  return p;
}

static const char *map_library(const char *name, char **owned) {
  *owned = NULL;
  if (!name || name[0] != '/')
    return name;
  const char *suffix = name;
  if (!strncmp(name, "/system/vendor/", 15))
    suffix = name + 7;
  if (strncmp(suffix, "/vendor/", 8) && strncmp(suffix, "/system/", 8) &&
      strncmp(suffix, "/apex/", 6))
    return name;
  /* Reject traversal within paths that cross into the staged runtime. */
  if (strstr(suffix, "/../") || !strcmp(suffix + strlen(suffix) - 3, "/..")) {
    errno = EINVAL;
    return NULL;
  }
  if (asprintf(owned, "%s%s", runtime_root, suffix) < 0)
    return NULL;
  return *owned;
}
static void *runtime_dlopen(const char *name, int flags) {
  char *owned;
  const char *path = map_library(name, &owned);
  if (name && !path)
    return NULL;
  void *h = android_dlopen(path, flags);
  free(owned);
  return h;
}
static void *runtime_dlopen_ext(const char *name, int flags, const void *ext) {
  char *owned;
  const char *path = map_library(name, &owned);
  if (name && !path)
    return NULL;
  void *h = android_dlopen_ext(path, flags, ext);
  free(owned);
  return h;
}
void *pixel_runtime_hook(const char *name, const char *requester) {
  (void)requester;
  if (!ready || !name)
    return NULL;
#define HOOK(n, fn)                                                            \
  if (!strcmp(name, n))                                                        \
  return (void *)(fn)
  HOOK("__system_properties_init", property_init);
  HOOK("__system_property_find", property_find);
  HOOK("__system_property_get", property_get);
  HOOK("__system_property_read", property_read);
  HOOK("__system_property_read_callback", property_callback);
  HOOK("__system_property_serial", property_serial);
  HOOK("__system_property_area_serial", area_serial);
  HOOK("__system_property_foreach", property_foreach);
  HOOK("__system_property_set", property_set);
  HOOK("__system_property_update", property_update);
  HOOK("__system_property_add", property_add);
  HOOK("__system_property_wait", property_wait);
  HOOK("__system_property_wait_any", property_wait_any);
  HOOK("__system_property_find_nth", property_find_nth);
  HOOK("dlopen", runtime_dlopen);
  HOOK("android_dlopen_ext", runtime_dlopen_ext);
#undef HOOK
  return NULL;
}
int pixel_runtime_start(const char *root) {
  if (ready) {
    errno = EALREADY;
    return -1;
  }
  if (!root || root[0] != '/') {
    errno = EINVAL;
    return -1;
  }
  char *canonical = realpath(root, NULL);
  if (!canonical)
    return -1;
  struct property *pending = NULL;
  if (read_file(&pending, canonical, "/system/build.prop") ||
      read_file(&pending, canonical, "/vendor/build.prop")) {
    int saved = errno;
    discard(pending);
    free(canonical);
    errno = saved;
    return -1;
  }
  runtime_root = canonical;
  properties = pending;
  ready = true;
  hybris_set_hook_callback(pixel_runtime_hook);
  return 0;
}
