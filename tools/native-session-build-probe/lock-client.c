#include <wayland-client.h>
#include "session-lock-client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <poll.h>
#include <unistd.h>
static struct ext_session_lock_manager_v1 *manager;
static struct ext_session_lock_v1 *lock;
static int locked, done;
static volatile sig_atomic_t release_requested;
static void sig_release(int sig) { release_requested = 1; }
static void global(void *data, struct wl_registry *reg, uint32_t name, const char *iface, uint32_t version) {
    if (!strcmp(iface, ext_session_lock_manager_v1_interface.name)) manager = wl_registry_bind(reg, name, &ext_session_lock_manager_v1_interface, 1);
}
static void global_remove(void *data, struct wl_registry *reg, uint32_t name) {}
static const struct wl_registry_listener registry_listener = {global, global_remove};
static void on_locked(void *data, struct ext_session_lock_v1 *p) { locked = 1; fprintf(stderr,"LOCKED\n"); fflush(stderr); }
static void on_finished(void *data, struct ext_session_lock_v1 *p) { done = 1; fprintf(stderr,"FINISHED\n"); fflush(stderr); }
static const struct ext_session_lock_v1_listener lock_listener = {on_locked, on_finished};
int main(void) {
    signal(SIGUSR1, sig_release);
    struct wl_display *d = wl_display_connect(NULL);
    if (!d) return 2;
    struct wl_registry *reg = wl_display_get_registry(d);
    wl_registry_add_listener(reg, &registry_listener, NULL);
    if (wl_display_roundtrip(d) < 0 || !manager) return 3;
    lock = ext_session_lock_manager_v1_lock(manager);
    ext_session_lock_v1_add_listener(lock, &lock_listener, NULL);
    wl_display_flush(d);
    fprintf(stderr,"REQUESTED pid=%d\n",getpid()); fflush(stderr);
    while (!done) {
        if (release_requested && locked) { ext_session_lock_v1_unlock_and_destroy(lock); wl_display_roundtrip(d); break; }
        if (wl_display_prepare_read(d) != 0) { if (wl_display_dispatch_pending(d) < 0) break; continue; }
        wl_display_flush(d);
        struct pollfd fd = {wl_display_get_fd(d), POLLIN, 0};
        int r = poll(&fd, 1, 100);
        if (r > 0) { if (wl_display_read_events(d) < 0 || wl_display_dispatch_pending(d) < 0) break; }
        else wl_display_cancel_read(d);
    }
    wl_display_disconnect(d);
    return 0;
}
