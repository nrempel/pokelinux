/* SPDX-License-Identifier: MIT */
/* Android ABI declarations only: this fixture links to the actual factory libc
 * and libm, not the build host's libc. It exercises libhybris relocations. */
extern void *aligned_alloc(unsigned long, unsigned long);
extern void free(void *);
extern double cos(double);
extern int *__errno(void);
extern int open(const char *, int, ...);
extern int pthread_create(unsigned long *, const void *, void *(*)(void *), void *);
extern int pthread_join(unsigned long, void **);
extern int __system_property_get(const char *, char *);
static void *worker(void *arg) {
  for (unsigned i = 0; i < 100; ++i) {
    void *p = aligned_alloc(64, 128);
    if (!p || ((unsigned long)p & 63)) return (void *)1;
    ((char *)p)[127] = 42;
    free(p);
    double x = cos(2.0);
    if (x < -0.416148 || x > -0.416145) return (void *)2;
    if (open("/pixel-bridge-test/nonexistent", 0) != -1 || *__errno() != 2)
      return (void *)3;
    char value[92];
    if (__system_property_get("ro.build.version.sdk", value) != 2 || value[0] != '3' || value[1] != '7')
      return (void *)4;
  }
  return arg;
}
int stock_runtime_smoke(void) {
  unsigned long threads[4];
  for (unsigned i = 0; i < 4; ++i)
    if (pthread_create(&threads[i], (void *)0, worker, (void *)0)) return 5;
  for (unsigned i = 0; i < 4; ++i) {
    void *result = (void *)9;
    if (pthread_join(threads[i], &result) || result) return 6;
  }
  return 0;
}
