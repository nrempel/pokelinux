#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/* Authored SOURCE-ONLY comparison proposal, tied to one qualified original.
 * No public C++ ABI, parser, getter, SendMessage, handler or transport is used.
 * Never manually release the original data pointer: only named original D1.
 */
enum { OBJECT = 24, GUARD = 64, CAPACITY = 65568 };
_Static_assert(sizeof(void *) == 8 && sizeof(int) == 4 && sizeof(bool) == 1,
               "selected 64-bit pointer / signed32 int / byte bool ABI");
typedef void (*constructor)(void *, const char *, int, bool);
typedef void (*destructor)(void *);
static void *implementation_base;
static const char *implementation_path;
static void *vtable;
static void *named_d1;

static void fail(int status) { _exit(status); }
static void *bound(void *handle, const char *name, uintptr_t rva) {
    void *p = dlsym(handle, name);
    Dl_info info;
    if (!p || !dladdr(p, &info) || !info.dli_fname ||
        (uintptr_t)p - (uintptr_t)info.dli_fbase != rva) fail(71);
    const char *leaf = strrchr(info.dli_fname, '/');
    leaf = leaf ? leaf + 1 : info.dli_fname;
    if (strcmp(leaf, "vendor.radio.protocol.sit.base.so")) fail(71);
    if (implementation_base && implementation_base != info.dli_fbase) fail(71);
    implementation_base = info.dli_fbase;
    implementation_path = info.dli_fname;
    return p;
}
static void json_string(const char *text) {
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)text; *p; ++p) {
        if (*p == '"' || *p == '\\') { putchar('\\'); putchar(*p); }
        else if (*p < 0x20 || *p >= 0x7f) printf("\\u%04x", (unsigned)*p);
        else putchar(*p);
    }
    putchar('"');
}
static void bindings(void *c1, void *c2, void *d1, void *d2) {
    const char *names[] = {"_ZN9ModemDataC1EPKcib", "_ZN9ModemDataC2EPKcib",
                          "_ZN9ModemDataD1Ev", "_ZN9ModemDataD2Ev", "_ZTV9ModemData"};
    void *targets[] = {c1, c2, d1, d2, vtable};
    printf("{\"schema\":1,\"stage\":\"bindings\",\"provider_path\":");
    json_string(implementation_path);
    printf(",\"provider_base_u64\":%" PRIuPTR ",\"targets\":[", (uintptr_t)implementation_base);
    for (size_t i = 0; i < 5; ++i) {
        if (i) putchar(',');
        printf("{\"symbol\":"); json_string(names[i]);
        printf(",\"target_pointer_u64\":%" PRIuPTR ",\"computed_RVA_u64\":%" PRIuPTR "}",
               (uintptr_t)targets[i], (uintptr_t)targets[i] - (uintptr_t)implementation_base);
    }
    printf("]}\n");
}
static int disjoint_ranges(const void *a, size_t a_bytes, const void *b, size_t b_bytes) {
    uintptr_t a_start = (uintptr_t)a, b_start = (uintptr_t)b;
    if (a_bytes > UINTPTR_MAX - a_start || b_bytes > UINTPTR_MAX - b_start) return 0;
    return a_start + a_bytes <= b_start || b_start + b_bytes <= a_start;
}
static unsigned char *allocate(size_t count) {
    if (count > CAPACITY) fail(64);
    unsigned char *p = malloc(count + 2 * GUARD);
    if (!p || (uintptr_t)(p + GUARD) % 16) fail(71);
    memset(p, 0xa5, count + 2 * GUARD);
    memset(p + GUARD, 0, count);
    return p;
}
static int guards(const unsigned char *p, size_t count) {
    for (size_t i = 0; i < GUARD; ++i)
        if (p[i] != 0xa5 || p[GUARD + count + i] != 0xa5) return 0;
    return 1;
}
static uintptr_t pointer(const void *p, size_t offset) {
    uintptr_t value; memcpy(&value, (const char *)p + offset, sizeof value); return value;
}
static int32_t count_of(const void *p) {
    int32_t value; memcpy(&value, (const char *)p + 0x10, sizeof value); return value;
}
static unsigned flag_of(const void *p) { return ((const unsigned char *)p)[0x14]; }
static int vptr_and_d1(const void *p) {
    uintptr_t vp = pointer(p, 0);
    return vp == (uintptr_t)vtable + 16 && pointer((const void *)vp, 0) == (uintptr_t)named_d1;
}
static int integer(const char *text, int low, int high) {
    errno = 0; char *end;
    long value = strtol(text, &end, 10);
    if (errno || end == text || *end || value < low || value > high) fail(64);
    return (int)value;
}
static size_t read_source(const char *path, unsigned char bytes[CAPACITY]) {
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISREG(st.st_mode) || st.st_size < 0 || st.st_size > CAPACITY) fail(64);
    size_t wanted = (size_t)st.st_size, have = 0;
    while (have < wanted) {
        ssize_t n = read(fd, bytes + have, wanted - have);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) fail(64);
        have += (size_t)n;
    }
    unsigned char extra;
    ssize_t n;
    do { n = read(fd, &extra, 1); } while (n < 0 && errno == EINTR);
    if (n != 0 || close(fd)) fail(64);
    return wanted;
}
static void hex(const unsigned char *p, size_t count) {
    static const char digits[] = "0123456789abcdef";
    char encoded[2 * CAPACITY];
    if (count > CAPACITY) fail(64);
    for (size_t i = 0; i < count; ++i) {
        encoded[2 * i] = digits[p[i] >> 4];
        encoded[2 * i + 1] = digits[p[i] & 15];
    }
    if (fwrite(encoded, 1, 2 * count, stdout) != 2 * count) fail(76);
}
static void payload(const char *stage, const unsigned char *p, size_t count) {
    printf("{\"schema\":1,\"stage\":\"%s\",\"owned_bytes\":%zu,\"owned_hex\":\"", stage, count);
    hex(p, count);
    printf("\"}\n");
}
static void source_payload(const char *stage, const unsigned char *p, size_t count) {
    printf("{\"schema\":1,\"stage\":\"%s\",\"source_bytes\":%zu,\"source_hex\":\"", stage, count);
    hex(p, count);
    printf("\"}\n");
}
static void hold(void) {
    unsigned char token; ssize_t n;
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 1 || token != 'G') fail(75);
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 0) fail(75);
}
int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc != 6) return 64;
    int provided = integer(argv[3], -1, CAPACITY);
    int flag = integer(argv[4], 0, 1), use_null = integer(argv[5], 0, 1);
    unsigned char expected[CAPACITY];
    size_t source_bytes = read_source(argv[2], expected);
    if (provided > 0 && (size_t)provided > source_bytes) return 64;
    unsigned char *source_owner = allocate(source_bytes), *source = source_owner + GUARD;
    unsigned char *object_owner = allocate(OBJECT); void *object = object_owner + GUARD;
    memcpy(source, expected, source_bytes);
    printf("{\"schema\":1,\"stage\":\"before-load\"}\n");
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "dlopen: %.1024s\n", dlerror()); return 70; }
    printf("{\"schema\":1,\"stage\":\"loaded\"}\n");
    void *c1 = bound(library, "_ZN9ModemDataC1EPKcib", 0xc030);
    void *c2 = bound(library, "_ZN9ModemDataC2EPKcib", 0xc030);
    named_d1 = bound(library, "_ZN9ModemDataD1Ev", 0xc150);
    void *d2 = bound(library, "_ZN9ModemDataD2Ev", 0xc150);
    vtable = bound(library, "_ZTV9ModemData", 0x10008);
    if (c1 != c2 || named_d1 != d2) return 71;
    bindings(c1, c2, named_d1, d2);
    printf("{\"schema\":1,\"stage\":\"before-constructor\",\"source_bytes\":%zu,\"provided_signed_count\":%d,\"opaque_bool_argument\":%d,\"source_pointer_null\":%s,\"source_hex\":\"", source_bytes, provided, flag, use_null ? "true" : "false");
    hex(source, source_bytes); printf("\"}\n");
    ((constructor)c1)(object, use_null ? NULL : (const char *)source, provided, (bool)flag);
    bool active = !use_null && provided > 0;
    size_t copied = active ? (size_t)provided : 0;
    const unsigned char *owned = (const unsigned char *)pointer(object, 8);
    if (!guards(source_owner, source_bytes) || !guards(object_owner, OBJECT) ||
        memcmp(source, expected, source_bytes) || !vptr_and_d1(object) ||
        count_of(object) != (int32_t)copied || flag_of(object) != (active ? (unsigned)flag : 0) ||
        (active ? (!owned || !disjoint_ranges(owned, copied, source_owner, source_bytes + 2 * GUARD) ||
                   memcmp(owned, expected, copied)) : owned != NULL)) return 72;
    printf("{\"schema\":1,\"stage\":\"constructed-layout\",\"observed_signed_count\":%d,\"observed_opaque_bool_byte\":%u,\"data_pointer_nonnull\":%s,\"data_pointer_distinct_from_source\":%s,\"qualified_vptr_and_named_D1_bound\":true,\"source_unchanged\":true,\"input_and_object_guards_intact\":true}\n", count_of(object), flag_of(object), owned ? "true" : "false", active ? "true" : "false");
    printf("{\"schema\":1,\"stage\":\"constructed-ranges\",\"input_allocation_pointer_u64\":%" PRIuPTR ",\"input_allocation_bytes\":%zu,\"input_payload_pointer_u64\":%" PRIuPTR ",\"constructor_source_argument_pointer_u64\":%" PRIuPTR ",\"receiver_pointer_u64\":%" PRIuPTR ",\"owned_pointer_u64\":%" PRIuPTR ",\"owned_provided_range_bytes\":%zu,\"owned_range_disjoint_from_live_input_allocation\":%s,\"range_end_overflow_rejected\":true}\n",
           (uintptr_t)source_owner, source_bytes + 2 * GUARD, (uintptr_t)source,
           use_null ? (uintptr_t)0 : (uintptr_t)source, (uintptr_t)object,
           (uintptr_t)owned, copied, active ? "true" : "null");
    source_payload("constructed-source", source, source_bytes);
    payload("constructed", owned, copied);
    for (size_t i = 0; i < source_bytes; ++i) source[i] ^= 0x5a;
    if (!guards(source_owner, source_bytes) || !guards(object_owner, OBJECT) ||
        (copied && memcmp(owned, expected, copied))) return 72;
    for (size_t i = 0; i < source_bytes; ++i)
        if (source[i] != (unsigned char)(expected[i] ^ 0x5a)) return 72;
    source_payload("source-overwrite-input", source, source_bytes);
    payload("source-overwritten", owned, copied);
    if (!guards(source_owner, source_bytes)) return 72;
    free(source_owner); source_owner = NULL; source = NULL;
    if (!guards(object_owner, OBJECT) || (copied && memcmp(owned, expected, copied))) return 72;
    payload("source-freed", owned, copied);
    printf("{\"schema\":1,\"stage\":\"held\",\"own_input_outer_free_completed\":true,\"input_guards_checked_before_free\":true,\"no_post_free_input_read\":true}\n");
    hold();
    ((destructor)named_d1)(object);
    if (!guards(object_owner, OBJECT) || !vptr_and_d1(object) || pointer(object, 8) || count_of(object) || flag_of(object)) return 73;
    free(object_owner);
    printf("{\"schema\":1,\"stage\":\"destructed\",\"original_named_D1_returned\":true,\"data_pointer_null\":true,\"signed_count_zero\":true,\"opaque_bool_zero\":true,\"object_guards_checked_before_outer_free\":true,\"own_object_outer_free_completed\":true}\n");
    if (dlclose(library)) return 74;
    printf("{\"schema\":1,\"stage\":\"closed\"}\n");
    return 0;
}
