# Wireless refresh failure diagnostics

The retained Rust radio owner now records the first failing refresh observation
phase and the command transaction state. The composed connectivity backend adds
whether that radio observation preceded or followed the IP-profile read.

Phases distinguish namespace checks, initial/final event checks, initial/final
interface checks, association dump/validation and the final deadline fence.
Admission failures may have no observation context. The first retained failure,
its classification and all original cutoffs remain unchanged.

`NotSent` means the current request has not passed its send fence. `Pending`
means dispatch was attempted without validated terminal framing; simply receiving
bytes does not complete it. `Completed` means the expected correlated terminal
framing was consumed. Ambiguous request flags or an older pending transaction
prevent that classification. Completion establishes neither valid radio identity
nor clean event state, freshness or permission to retry. No socket replacement,
retry, recovery or implicit Disconnect is added.

Three real host `nlctrl` controls use the built-in family by numeric ID, without
module lookup, network capabilities or a wireless device. The inert baseline
passes expiry before send and fails the pending/completed state assertions with
actual test exits 0/101/101. The identical controls then pass 0/0/0. Their timed
cases retain the original 100 ms BOOTTIME deadline.

A fourth actual kernel control confirms that receiving bytes leaves the command
Pending. Authored flag/latch and backend-bracket controls remain separate from
kernel evidence. All 34 wireless tests and 13 backend tests pass; both crates
pass formatting, strict all-target Clippy and Release libtest compilation.
The initial formatter-only setup failure is excluded from behavioral RED.
All selected executions close within their original bounds.

These are native x86_64 host results. The precise Cold 23 radio phase remains
unknown. A combined ARM Main/worker build and installed run must still qualify
these diagnostics together with the [journal-stream sink](native-connectivity-worker-error-context.md#journal-stream-diagnostic).
[Retained evidence](evidence/native-wireless-refresh-diagnostics.json).
