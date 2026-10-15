#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Authored isolated comparison helper; opaque allocations are pinned to one
 * matched original binary, not a public C++ ABI. No service/transport calls. */
enum { GUARD = 64, PACKET = 24, ADAPTER = 10912, ADDRESS = 193, RESPONSE = 144 };
typedef void (*packet_ctor)(void *, const char *, int, bool);
typedef void (*adapter_ctor)(void *, const void *);
typedef void (*destructor)(void *);
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
int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc != 3 || strlen(argv[2]) != RESPONSE * 2) return 64;
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
    printf("{\"schema\":1,\"stage\":\"held\"}\n");
    hold();
    ((destructor)d1)(adapter);
    destroy_packet(packet);
    if (!guarded(pa, PACKET) || !guarded(aa, ADAPTER) || !guarded(v4, ADDRESS) || !guarded(v6, ADDRESS)) return 73;
    free(aa); free(pa); free(v4); free(v6);
    printf("{\"schema\":1,\"stage\":\"destructed\",\"guards_intact\":true}\n");
    if (dlclose(library)) return 74;
    printf("{\"schema\":1,\"stage\":\"closed\"}\n");
    return 0;
}
