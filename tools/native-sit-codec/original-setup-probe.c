#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Authored isolated comparison helper. The caller's original allocation is
 * 11000 bytes; this does not claim a stable public vendor C++ ABI. */
enum { CONTEXT_BYTES = 11000, GUARD_BYTES = 64, NAME_BYTES = 21 };
typedef void (*context_ctor)(void *, int, const char *, int);
typedef void (*context_dtor)(void *);
static int integer(const char *s, int *out) {
    char *end = NULL;
    errno = 0;
    long value = strtol(s, &end, 10);
    if (errno || !end || end == s || *end || value < INT_MIN || value > INT_MAX) return -1;
    *out = (int)value;
    return 0;
}
static int guard_ok(const unsigned char *allocation) {
    for (size_t i = 0; i < GUARD_BYTES; ++i)
        if (allocation[i] != 0xa5 || allocation[GUARD_BYTES + CONTEXT_BYTES + i] != 0xa5) return 0;
    return 1;
}
static void hold(void) {
    unsigned char token = 0;
    ssize_t n;
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 1 || token != 'G') _exit(75);
    do { n = read(STDIN_FILENO, &token, 1); } while (n < 0 && errno == EINTR);
    if (n != 0) _exit(75);
}

typedef void *(*apn_new)(const char *, const char *, int, const char *, const char *, const char *, const char *, int);
typedef void (*set_apn)(void *, void *);
typedef void (*builder_ctor)(void *);
typedef void *(*setup_call)(void *, int, void *, int, int, const char *, int, int, int, int, int, int, const void *, bool, bool);
static void *symbol(void *library, const char *name, uintptr_t offset) {
    void *address = dlsym(library, name);
    Dl_info info;
    if (!address || !dladdr(address, &info) || (uintptr_t)address - (uintptr_t)info.dli_fbase != offset) _exit(71);
    return address;
}
int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc != 8) return 64;
    int cid, index, carrier, format;
    if (integer(argv[2], &cid) || integer(argv[3], &index) || integer(argv[6], &carrier) || integer(argv[7], &format)) return 64;
    if (cid < 1 || cid > 30 || index < 0 || index > 99 || carrier < 0 || (format != 1 && format != 2)) return 64;
    if (strcmp(argv[4], "IP") && strcmp(argv[4], "IPV6") && strcmp(argv[4], "IPV4V6") && strcmp(argv[4], "NON-IP")) return 64;
    if (!*argv[5] || strlen(argv[5]) > 99) return 64;
    for (const char *s = argv[5]; *s; ++s)
        if (!((*s >= 'a' && *s <= 'z') || (*s >= '0' && *s <= '9') || *s == '.' || *s == '-')) return 64;
    printf("{\"schema\":1,\"stage\":\"before-load\"}\n");
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "dlopen: %.1024s\n", dlerror()); return 70; }
    printf("{\"schema\":1,\"stage\":\"loaded\"}\n");
    context_ctor construct = (context_ctor)symbol(library, "_ZN10PdpContextC1EiPKci", 0x4d230);
    context_dtor destruct = (context_dtor)symbol(library, "_ZN10PdpContextD1Ev", 0x4df20);
    apn_new make_apn = (apn_new)symbol(library, "_ZN10ApnSetting11NewInstanceEPKcS1_iS1_S1_S1_S1_i", 0x31fe0);
    set_apn attach_apn = (set_apn)symbol(library, "_ZN10PdpContext13SetApnSettingEP10ApnSetting", 0x4e820);
    builder_ctor make_builder = (builder_ctor)symbol(library, "_ZN15ProtocolBuilderC1Ev", 0xf4e0);
    setup_call setup = (setup_call)symbol(library, "_ZN17ProtocolPsBuilder18BuildSetupDataCallEiP10PdpContextiiPKciiiiiiPK21RIL_TrafficDescriptorbb", 0x7ba30);
    context_dtor delete_packet = (context_dtor)symbol(library, "_ZN9ModemDataD0Ev", 0xc1e0);
    void *vtable = symbol(library, "_ZTV17ProtocolPsBuilder", 0x8eb08);
    if (!vtable) return 71;
    unsigned char *allocation = malloc(CONTEXT_BYTES + 2 * GUARD_BYTES);
    if (!allocation) return 71;
    memset(allocation, 0xa5, CONTEXT_BYTES + 2 * GUARD_BYTES);
    void *context = allocation + GUARD_BYTES;
    memset(context, 0, CONTEXT_BYTES);
    construct(context, cid, "rmnet", index);
    void *apn = make_apn("00101", argv[5], 1, "", "", argv[4], argv[4], 0);
    if (!apn) return 71;
    attach_apn(context, apn); /* Ownership transfer to the real original context. */
    int observed_cid, context_type;
    memcpy(&observed_cid, context, sizeof observed_cid);
    memcpy(&context_type, (const char *)context + 0x30, sizeof context_type);
    if (observed_cid != cid || context_type != 0 || !guard_ok(allocation)) return 72;
    unsigned char builder[24] __attribute__((aligned(16)));
    memset(builder, 0, sizeof builder);
    make_builder(builder + 8);
    uintptr_t address_point = (uintptr_t)vtable + 16;
    memcpy(builder, &address_point, sizeof address_point);
    memcpy(builder + 16, &carrier, sizeof carrier);
    memcpy(builder + 20, &format, sizeof format);
    printf("{\"schema\":1,\"stage\":\"before-serializer\",\"context_cid\":%d,\"ifname\":\"%s\",\"context_type\":%d,\"carrier\":%d,\"format_flag\":%d,\"radio_arg\":14,\"other_scalar_args_zero\":true,\"TD_null\":true,\"boolean_args_false\":true}\n", cid, (const char *)context + 4, context_type, carrier, format);
    void *packet = setup(builder, 14, context, 0, 0, NULL, 0, 0, 0, 0, 0, 0, NULL, false, false);
    if (!packet) return 76;
    const unsigned char *bytes;
    int length;
    memcpy(&bytes, (const char *)packet + 8, sizeof bytes);
    memcpy(&length, (const char *)packet + 16, sizeof length);
    if (!bytes || length < 12 || length > 2048 || !guard_ok(allocation)) return 77;
    uintptr_t controller;
    memcpy(&controller, (const char *)context + 0x2ab0, sizeof controller);
    if (controller) return 77;
    printf("{\"schema\":1,\"stage\":\"serialized\",\"bytes\":%d,\"hex\":\"", length);
    for (int i = 0; i < length; ++i) printf("%02x", bytes[i]);
    printf("\",\"context_guards_intact\":true,\"netif_controller_null\":true}\n");
    printf("{\"schema\":1,\"stage\":\"held\"}\n");
    hold();
    delete_packet(packet);
    destruct(context);
    if (!guard_ok(allocation)) return 73;
    free(allocation);
    printf("{\"schema\":1,\"stage\":\"destructed\",\"guards_intact\":true}\n");
    if (dlclose(library)) return 74;
    printf("{\"schema\":1,\"stage\":\"closed\"}\n");
    return 0;
}
