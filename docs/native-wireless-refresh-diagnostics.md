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
nor clean event state, freshness or permission to retry. Those diagnostic additions
introduce no socket replacement, retry or implicit Disconnect; the guarded periodic boundary below has separate implementation and
execution evidence.

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

Those diagnostic checks are native x86_64 host results. The precise Cold 23 radio
phase remains unknown. The diagnostic-source build at `9a40f26` compiled both
normal ARM Main/worker binaries, with 65 passing host regressions and one ignored
fixture.
[ARM build evidence](evidence/native-connectivity-radio-diagnostics-arm-build.json).
Cold 24 now retains a complete early record through the installed
[journal-stream sink](native-connectivity-worker-error-context.md#journal-stream-diagnostic):
radio Refresh `Failed(Expired)` at `BeforeProfile / QuietAfter / Completed`.
The later Termination output is partial. The run reaches Settings but fails before
Enable-ready; cleanup qualifies separately. No safe recovery, retry or completed
connection lifecycle follows from the framing state.
[Installed evidence](evidence/native-installed-wireless-refresh-cold24.json).
[Retained host evidence](evidence/native-wireless-refresh-diagnostics.json).


## Periodic-only deferral

The actual ARM/hwsim direct-library baseline reaches final `QuietAfter` after
completed command framing without consuming event bytes, then exits 101 on the
desired `Deferred` assertion. Fixture prerequisites, actual waits/EOF, radio and
namespace teardown, QEMU exit 0 and expected-RED outer closure qualify. Earlier
failed setup cohorts are preserved and excluded from that behavioral RED.

Periodic read-only observations now return typed `Observed` or `Deferred`.
Deferral requires expiry at final `QuietAfter`, completed command framing and no
consumed-but-unvalidated event datagram. The obligation is latched immediately
after receive and cleared only after the whole datagram is validated. Expiry
with that obligation, pending commands, loss, malformed data, changed identity
and contradictions retain ordinary refusal/retirement. Completion alone is
insufficient.

The composed backend checks already observed radio/profile contradictions before
any later deferral and returns no partial bracket or combined fact on `Deferred`.
The worker publishes nothing and preserves the old timestamp and 250 ms lease.
After deferral completes it schedules a new full observation for completion
+50 ms, with its own fixed 100 ms cutoff. It never resumes or extends the expired
operation. Startup and explicit Connect/Disconnect use strict reads and original
cutoffs. No mutation retry, socket replacement, rollback or implicit Disconnect
is added.

The validation build passes formatting, strict all-target Clippy and 51 selected
host tests: 17 composed checks and 34 radio checks. Five hwsim tests are ignored in
that host invocation; 45 other connectivity tests are filtered out. All 65 selected
sources match its formatted snapshot. Host/ARM libtests and normal ARM validation
binaries compile. The artifacts remain explicitly candidate-validation outputs,
separate from the subsequent public-source deployment build below.

All five frozen direct-radio tests then pass on actual generic ARM/hwsim:

- Final quiet expiry returns `Deferred`, followed by a fresh full observation on
  the same epoch and command/event descriptors.
- Strict refresh at that boundary remains terminal `Expired`.
- One dispatched CONNECT expires `Uncertain(Expired)`; later calls refuse
  without another command.
- After an actual AP deauthentication, consuming 2384 event bytes then expiring
  remains terminal `Expired`.
- AP loss queued after `Deferred` remains terminal `Contended` on the next read.

Both AP-loss controls admit one deauthentication and independently observe loss
before the original 1 s gate. Each case qualifies full fixture/radio/namespace,
QEMU exit 0 and container exit 0 closure with unchanged selected inputs. All
recorded owned-child waits/EOF are timely. Each guest separately reaps 33 terminal
adoptees with actual bounded waits and ECHILD; their PIPE EOF is not claimed.
The two initial EACCES gate-handshake runs are excluded. Their corrected runs
use guest tmpfs gates with unchanged Rust, ELF, capabilities and cutoffs.

These controls execute the direct radio library, not normal Main, SDK or GUI.
The subsequent public `9882af9` build passes 61 connectivity-library and eight
owner tests with one ignored fixture and produces both normal ARM binaries.
Those counts overlap the earlier 51 host checks. Image 21 assembly/readback and
loaders pass; only Main/worker change among 326 installed rows. Assembly executes
no normal app. Actual QEMU/container exits 0 come from retained receipts; no
image-owner tool terminal exit is asserted.

Cold 25 then fails during first Permissions-menu rendering when the guest watcher
gets ESRCH reading a task's proc children. It reaches no Allow/Enable or packet
flow. Guest/QEMU/outer cleanup qualifies independently of functional failure.
Normal-service deferral/recovery and the full installed two-boot flow remain
unqualified. Cold 24's complete diagnostic and failed outcome retain their
original scope; no process identity or radio cause is inferred for Cold 25.
[Qualified RED, host/ARM controls and pending gates](evidence/native-wireless-periodic-refresh.json).
