#define _GNU_SOURCE
/* Synthetic Linux-PAM public module entrypoints. This is not authentication.
   Load only from a separate root-owned test service in a disposable guest. */
#include <security/pam_modules.h>
#include "fixture_contract.h"
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

struct cleanup_state { int ready_fd; };

static int pipe_argument(const char *text) {
    static const char prefix[] = "ready_fd=";
    if (!text || strncmp(text, prefix, sizeof(prefix) - 1)) return -1;
    const unsigned char *at = (const unsigned char *)text + sizeof(prefix) - 1;
    if (!*at) return -1;
    unsigned int value = 0;
    for (; *at; ++at) {
        if (*at < '0' || *at > '9' || value > (unsigned int)(INT_MAX - (*at - '0')) / 10U)
            return -1;
        value = value * 10U + (*at - '0');
    }
    /* The fixed isolated worker retains stdio only. FD 2 is the owned,
       credential-free readiness pipe; no caller-selected extra FD is allowed. */
    if (value != 2U) return -1;
    const int fd = (int)value;
    struct stat metadata;
    const int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_WRONLY || !(flags & O_NONBLOCK)
        || fstat(fd, &metadata) || !S_ISFIFO(metadata.st_mode)
        || fpathconf(fd, _PC_PIPE_BUF) < (long)sizeof(struct pokelinux_pam_fixture_record))
        return -1;
    return fd;
}

static int record_ready(int fd, uint32_t role) {
    struct timespec ts;
    if (clock_gettime(CLOCK_BOOTTIME, &ts) || ts.tv_sec < 0 || ts.tv_nsec < 0
        || ts.tv_nsec >= 1000000000L) return -1;
    if ((uint64_t)ts.tv_sec > (UINT64_MAX - (uint64_t)ts.tv_nsec) / 1000000000ULL) return -1;
    const pid_t pid = getpid(), ppid = getppid(), sid = getsid(0), pgid = getpgrp();
    if (pid <= 0 || ppid <= 0 || sid <= 0 || pgid <= 0) return -1;
    const struct pokelinux_pam_fixture_record record = {
        .magic = {'P','A','M','O','W','N','1',0}, .version = 1, .role = role,
        .pid = (uint32_t)pid, .ppid = (uint32_t)ppid,
        .sid = (uint32_t)sid, .pgid = (uint32_t)pgid,
        .observed_at_boottime_ns = (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec
    };
    ssize_t result;
    do { result = write(fd, &record, sizeof(record)); } while (result < 0 && errno == EINTR);
    return result == (ssize_t)sizeof(record) ? 0 : -1;
}

static _Noreturn void block_until_cgroup_kill(void) {
    /* Termination through the worker's PID or process group cannot stand in
       for containing its session-detached descendants. SIGKILL remains unblocked. */
    sigset_t signals;
    if (sigfillset(&signals) || sigprocmask(SIG_SETMASK, &signals, NULL)) _exit(90);
    for (;;) pause();
}

static void cleanup_hang(pam_handle_t *handle, void *data, int status) {
    (void)handle; (void)status;
    struct cleanup_state *state = data;
    if (!state || record_ready(state->ready_fd, POKELINUX_PAM_END_CLEANUP_HANG)) _exit(91);
    close(state->ready_fd);
    /* Deliberately never free/return: only this synthetic cleanup callback hangs. */
    block_until_cgroup_kill();
}

PAM_EXTERN int pam_sm_authenticate(pam_handle_t *handle, int flags, int argc, const char **argv) {
    (void)flags;
    if (!handle || argc != 2 || !argv || !argv[0] || !argv[1]) return PAM_SERVICE_ERR;
    const int fd = pipe_argument(argv[1]);
    if (fd < 0) return PAM_SERVICE_ERR;
    if (!strcmp(argv[0], "mode=success-cleanup-hang")) {
        struct cleanup_state *state = malloc(sizeof(*state));
        if (!state) return PAM_BUF_ERR;
        state->ready_fd = fd;
        const int status = pam_set_data(handle, "pokelinux-auth-ownership-cleanup", state, cleanup_hang);
        if (status != PAM_SUCCESS) { free(state); return status; }
        if (record_ready(fd, POKELINUX_PAM_AUTH_RETURNING_SUCCESS)) return PAM_SYSTEM_ERR;
        return PAM_SUCCESS;
    }
    if (strcmp(argv[0], "mode=descendant-block")) return PAM_SERVICE_ERR;
    const pid_t child = fork();
    if (child < 0) return PAM_SYSTEM_ERR;
    if (child == 0) {
        if (setsid() < 0) _exit(92);
        const pid_t grandchild = fork();
        if (grandchild < 0) _exit(93);
        if (grandchild == 0) {
            if (record_ready(fd, POKELINUX_PAM_GRANDCHILD_BLOCKED)) _exit(94);
            close(fd);
            block_until_cgroup_kill();
        }
        if (record_ready(fd, POKELINUX_PAM_SETSID_CHILD_BLOCKED)) _exit(95);
        close(fd);
        block_until_cgroup_kill();
    }
    if (record_ready(fd, POKELINUX_PAM_WORKER_BLOCKED)) return PAM_SYSTEM_ERR;
    close(fd);
    block_until_cgroup_kill();
}

PAM_EXTERN int pam_sm_setcred(pam_handle_t *handle, int flags, int argc, const char **argv) {
    (void)handle; (void)flags; (void)argc; (void)argv;
    return PAM_SUCCESS;
}
