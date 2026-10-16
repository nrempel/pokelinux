# Source publication waits for event dispatch

The compositor adapter now defers every publication until its first actual
Wayland timer callback. Initialization and early focus/lock callbacks still
observe conservative state, but cannot advance packet sequence or reconcile a
positive lock association before sampling starts. A shared publisher performs
the existing bounded JSON/sendmsg/SCM_RIGHTS path. The wire, Rust SourceOwner,
Observer and100ms freshness lease are unchanged; later failure stays terminal.

The preceding [combined startup failures](native-session-owner.md#cold-start-source-failure)
include a separately modified private owner diagnostic: first returned sequence1
ages120.9ms, then `TimedOut` and actual child signal9. This identifies that
private epoch's expiry, without identifying the blocking startup operation or
retrospectively diagnosing other epochs.

## Actual boundary tests

The adapter's packet publisher, BOOTTIME reader and inherited-channel validation
are first extracted without changing immediate publication. No feature RED is
claimed for that extraction. Two fixed Rust cases launch an authored native
UID1000 C++ producer through the unchanged production bootstrap and retain the
actual SourceOwner Child, pidfd and Rust Observer. The producer executes real
libwayland timer dispatch, with a separate fixture-only control socket.

On a generic ARM kernel, the baseline produces one actual early-publication
failure and one passing expiry control: Rust receives sequence2 before any timer
dispatch. With tests and C++ fixture frozen, only the shared publisher's gate
changes. Both cases then pass:150ms with no packet before first dispatch, fresh
sequence1 then2, and permanent `TimedOut` after a later real stall, including
after dispatch resumes. The configured25ms timer produces an observed25,690,904ns
sample gap; this is an observation, not a timing-accuracy qualification.

Each run retains two actual Rust parent waits before its original8s case cutoff.
Source stdout/stderr are DEVNULL, without PIPE EOF claims. The test binary,
compilers and outer helpers have their applicable waits and PIPE EOFs; guest
ECHILD, actual QEMU/CLI exits, same-daemon exact container absence and input
closure qualify separately. Failed compiler/SDK preparation remains setup
failure, not behavioral RED. No package or network provisioning is needed.

Strict ARM Rust formatting/Clippy/test compilation passes. GREEN also compiles
the actual adapter against the exact installed Hyprland SDK; that is compile-only
and executes no original compositor. The fixed test sources and reproducible
component instructions live in [the fixture](../tools/native-source-timer-boundary/README.md).
[Hash-only provenance](evidence/native-source-timer-startup-provenance.json)
binds source, fixed tests, native artifacts and RED/GREEN process evidence.

## Remaining integration

The next [original-compositor replay](native-session-owner.md#gated-original-compositor-replay)
compiles all three plugins and reaches the display metadata probe, which exits255
before Sounds or PCM. The compositor is dead before Stop; cause is unknown.
Private profile ordering and readiness are fixture repairs, without a feature
RED claim. Mandatory ordered-receipt prerequisites pass in executed setup, but
the raw receipts are not retained before the later failure.

Next expose bounded observed Source state/first failure through the existing
root-admin snapshot before another exact replay. Keep the initial120s limit,
100ms sample lease and terminal failure unchanged. Normal GUI, genuine theme, permission/native focus,
per-app listener, background route, virtual graph/PCM and Source-loss effects
must qualify in that full selection. No new screenshot or physical qualification
follows from these component tests. All smartphone completion and hardware
gates remain in force.
