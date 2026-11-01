# Selected VM seat dependency

This optional profile supplies standard seatd for the disposable ARM graphical
VM. Install `pokelinux-vm-seatd.service` into `/usr/lib/systemd/system/` and
`session-owner-seatd.conf` as
`/usr/lib/systemd/system/pokelinux-session-owner.service.d/63-vm-seatd.conf`.
Enable the selected service offline with the normal session target; disable the
unselected stock `seatd.service`. This profile is not selected by the normal
installer and makes no phone or physical-seat claim.

The actual installed cold7 compositor failed to acquire a seat: its retained
crash report records the missing `/run/seatd.sock`, no primary logind session,
then DRM/allocator startup failure. An earlier separate ARM graphical fixture
ran `/usr/bin/seatd -g alarm` with `SEATD_VTBOUND=0`; the compositor at primary
UID/GID1000 opened seat0. That earlier fixture was not the complete installed
session. The new installed run now reaches active seatd and session-owner units,
with matching PID1 exit0 observations after Stop. The observer fails on valid
repeated `Environment=` directives before UI actions and full seat binding, so
complete identity/socket/Stop and functional qualification remain open. See
[the partial startup evidence](../../../docs/evidence/native-installed-vm-seat-startup.json).

The daemon keeps UID/GID0. Its `-g alarm` argument changes the socket group,
not the daemon credentials. The selected image must verify `alarm` is GID1000,
retain the actual `/usr/bin/seatd` ELF and loader identity, and independently
observe `/run/seatd.sock` as a UNIX stream socket owned by UID0/GID1000 with
mode0770. Seatd0.9.3 explicitly applies that mode when assigning a socket group.
No supplementary groups or compositor/owner arguments change.

`Type=exec` is followed by the existing `pokelinux-audio-backend-ready` helper.
Its original five-second BOOTTIME bound, nonblocking connection and exact
SO_PEERCRED PID/UID/GID check bind the socket to this actual root MainPID before
the owner can start. This is listener readiness, not Source, seat-allocation or
window readiness. The owner dependency also reverses Stop ordering: the owner
and its compositor retire before seatd. PartOf follows normal-session Stop.
`Restart=no` prevents automatic replacement; an existing socket refuses startup.

The installed consumer must retain the actual service invocation, Main pidfd,
executable, credentials and cgroup and include this service in normal Stop and
journal collection. Normal seatd SIGTERM handling exits0 after client/seat
cleanup and native socket unlink; require the actual terminal/empty-cgroup and
socket-absence evidence. No ExecStopPost deletion substitutes for native cleanup.
The existing compositor-before-seatd order and original aggregate deadlines stay
in force. The VM preparation only installs and checks files; it does not start a
seat, compositor, GUI, permission action or connection.

Upstream implementation:
[seatd0.9.3](https://github.com/kennylevinsen/seatd/blob/0.9.3/seatd/seatd.c),
[seat lifecycle](https://github.com/kennylevinsen/seatd/blob/0.9.3/seatd/server.c).
