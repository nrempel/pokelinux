# Rust session startup readiness

The installed session owner now uses native Rust startup notification. Its unit
selects `Type=notify`, `NotifyAccess=main` and a120s startup limit. This addresses
the observed ordering defect: `Type=exec` allowed the genuine shell to try its
Wayland connection before the compositor endpoint existed. Subsequent installed
PID1 runs reach Source freshness failures before healthy cold-shell startup;
those failures remain failed. See the [actual cold RED](native-normal-upstream-tracer.md#cold-installed-startup-fails-before-sounds)
and [later cold diagnostics](native-session-x86-runtime.md).

## Existing authority, one notification

The existing lifecycle actor supplies its already accepted Source sample cutoff
after polling the sole Source provider. Durable policy must be ready, Source must
be required and its accepted sample must remain fresh and nonterminal. A locked
session is eligible; unlock, foreground focus and app permission are independent.
No second Source consumer, synthetic heartbeat or renewed deadline is added.

The current provider performs [bounded Source establishment](native-source-establishment.md)
before supplying that first accepted cutoff. Provisional packets cannot reach
the lifecycle actor or satisfy the notifier; the original startup limit remains.

The Rust notifier makes a nonblocking connection to the configured Wayland
endpoint. It checks actual peer PID/UID/GID1000 and obtains the kernel's peer pidfd,
matching it against the retained compositor pidfd through the existing trusted
procfs view and final joint liveness check. Before its one nonblocking `READY=1`
datagram, it rechecks both the accepted sample cutoff and the original startup
BOOTTIME deadline. These checks establish freshness at the last userspace
publication decision; they are not an atomic kernel-send or delivery deadline.

An absent endpoint or full connection backlog remains pending within the original
startup bound. Wrong identity, notification failure or deadline failure is
terminal. Stop permanently fences later publication. A full notification queue
never creates an implicit retry. The owner sends from Main; optional
`NOTIFY_SOCKET` handling adds no helper, detached thread or production fallback.
All setup errors retain the existing Source owner through its actual drain.

The exact selected normal Hyprland configuration directly loads the Source
plugin before any shell unit starts. Its timer can publish a valid no-focus
observation independently of the shell. Source-only review finds no dependency
cycle in waiting for this observation; it does not qualify actual cold startup.
The100ms sample budget and120s original Source startup bound are unchanged.
Established timing loss now follows the separate
[recovery contract](native-session-timing-recovery.md); terminal identity/wire/
protocol failure and Source-death handling remain intact. Upstream graphics
engines remain unchanged.

## Executed Harbour boundaries

Two actual missing-interface compiler REDs precede production implementation:
missing session-owner readiness module, then missing lifecycle startup Observer.
They are structural REDs and execute no functional cases. Separate failed host
post-check aggregates remain preserved: the first Rust image had no Python
post-checker, and the actor diagnostic validator expected different top-level
compiler wording. Actual structured compiler diagnostics, input closure and
owned process/container closure independently qualify those compiler failures.

The implemented selection passes these actual host boundaries:

| Boundary | Passing cases | Scope |
| --- | ---: | --- |
| Notification/socket/process |10 | Real credentials, retained pidfds, absent/stale Source cutoff, wrong/dead/replaced peer, original deadline, Stop, full stream backlog and datagram queue, terminal send failure |
| Existing display regression |9 | Existing owned context and peer behavior |
| Lifecycle startup eligibility |5 | Actual lifecycle actor, storage worker, root-admin IPC and original SourceOwner/Observer; authored UID1000 Source sender |
| Existing Source-status integration |12 | Actual actor/Observer rejection, timeout, exit and first-cause retention |
| Default lifecycle components |68 across two epochs |66 initial passes; two missing-helper setup failures then separately pass with the same production source and correct same-build daemon.20 ignored integration cases remain unselected |

The initial pressure test did not fill the standard library's implicit listen
backlog within its256-socket cap. It remains failed setup evidence. The existing
test-only peer now explicitly selects a real eight-entry backlog; every frozen
notification assertion and the256-socket budget remain unchanged. A later
all-target check exposed accidental standalone Cargo discovery of the consumer;
its byte-identical body now lives under `tests/support`. The actor consumers
receive formatting only, with their assertions unchanged.

Owner/lifecycle formatting and strict all-target Clippy pass. The enabled
per-app display fixture check and optimized native owner build also pass with
the same selected production sources. Actual actor and
peer waits, applicable PIPE EOFs, exact container absence, same daemon and selected
source closure are retained. The combined first actor aggregate remains failed
because its two existing storage fixtures initially selected a nonexistent daemon
filename; the targeted recovery also supplies their required private writable
`/opt`. Later optional generated-camera fixture compilation fails setup because
its verified schema inputs were not supplied; that aggregate is not promoted.
The generated camera/phone fixture build gates remain unqualified here.

The [hash-only evidence](evidence/native-session-startup-readiness-provenance.json)
binds the separate selections and qualifications. Private run logs and binaries
are retained on Harbour and are not published.

## Separate Stop defect and remaining gates

The actual released pre-repair owner separately fails a real pidfd SIGTERM
contract while awaiting Source: authenticated setup succeeds, Main exits-15 and
the subreaper must reap its compositor and storage children after parent-death
SIGKILL. Actual child death is not graceful owner cleanup. The contract retains
its failure, final ECHILD, all waits and exact container closure. The later [Rust signal
Stop/drain correction](native-session-signal-stop.md) now passes its separate
actual owner consumer. Later [cold PID1 epochs](native-session-manager-stop.md)
observe notify active/running followed by Source expiry, and clean manager Stop
after that failure. The old strict native `--stop` behavior is unchanged in this
readiness selection; the later unit removes the redundant ExecStop helper.

Still open: sustained healthy installed startup/Stop, genuine cold full shell and Sounds,
combined live themes and Messages, coherent ARM execution, rendering-stall
availability, native display cleanup, and every Pixel hardware/carrier gate.
No physical phone has been accessed.
