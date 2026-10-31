# Native connectivity worker

The current Rust Main selects one backend from a bounded root-owned regular
`0600` JSON profile passed as its second argument. It rejects unknown fields,
symlinks and old raw-namespace startup arguments. For example:

```json
{"backend":"virtual_pair","peer_namespace":"/proc/123/ns/net"}
```

The [wireless-open profile](../tools/native-wireless-service/README.md) instead
selects the actual current station namespace and existing interface. Main retains
that namespace; only virtual-pair startup retains a peer namespace too. The worker
independently checks its inherited namespace before backend construction. Decoded
configuration is copied to the authenticated worker; it never reopens the profile.

Main drops all capabilities; the worker keeps only `CAP_NET_ADMIN`. Typed private
v2 observations distinguish witnessed virtual lifetimes from existing wireless
identity. Old worker startup/record forms are rejected. Observations expire after
250ms; idle refresh is scheduled after50ms with its original100ms cutoff. Wireless
facts use the oldest bracketed radio/profile observation, not publication time.
Backend I/O stays in the worker; snapshots do local model/deadline work.

A subsequent current-source x86 container regression passes the real virtual-link
Service/worker lifecycle: two independent packets across requester withdrawal,
same-worker idle observations, explicit Disconnect and quiet, then actual worker
wait/EOF before namespace release. Strict Clippy first rejected the large backend
enum and a nested test conditional; the repair owns the wireless backend in one
Box and collapses that conditional. The unchanged formatting, lint, eight owner
tests and cache-custody test then pass. All 24 owned waits/EOF and exact container
absence qualify. This direct Service test does not exercise Main's protected
profile-file entry. [Current virtual evidence](evidence/native-connectivity-current-virtual-lifecycle.json).

The third candidate ARM cohort now passes the same frozen
[wireless service consumer](../tools/native-wireless-service/README.md) in a generic
ARM64 QEMU guest with simulated hwsim radios. The actual UID11050 Settings SDK app
obtains its durable grant through authored actor authority and submits Connect to
normal Main. Independent station/AP and fixed IPv4 readback agree with a cap0
nonce/ACK exchange. Actual SDK Stop/join and requester wait precede reopening in a
distinct requester, two fresh observations of the same worker/connection, and a
second independent cap0 packet exchange. Explicit Disconnect has independent
radio/address/subnet-route absence and the original quiet-window checks: the800ms
budget includes gate delivery, with zero peer datagrams and actual final EAGAIN
after the original cutoff.

Main, actor driver and packet peer exit0. Main's normal Stop retires the retained
worker with SIGKILL9, actual wait/output EOF and retained namespace custody. The
signal establishes process retirement; independent radio/address checks and
owned AP/radio/namespace teardown establish their separate closure. Guest, QEMU
wait0, container exit0/absence and unchanged selected inputs all qualify. The
original six-second case, three-second startup, one-second actions and250ms lease
remain unchanged.

The first candidate loses the worker after initial Ready without an identified
cause; the diagnostic second candidate confirms association, then fails profile
observation. Both remain failed. Zero success-gate counters do not prove zero
dispatched work. The passing third cohort uses an authored open AP and fixed IPv4;
normal Wi-Fi GUI, WPA/credentials, DHCP/DNS, Internet, matched Pixel startup and
physical-phone gates remain open. The following virtual packet/expiry/Stop
results retain their separate historical cohorts. See
[wireless candidate evidence](evidence/native-wireless-service-candidate.json).

## Earlier actual host join

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
do not qualify arbitrary uninterruptible kernel work. Those earlier virtual
results use native x86_64 host Debug execution with authored authority and fixed
synthetic IPv4. They establish no installed service activation, normal Settings
GUI, restart/recovery, actual suspend, ARM or physical-radio result; the newer
generic ARM hwsim checkpoint above is separate. A new epoch requires explicit
trusted startup after old custody closes; no same-namespace replacement policy
is selected.

[Hash-only provenance](evidence/native-connectivity-worker-provenance.json) and
[the exact public fixture sources](../tools/native-connectivity-worker/README.md).

## Later normal CLI error context

A separate [error-context checkpoint](native-connectivity-worker-error-context.md)
uses the normal Release worker as an actual UID1000, cap0/NNP1 process. Its real
capability-reduction EPERM first lacks the desired stage, then passes the unchanged
consumer after error-only context is added. Natural worker exit1, the original
source error, actual wait/PIPE EOF and the original1s cutoff are preserved.
Seven contexts compile; only `ReduceCapabilities` executes in this control.
At that checkpoint normal host/ARM builds passed, but this added no ARM context execution,
virtual-link result or explanation of the failed ARM normal-window worker loss.
The historical packet/expiry/Stop results above retain their original cohorts.
