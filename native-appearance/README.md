# Native appearance owner

This Rust service publishes a bounded public palette from the desktop user's
private Omarchy theme. It supplies presentation only: an appearance revision
never grants focus, service access, permission, or lock authority.

The root-installed executable accepts
`--serve /run/pokelinux/appearance.sock /etc/pokelinux/appearance.json`.
The root-owned configuration contains exactly `home` and `runtime`, for example
`{"home":"/home/alarm","runtime":"/run/user/1000"}`. The selected desktop
UID/GID is1000. Executable, configuration and socket-directory ancestry must be
root owned and exclude group/other writes and symbolic links. This is an
experimental component; installed supervision and full app migration remain
acceptance gates, not completed deployment.

The public `appearance.sock` is a mode0666 Unix SEQPACKET endpoint. Its only
operation is `{"op":"snapshot"}`. Root-only mode0600
`appearance-control.sock` accepts status and Stop. Stop acknowledges a request;
successful process exit requires actual collector reap. Separate client capacity
and original500ms idle/reply budgets keep public idle clients from consuming
owner-control capacity.

Snapshots contain exactly `schema`, `owner`, `revision`, `until_ns`, `available`
and `palette`. The schema is1; the incarnation is32 lowercase hexadecimal
characters. `until_ns` is a canonical decimal string preserving exact u64
`CLOCK_BOOTTIME` nanoseconds. An available palette has exactly nine RGB fields:
background, foreground, surface, border, accent, muted, urgent, positive and ink.
Unavailable publication has `palette:null`. A cached color is not a current
lease. Queued replies are revalidated before every actual send.

Source traversal and parsing run in one owned UID1000 collector. Completion
requires actual packet PID/UID/GID, retained pidfd liveness, incarnation/token
and the original one-second BOOTTIME deadline. The collector holds the original
theme-set shared lock and selected directory generation through publication ACK.
This qualifies cooperating writers only. The owner retains a terminated worker
slot until actual wait; collection retry is read-only and cannot retry a radio
registration or imply vendor cleanup.

Input traversal uses bounded regular files, no-follow component opens, ownership
and mode checks, single-link checks and generation/lock identities. After
acquisition an architecture-checked default-deny seccomp filter rejects new
content opens, process creation, execution, io_uring and new networking. Metadata
syscalls remain available. This is not whole-filesystem privacy, and the matched
Pixel kernel's capability behavior is unqualified.

An exclusive root-owned filesystem lock protects socket-node recovery. A second
live owner cannot unlink the endpoints. This lock does not prove a crashed
predecessor's process tree has been reaped: the crash-restart test independently
waits the orphan collector before restarting. An installed external supervisor
must establish that closure before restart.

Run `cargo test --locked` and `cargo clippy --all-targets -- -D warnings` in the
isolated root test environment. Process fixtures additionally require SETUID,
SETGID and KILL; CHOWN, FOWNER and DAC_OVERRIDE provision synthetic fixtures.
Those fixture capabilities are not an installed-service qualification. Raw
runtime logs, synthetic private homes and captures remain private. See the
[boundary and evidence](../docs/native-appearance-service.md) for chronology and
the separate evidence classes.

This initial projection covers selected opaque colors. It does not yet preserve
all previously qualified font resolution, user overrides, typography, spacing or
alpha projection. The existing public application's full theme path must not be
replaced by this partial projection until those features and installed integration
are qualified. ARM compilation alone is not runtime, original Omarchy or phone
evidence. Physical Pixel hardware, real suspend, current Source/lock composition
and the complete smartphone remain open.
