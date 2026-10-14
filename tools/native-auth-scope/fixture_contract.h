#ifndef POKELINUX_PAM_OWNERSHIP_FIXTURE_CONTRACT_H
#define POKELINUX_PAM_OWNERSHIP_FIXTURE_CONTRACT_H
/* Authored synthetic fixture only. No credentials, PAM messages or result IPC. */
#include <stddef.h>
#include <stdint.h>

enum pokelinux_pam_fixture_role {
    POKELINUX_PAM_WORKER_BLOCKED = 1,
    POKELINUX_PAM_SETSID_CHILD_BLOCKED = 2,
    POKELINUX_PAM_GRANDCHILD_BLOCKED = 3,
    POKELINUX_PAM_AUTH_RETURNING_SUCCESS = 4,
    POKELINUX_PAM_END_CLEANUP_HANG = 5,
    /* Only the Rust isolated worker hook may emit this after its actual typed
       candidate send succeeds. The PAM module cannot assert this event. */
    POKELINUX_PAM_TYPED_SUCCESS_SENT_HANG = 6
};

/* Exactly one atomic pipe write. The selected Arch ARM and x86 hosts are LE;
   consumers must explicitly decode LE, never cast an unaligned byte buffer. */
struct pokelinux_pam_fixture_record {
    unsigned char magic[8]; /* {'P','A','M','O','W','N','1',0} */
    uint32_t version;       /* 1 */
    uint32_t role;
    uint32_t pid;
    uint32_t ppid;
    uint32_t sid;
    uint32_t pgid;
    uint64_t observed_at_boottime_ns;
};
_Static_assert(sizeof(struct pokelinux_pam_fixture_record) == 40, "wire size");
_Static_assert(offsetof(struct pokelinux_pam_fixture_record, observed_at_boottime_ns) == 32,
               "wire timestamp offset");

#endif
