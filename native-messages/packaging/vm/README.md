# Explicit Messages VM history owner

Select the production `pokelinux-messages.service` and normal feature-empty
`messages-service`/`messages-stop` through the finite authenticated
`install-native-services` manifest. The daemon takes exactly three absolute
arguments: private runtime, raw receipts directory and Inbox directory. The unit
creates root:root0700 runtime/state directories. It owns the sole Projector and
has no app profile, lifecycle grant store or standalone app endpoint. No Binder
dependency or receiving selector is installed.

The graphical session independently selects
`{"mode":"owner","socket":"/run/pokelinux/messages/read-provider.sock"}`,
its fixed UID11010 app profile and normal shared appearance QML. Start the daemon
independently before that session; selection adds no service start, retry or
permission grant. Only the daemon knows database paths. Its private root peer
returns bounded authenticated copied history; the session actor checks current
app permission, principal, Source and the original deadline before publication.
Session cancellation/exit closes local reads without stopping background history.

The separate `messages-seed-fixture` is built only with `isolated-fixture`, kept
under an explicit fixture path and invoked once during disposable image
preparation with `NATIVE_MESSAGES_ISOLATED=1`. It precommits two authored
unqualified raw GSM occurrences. The production service is never built with that
feature. Require the Inbox database to remain absent and the Messages unit
stopped before retaining the prepared image.

The current installed-service probe now passes one fresh generic ARM/PID1 cold
boot with the normal three-argument Main, sole writer/readonly pair, and exact
private State/Read history. This upgrade retains the original raw seed without
reseeding. Preparation keeps the old unit masked with no Main or Inbox, stages
exactly service/Stop/unit files and removes only the pinned obsolete app profile.
The 353 package versions, 50 Qt files and audio/appearance pair stay unchanged.

Typed Stop records helper 0, Main 0, inactive/dead state, empty cgroup and dead
retained pidfds. Preparation and cold1 both close QEMU 0, owned waits/EOFs and
exact container removal on the same daemon. See
[the current owner result](../../../docs/native-installed-messages-history.md#current-owner-integration).
The earlier two cold boots remain historical six-argument evidence. A second new
boot/reboot, session app/UI, normal Source, incoming SMS and hardware remain open.
The next x86 preview still has a separate cold Source `sample_gap` failure exceeding its
100 ms limit; the 50 ms connectivity guard is unchanged.
