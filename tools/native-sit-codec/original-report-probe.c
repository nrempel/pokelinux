#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdbool.h>
#include <limits.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Authored isolated comparison helper; opaque allocations are pinned to one
 * matched original binary, not a public C++ ABI. Direct memory-only context update; no service/activation/transport calls. */
enum { GUARD = 64, PACKET = 24, ADAPTER = 10912, ADDRESS = 193, RESPONSE = 144, CONTEXT = 11000, PREFIX = 0x160, BUILDER = 16, REPORT = 0x1478 };
typedef void (*packet_ctor)(void *, const char *, int, bool);
typedef void (*adapter_ctor)(void *, const void *);
typedef void (*destructor)(void *);
typedef void (*context_ctor)(void *, int, const char *, int);
typedef uint8_t (*context_update)(void *, const void *);
typedef void (*report_builder_ctor)(void *, int);
typedef void *(*report_builder)(void *, int, void *);
typedef const void *(*data_getter)(const void *);
typedef int32_t (*length_getter)(const void *);
typedef int32_t (*error_getter)(const void *);
typedef int32_t (*address_getter)(const void *, void *, int);
static unsigned char *owned(size_t bytes) {
    unsigned char *p = malloc(bytes + 2 * GUARD);
    if (!p) _exit(71);
    memset(p, 0xa5, bytes + 2 * GUARD);
    memset(p + GUARD, 0, bytes);
    if ((uintptr_t)(p + GUARD) % 16) _exit(71);
    return p;
}
static int guarded(const unsigned char *p, size_t bytes) {
    for (size_t i = 0; i < GUARD; ++i)
        if (p[i] != 0xa5 || p[GUARD + bytes + i] != 0xa5) return 0;
    return 1;
}
static void *symbol(void *library, const char *name, uintptr_t offset) {
    void *p = dlsym(library, name);
    Dl_info info;
    if (!p || !dladdr(p, &info) || (uintptr_t)p - (uintptr_t)info.dli_fbase != offset) _exit(71);
    return p;
}
static int nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}
static uint32_t word(const void *p, size_t offset) {
    uint32_t value;
    memcpy(&value, (const char *)p + offset, sizeof value);
    return value;
}
static void hex(const void *bytes, size_t count) {
    const unsigned char *p = bytes;
    for (size_t i = 0; i < count; ++i) printf("%02x", p[i]);
}
static void hold(void) {
    unsigned char token;
    ssize_t n;
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 1 || token != 'G') _exit(75);
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 0) _exit(75);
}
static int number(const char *text, int low, int high) {
    errno = 0; char *end;
    long n = strtol(text, &end, 10);
    if (errno || end == text || *end || n < low || n > high) _exit(64);
    return (int)n;
}
static int controller_null(const void *context) {
    for (size_t i = 0; i < 3; ++i) {
        const size_t offsets[] = { 0x20, 0x28, 0x2ab0 };
        uintptr_t value; memcpy(&value, (const char *)context + offsets[i], sizeof value);
        if (value) return 0;
    }
    return 1;
}
static uintptr_t pointer(const void *p, size_t offset) {
    uintptr_t value; memcpy(&value, (const char *)p + offset, sizeof value); return value;
}
static int64_t signed64(const void *p, size_t offset) {
    int64_t value; memcpy(&value, (const char *)p + offset, sizeof value); return value;
}
static void text(const char *p, size_t bound) {
    if (!p) { printf("null"); return; }
    size_t length = strnlen(p, bound);
    if (length == bound) _exit(77);
    for (size_t i = 0; i < length; ++i)
        if ((unsigned char)p[i] < 32 || (unsigned char)p[i] > 126 || p[i] == '"' || p[i] == 92) _exit(77);
    printf("\"%.*s\"", (int)length, p);
}
static void string_vector(const void *p, size_t offset) {
    uintptr_t begin = pointer(p, offset), end = pointer(p, offset + 8), cap = pointer(p, offset + 16);
    if (end < begin || cap < end || (end - begin) % 24 || (end - begin) / 24 > 16 || (!begin && (end || cap))) _exit(77);
    printf("[");
    for (size_t i = 0; i < (end - begin) / 24; ++i) {
        const unsigned char *item = (const unsigned char *)(begin + 24 * i);
        const char *chars; size_t length;
        if (item[0] & 1) {
            size_t capacity = pointer(item, 0) & ~(size_t)1;
            length = pointer(item, 8); chars = (const char *)pointer(item, 16);
            if (!chars || length > 128 || capacity <= length || capacity > 4096) _exit(77);
        } else {
            length = item[0] >> 1; chars = (const char *)item + 1;
            if (length > 22) _exit(77);
        }
        if (chars[length] || strnlen(chars, length + 1) != length) _exit(77);
        if (i) printf(","); text(chars, length + 1);
    }
    printf("]");
}
static void offset_is(void *function, uintptr_t expected) {
    Dl_info info;
    if (!function || !dladdr(function, &info) || (uintptr_t)function - (uintptr_t)info.dli_fbase != expected) _exit(72);
}
int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc != 6 || strlen(argv[2]) != RESPONSE * 2) return 64;
    int context_cid = number(argv[3], 1, 30), context_index = number(argv[4], 0, 99), use_null = number(argv[5], 0, 1);
    unsigned char input[RESPONSE];
    for (size_t i = 0; i < RESPONSE; ++i) {
        int hi = nibble(argv[2][i * 2]), lo = nibble(argv[2][i * 2 + 1]);
        if (hi < 0 || lo < 0) return 64;
        input[i] = (unsigned char)(hi * 16 + lo);
    }
    printf("{\"schema\":1,\"stage\":\"before-load\"}\n");
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "dlopen: %.1024s\n", dlerror()); return 70; }
    printf("{\"schema\":1,\"stage\":\"loaded\"}\n");
    packet_ctor make_packet = (packet_ctor)symbol(library, "_ZN9ModemDataC1EPKcib", 0xc030);
    destructor destroy_packet = (destructor)symbol(library, "_ZN9ModemDataD1Ev", 0xc150);
    adapter_ctor make_adapter = (adapter_ctor)symbol(library, "_ZN30ProtocolPsSetupDataCallAdapterC1EPK9ModemData", 0x54a70);
    error_getter get_error = (error_getter)symbol(library, "_ZNK30ProtocolPsSetupDataCallAdapter12GetErrorCodeEv", 0x57490);
    address_getter get_address = (address_getter)symbol(library, "_ZNK30ProtocolPsSetupDataCallAdapter11GetAddrInfoEP8PDP_ADDRi", 0x574d0);
    void *vtable = symbol(library, "_ZTV30ProtocolPsSetupDataCallAdapter", 0x8d4b0);
    unsigned char *pa = owned(PACKET), *aa = owned(ADAPTER), *v4 = owned(ADDRESS), *v6 = owned(ADDRESS);
    void *packet = pa + GUARD, *adapter = aa + GUARD;
    make_packet(packet, (const char *)input, RESPONSE, true);
    const unsigned char *copy;
    memcpy(&copy, (const char *)packet + 8, sizeof copy);
    if (!copy || copy == input || word(packet, 16) != RESPONSE || memcmp(copy, input, RESPONSE) || !guarded(pa, PACKET)) return 72;
    printf("{\"schema\":1,\"stage\":\"before-parser\",\"bytes\":144,\"input_hex\":\"");
    hex(input, sizeof input);
    printf("\",\"owned_packet_copy\":true,\"packet_guards_intact\":true}\n");
    make_adapter(adapter, packet); /* Original C1 calls Init exactly once. */
    uintptr_t vp;
    memcpy(&vp, adapter, sizeof vp);
    if (vp != (uintptr_t)vtable + 16) return 72;
    void *d1, *error_slot;
    memcpy(&d1, (const void *)vp, sizeof d1);
    memcpy(&error_slot, (const void *)(vp + 0x48), sizeof error_slot);
    Dl_info info;
    if (!dladdr(d1, &info) || (uintptr_t)d1 - (uintptr_t)info.dli_fbase != 0x62540 || error_slot != (void *)get_error) return 72;
    int32_t error = get_error(adapter);
    int32_t r4 = get_address(adapter, v4 + GUARD, 1);
    int32_t r6 = get_address(adapter, v6 + GUARD, 2);
    if (!guarded(pa, PACKET) || !guarded(aa, ADAPTER) || !guarded(v4, ADDRESS) || !guarded(v6, ADDRESS) || r4 || r6) return 72;
    const unsigned char *data = (const unsigned char *)adapter + 0x40;
    printf("{\"schema\":1,\"stage\":\"parsed\",\"error\":%d,\"status\":%u,\"cid\":%u,\"active\":%u,\"pdp_selector\":%u,\"opaque_u32_at_127\":%u,\"opaque_u8_at_131\":%u,\"v4_return\":%d,\"v6_return\":%d,\"v4_hex\":\"", error, word(data, 0), word(data, 0x10), word(data, 0x14), word(data, 0x18), word(data, 0x124), word(data, 0x110), r4, r6);
    hex(v4 + GUARD, ADDRESS);
    printf("\",\"v6_hex\":\"");hex(v6 + GUARD, ADDRESS);
    printf("\",\"guards_intact\":true,\"vtable_and_D1_verified\":true,\"Init_calls_by_helper\":0}\n");
    context_ctor make_context = (context_ctor)symbol(library, "_ZN10PdpContextC1EiPKci", 0x4d230);
    destructor destroy_context = (destructor)symbol(library, "_ZN10PdpContextD1Ev", 0x4df20);
    context_update update = (context_update)symbol(library, "_ZN10PdpContext18UpdateDataCallInfoEPK8DataCall", 0x4e8e0);
    unsigned char *ca = owned(CONTEXT); void *context = ca + GUARD;
    make_context(context, context_cid, "rmnet", context_index);
    unsigned char before[PREFIX], source[PREFIX], first[PREFIX];
    memcpy(before, (const char *)context + 0x38, PREFIX); memcpy(source, data, PREFIX);
    if (!guarded(ca, CONTEXT) || !controller_null(context) || word(context, 0x34) != 0) return 72;
    printf("{\"schema\":1,\"stage\":\"before-context-update\",\"context_cid\":%u,\"ifname\":\"%.20s\",\"context_state\":%u,\"controller_null\":true,\"APN_TD_pointers_null\":true,\"use_null\":%d,\"prefix_hex\":\"", word(context, 0), (const char *)context + 4, word(context, 0x34), use_null);
    hex(before, PREFIX); printf("\",\"source_prefix_hex\":\"");hex(source, PREFIX);printf("\"}\n");
    uint8_t changed = update(context, use_null ? NULL : data);
    memcpy(first, (const char *)context + 0x38, PREFIX);
    if (!guarded(ca, CONTEXT) || !guarded(aa, ADAPTER) || !controller_null(context) || memcmp(source, data, PREFIX)) return 72;
    printf("{\"schema\":1,\"stage\":\"context-updated\",\"address_change_bit\":%u,\"context_cid\":%u,\"context_state\":%u,\"controller_null\":true,\"APN_TD_pointers_null\":true,\"prefix_changed\":%s,\"prefix_matches_source\":%s,\"prefix_hex\":\"", changed, word(context,0), word(context,0x34), memcmp(before, first, PREFIX) ? "true" : "false", memcmp(source, first, PREFIX) ? "false" : "true");
    hex(first, PREFIX);printf("\"}\n");
    uint8_t changed_again = update(context, use_null ? NULL : data);
    if (!guarded(ca, CONTEXT) || !guarded(aa, ADAPTER) || !controller_null(context) || memcmp(source, data, PREFIX)) return 72;
    printf("{\"schema\":1,\"stage\":\"context-updated-twice\",\"address_change_bit\":%u,\"context_cid\":%u,\"context_state\":%u,\"controller_null\":true,\"APN_TD_pointers_null\":true,\"prefix_same_as_first\":%s,\"guards_intact\":true,\"source_prefix_unchanged\":true}\n", changed_again, word(context,0), word(context,0x34), memcmp(first, (const char *)context + 0x38, PREFIX) ? "false" : "true");
    if (word(data, 0x194) != 0 || pointer(data,0x160) || pointer(data,0x168) || pointer(data,0x170)) return 72;
    report_builder_ctor make_builder = (report_builder_ctor)symbol(library, "_ZN13PsDataBuilderC1Ei", 0x1e6970);
    report_builder build = (report_builder)symbol(library, "_ZN13PsDataBuilder26BuildSetupDataCallResponseEiP10PdpContext", 0x1e6e90);
    destructor destroy_builder = (destructor)symbol(library, "_ZN14RilDataBuilderD1Ev", 0x11db30);
    void *builder_vtable = symbol(library, "_ZTV13PsDataBuilder", 0x243fa8);
    void *report_vtable = symbol(library, "_ZTV20DataCallListResponse", 0x243ff8);
    unsigned char *ba = owned(BUILDER); void *builder = ba + GUARD;
    make_builder(builder, 22); /* Authored version22, original default order false. */
    if (pointer(builder,0) != (uintptr_t)builder_vtable + 16 || pointer((const void *)pointer(builder,0),0) != (uintptr_t)destroy_builder || word(builder,8) != 22 || ((const unsigned char *)builder)[12] || !guarded(ba,BUILDER)) return 72;
    printf("{\"schema\":1,\"stage\":\"before-report\",\"version\":22,\"status_override\":0,\"IPv4_first\":false,\"builder_D1_bound\":true,\"source_dynamic_QoS_TD_empty\":true}\n");
    void *report = build(builder, 0, context);
    if (!report || pointer(report,0) != (uintptr_t)report_vtable + 16 || word(report,0x13c8) != 1 || word(report,0x13cc) != 22 || ((const unsigned char *)report)[0x1473] || !guarded(ba,BUILDER) || !guarded(ca,CONTEXT) || !controller_null(context)) return 72;
    uintptr_t report_vptr = pointer(report,0);
    destructor delete_report = (destructor)pointer((const void *)report_vptr,8);
    data_getter get_data = (data_getter)pointer((const void *)report_vptr,0x10);
    length_getter get_length = (length_getter)pointer((const void *)report_vptr,0x18);
    offset_is((void *)delete_report,0x1e7b00); offset_is((void *)get_data,0x1e7b30); offset_is((void *)get_length,0x1e7b50);
    const void *result = get_data(report); int32_t result_bytes = get_length(report);
    if (result != (const char *)report + 0x450 || result_bytes != 0x108 || pointer(result,0x30) != pointer(result,0x38) || pointer(result,0xb8) != pointer(result,0xc0) || pointer(result,0xf0) != pointer(result,0xf8)) return 72;
    printf("{\"schema\":1,\"stage\":\"reported\",\"bytes\":%d,\"cause\":%d,\"opaque_i64_at_8\":%" PRId64 ",\"cid\":%d,\"active\":%d,\"protocol_code\":%d,\"mtu_v4\":%d,\"mtu_v6\":%d,\"ifname\":", result_bytes,(int32_t)word(result,0),signed64(result,8),(int32_t)word(result,0x10),(int32_t)word(result,0x14),(int32_t)word(result,0x18),(int32_t)word(result,0x90),(int32_t)word(result,0x94));
    text((const char *)pointer(result,0x20),256);printf(",\"address_text\":");text((const char *)pointer(result,0x28),256);
    printf(",\"dnses\":");string_vector(result,0x48);printf(",\"gateways\":");string_vector(result,0x60);printf(",\"pcscf\":");string_vector(result,0x78);
    printf(",\"structured_address_vector_empty\":true,\"QoS_TD_vectors_empty\":true,\"borrowed_interior_getter_verified\":true,\"guards_intact\":true}\n");
    printf("{\"schema\":1,\"stage\":\"held\"}\n");
    hold();
    delete_report(report); /* Original-new outer object: original deleting D0. */
    destroy_builder(builder); /* Verified non-deleting inherited D1 on guard interior. */
    if (!guarded(ba,BUILDER) || !guarded(ca,CONTEXT) || memcmp(first,(const char *)context+0x38,PREFIX) || memcmp(source,data,PREFIX)) return 73;
    free(ba);
    destroy_context(context);
    if (!guarded(ca, CONTEXT)) return 73;
    free(ca);
    ((destructor)d1)(adapter);
    destroy_packet(packet);
    if (!guarded(pa, PACKET) || !guarded(aa, ADAPTER) || !guarded(v4, ADDRESS) || !guarded(v6, ADDRESS)) return 73;
    free(aa); free(pa); free(v4); free(v6);
    printf("{\"schema\":1,\"stage\":\"destructed\",\"guards_intact\":true}\n");
    if (dlclose(library)) return 74;
    printf("{\"schema\":1,\"stage\":\"closed\"}\n");
    return 0;
}
