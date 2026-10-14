# Native appearance owner

This Rust service publishes bounded typed colors, font sizes and spacing from
the desktop user's private Omarchy theme and user shell settings. It supplies
presentation only: an appearance revision
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
and `appearance`. The schema is2; the incarnation is32 lowercase hexadecimal
characters. `until_ns` is a canonical decimal string preserving exact u64
`CLOCK_BOOTTIME` nanoseconds. Available `appearance` has exactly `palette` and `style`. The
palette has exactly nine RGB fields:
background, foreground, surface, border, accent, muted, urgent, positive and ink.
Style has exactly `base_size`, eight `font_tokens` and six `spacing_tokens`.
Base/font values are positive signed-int32-range integers; spacing also permits
zero. The family/resource resolver is still separate unfinished work. Unavailable
publication has `appearance:null`; no schema1 reader is retained. The whole typed
payload participates in revision equality and final queued-send validation. A
cached presentation is not a current lease.

Source traversal and parsing run in one owned UID1000 collector. Completion
requires actual packet PID/UID/GID, retained pidfd liveness, incarnation/token
and the original one-second BOOTTIME deadline. The collector holds the original
theme-set shared lock and selected directory generation through publication ACK.
Generated and user shell maps are parsed independently and user keys win;
removal reveals generated settings/defaults. Input handles, fixed ancestor chains,
selected path identity, acquisition/read metadata, retained bytes and optional
absence are rechecked before completion and through the ACK loop. Original user
settings writers do not use the theme-set lock: these are separately qualified
observations and convergence, not an atomic transaction across files. The owner
retains a terminated worker slot until actual wait; collection retry is read-only and cannot retry a radio
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

Nineteen actual host groups pass, including user token replacement/deletion,
style revisions, unsafe user file types/content and absence recovery. A separate
actual held-acquisition control exposes obsolete colors renewal before this repair;
the unchanged control now observes a null candidate, withdrawal, fresh collection
and actual reaps. Original full-style ARM comparison has a genuine pre-change RED;
seven later original ARM composition groups now pass. All15 token/scale observations
match twice per phase in the same normal app/window with private HOME/EACCES. The
repaired process fixture also passes18 separate generic ARM kernel groups while
the production ELF stays unchanged. Its prior ESRCH aggregate remains failed;
the test now retains a fresh pidfd and observes actual stopped state before the
original kill/reap/restart assertion. Qt schema2 controls remain private synthetic
consumer evidence, not complete app migration. Visible large-font clipping,
concrete font resources and normal-window font/DPR remain open.

This projection covers selected opaque colors and integer font/spacing tokens.
Concrete font resolution/resources, translucent colors, original parser edge cases
and Qt signed-int overflow coercions remain unqualified. Unsupported out-of-range
sizes are refused, never silently clamped; original Style defines no upper cap.
The full original font/settings/alpha behavior is not yet preserved. The existing
public application's full theme path must not be replaced by this partial projection until those features and installed integration
are qualified. ARM compilation alone is not runtime, original Omarchy or phone
evidence. Physical Pixel hardware, real suspend, current Source/lock composition
and the complete smartphone remain open.

The Color/Style behavioral reference is pinned
[Omarchy v4.0.4](https://github.com/omacom/omarchy/tree/c668141e9c42b13c80c9ca4ea108e11708c5e8a5),
copyright David Heinemeier Hansson, under its
[MIT license](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/LICENSE).
The Rust projection is checked against executed original observations; no
upstream tree, private home or raw runtime evidence is published here.
