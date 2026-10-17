# Native owner SIGTERM Stop

The native Rust session owner now translates SIGTERM into its existing actor
Stop operation. The real optimized owner passes termination before readiness:
Main exits0, both retained compositor/storage children are dead and absent,
no child is adopted by the test subreaper, and final ECHILD is observed. One
Harbour host measurement takes20.29ms from pidfd SIGTERM to the actual Main wait.
This is an authored-peer Linux integration result, not a phone measurement.
This host result alone does not qualify installed manager Stop. The later
[PID1 selection](native-session-manager-stop.md) is recorded separately.

## One Stop owner

A process-owned guard is installed before compositor custody can exist and is
retained through the outer Source/history drain. Its signal handler only stores
an atomic flag. It creates no thread, file descriptor, mask change or second
request route. The lifecycle actor consumes that intent before polling Source or
handling more requests, then calls the same `begin_stop` used by authenticated
admin Stop. The first operation retains its acknowledgement truth and original
cleanup deadline; repeated signals cannot start another shutdown or renew time.

The shared operation withdraws authority, fences providers, starts the existing
app grace/retirement path, marks Source stopping and either requests the existing
clean storage shutdown or retires outstanding work. Occupancy still clears only
through existing actual waits. The strict `--stop` client and its wire protocol
are unchanged. Reaping local processes establishes no vendor resource cleanup.

The small Linux signal primitive lives in `native-kernel-identity`; it does not
modify the kernel. Installation and destruction require the initial main thread,
a single-threaded process and trusted procfs. SIGTERM must be unblocked. The
process owner excludes concurrent thread/signal-configuration changes at those
boundaries; any later threads must join before destruction. The guard is neither
Send nor Sync. It restores the complete prior action or aborts instead of
claiming restoration. A fork child cannot consume its parent's copied latch;
caught dispositions reset on exec. Adding in-process libraries must preserve
these explicit assumptions.

## Actual RED and GREEN

The frozen production-owner consumer first runs the released pre-repair owner.
It authenticates root-admin Snapshot from that exact Main, observes
`awaiting_first` with no accepted sample, retains its actual UID1000 compositor
and root storage child, then sends SIGTERM through the retained Main pidfd. Main
exits-15. The test subreaper must reap both parent-death-killed children. That
aggregate remains failed; its bounded process/container cleanup is retained.

The byte-identical consumer then runs the new default optimized owner. Setup
still has policy not ready, no Source frames and no positive audio reply. Main
exits0, no children are adopted, both exact descendant pidfds are dead, process
entries are absent and final ECHILD passes. The original5s work/8s close limits,
real waits, PIPE EOFs, exact container absence and source/input closure pass.
This exercises actual bootstrap, SourceOwner, storage and lifecycle composition;
the compositor/audio peers are authored and supply no successful service facts.

A separate frozen signal primitive consumer first fails compilation for its
missing interface, then passes five real signal/lifetime groups: duplicate latch
and prior-action restoration, fresh lifetime, blocked-mask refusal, actual joined
foreign-thread refusal and an exec child with default/unblocked SIGTERM. All14
existing active leaf tests pass serially; three helper modes remain ignored.
The first parallel run's existing numeric-FD-slot assertion race is preserved,
with no change to those tests. Formatting, strict Clippy and Release pass.

The composed selection also passes110 current regressions:68 default lifecycle
cases,12 Source-status,19 display/notification, five startup-eligibility and six
strict native Stop cases.20 other lifecycle integration cases remain unselected.
Owner/lifecycle formatting, strict all-target Clippy and the optimized default
owner build pass. These host cases do not qualify current ARM or Pixel behavior.
The [hash-only evidence](evidence/native-session-signal-stop-provenance.json)
binds the actual separate epochs; raw logs and binaries stay private on Harbour.

## Installed gates

The later unit removes the redundant ExecStop helper and selects KillMode=mixed.
Its [actual PID1 checks](native-session-manager-stop.md) observe clean manager Stop
through the Rust signal path after terminal Source loss. The unchanged-unit
baseline also stops cleanly when its helper runs while Main is live; it is not
reported as a new unit-repair RED. The earlier native-IPC-first ExecStop failure
remains separate evidence.

The earlier readiness-owner cold run stops on the queued-job observer error and
retains its failed aggregate and incomplete guest closure. Corrected observers
then reach notify active/running in two genuine cold epochs, but Source expires
before the shell can connect. No full-shell/app/PCM pass follows. Held unit
references preserve matching invocation exits; reset PID1 defaults still cannot
manufacture success.

Next identify that startup Source gap and qualify the complete healthy cold
full-shell/Sounds/Stop path. Live combined themes, Messages, coherent ARM
composition, rendering-stall availability, native display cleanup and all Pixel
hardware/carrier gates remain open. No physical phone is accessed.
