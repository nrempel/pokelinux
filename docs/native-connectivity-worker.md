# Native connectivity worker

Three current-source host cases pass: persistent packet lifecycle, request expiry,
and explicit Stop with an outstanding queued Connect. The Rust component exports
a reusable `Service` Main library and a worker-only executable. There is no
installed Main, public application ingress or Settings permission/UI boundary.

Trusted startup retains two fresh, exclusively administered network namespaces,
launches one cooperative worker, then drops Main's capabilities. The worker keeps
only `CAP_NET_ADMIN`; no privileged broker or replacement worker remains. It owns
the fixed IPv4 virtual backend while Main snapshots do local model/deadline work.
Observations expire after250ms. Idle refresh is scheduled after50ms and uses its
original100ms read deadline. Absolute BOOTTIME fences publication, not kernel
syscall duration or cancellation.

## Actual host join

- Lifecycle: one worker exchanges two nonces across fixture-principal withdrawal,
  publishes fresh idle observations, and handles explicit Disconnect from a fresh
  trusted principal. Independent address readback and peer quiet confirm the case.
- Expiry: Connect is queued under an original100ms deadline while the worker is
  stopped through its actual pidfd. During a350ms snapshot-only hold, both visible
  observation layers expire and the epoch
  becomes terminally uncertain. After SIGCONT, the selected independent window
  observes no target IPv4 address or address notification and no revived reply.
- Stop: while the worker is stopped, Connect is actually queued before explicit
  Stop. Admission is revoked, the pending result is retired uncertain, and the
  worker is actually waited with stdout EOF before namespace handles are released.

The lifecycle builder also passes eight owner groups, full formatting and strict
all-target Clippy, plus a focused admission-cache test. That component test authors
a cached Fact around a real retained child and pipe. It proves cached readiness
is revoked after actual wait while stdout is held open; it does not exercise
`Service::start`, bootstrap, namespace creation or a network connection.

The final join has62 disjoint Scope waits/stdout PIPE EOFs:24 lifecycle and19 each
for expiry and Stop. Nine regular Cargo stderr handle closures are separate.
The three network workers and the cache test's helper each have an additional
native wait/EOF; these four are not extra Scope receipts. Owned scopes, exact
containers, original cutoffs and input hashes close without adopted waits.

## Preserved lineage and limits

Initial Connect RED remains qualified separately from its failed outer aggregate
(private-result read permission). Pre-format lifecycle GREEN, nested expiry RED,
and real-child admission/EOF RED remain distinct. Formatter bridges are recorded.
Final lifecycle Sol1 stopped at Clippy; Sol2 was unexecuted; Sol3 refused the
stderr/preexec setup before functional execution. Sol4 changes only that builder
plumbing and runs the unchanged final fixtures against the current sources.

Action uncertainty is terminal: no retry, rollback, implicit Disconnect or restart
is supplied. Local wait/EOF proves no remote cleanup. SIGSTOP and signal9 reaping
do not qualify arbitrary uninterruptible kernel work. All evidence is native
x86_64 host Debug execution with authored authority and fixed synthetic IPv4.
Installed Main, OS/application permissions, Settings, restart/recovery, actual
suspend, ARM, Wi-Fi, DNS, Internet and physical-phone gates remain open. A new
epoch requires explicit trusted startup in fresh namespaces after old custody
closes; no same-namespace replacement or connection-persistence policy is selected.

[Hash-only provenance](evidence/native-connectivity-worker-provenance.json) and
[the exact public fixture sources](../tools/native-connectivity-worker/README.md).

## Later normal CLI error context

A separate [error-context checkpoint](native-connectivity-worker-error-context.md)
uses the normal Release worker as an actual UID1000, cap0/NNP1 process. Its real
capability-reduction EPERM first lacks the desired stage, then passes the unchanged
consumer after error-only context is added. Natural worker exit1, the original
source error, actual wait/PIPE EOF and the original1s cutoff are preserved.
Seven contexts compile; only `ReduceCapabilities` executes in this control.
Current normal host/ARM builds pass, but this adds no ARM context execution,
virtual-link result or explanation of the failed ARM normal-window worker loss.
The historical packet/expiry/Stop results above retain their original cohorts.
