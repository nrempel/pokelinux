# Settings Rust connectivity client

The normal Settings Rust crate now has a separate safe connectivity engine. Its
worker uses the lifecycle actor's public credentialed socket. It accepts one
explicit desired boolean and the caller's original absolute CLOCK_BOOTTIME
cutoff; local acceptance is not a network acknowledgement. This is fixed local
IPv4 backend work, without a Wi-Fi, Internet or modem claim.

The worker retains the socket established by readonly status. Exactly one
transaction is outstanding; completion is bound to that socket generation and
local click. Only readonly recovery reconnects. A mutation makes one nonblocking
send with no retry after backpressure, interruption, loss or uncertain completion.
The actor remains responsible for actual process, grant, lifecycle, session and
backend authority; a cached client `can_control` is only an admission hint.

Queued work permanently becomes `not_sent` when its old observation expires,
control becomes unavailable, transport changes, the original action deadline
expires or Stop is observed. A later readonly reply cannot revive it. The
`Queued → Attempting` transition commits dispatch under a short model lock; the
single send and all socket I/O happen outside that lock. Stop racing after this
commitment may still be followed by the send. Stop withdraws local publication;
it is not a remote Disconnect or cleanup acknowledgement.

Attempting/sent custody remains occupied until the worker actually returns,
even when a getter has already retired the action. A late or invalid completion
cannot overwrite that retirement. A successful completion must match the target,
carry a fresh exact 250ms observation after dispatch, and arrive before the
original action deadline. It never grants permission. Current readonly facts and
sticky `uncertain`/`not_sent` action outcomes remain separate; fresh facts alone
cannot confirm an earlier action. A new accepted explicit choice can clear the
previous outcome.

All getters expire facts against current BOOTTIME. Transition timestamps are
sampled after acquiring the model lock. Polling uses bounded intervals. Stop
captures one original 200ms join budget; the thread handle stays owned through
actual join. Dropping an unjoined engine aborts rather than detaching it. A
nonblocking flag is not evidence that arbitrary kernel stalls can be reclaimed.

## Integration evidence

The [frozen process consumer](../tools/native-connectivity-settings-sdk/README.md)
first compiled and ran the real normal SDK as its app dependency, with a
refusing setter. An actual launched UID11050 process obtained a fresh false
state and durable NetworkControl grant, but Connect returned false. Independent
kernel readback and packet silence agreed. The exact desired assertion exited
101 after SDK join/task-count-one, actual Apps wait, actor join, Main's native
worker wait/output EOF, peer/keeper waits and container closure. Compilation or
setup failure is not counted as RED.

The implementation passes that same frozen consumer: granted Connect and
fresh matching SDK state, independent packets, actual first app exit, fresh app
observations and another packet, explicit Disconnect and independently observed
address removal/ENETUNREACH101/quiet. Both SDK workers actually join in about
2.1ms against their original 200ms cutoffs, with independent task-count-one
observations and separate actual Apps waits. The functional case takes about
1.21s within its original six seconds. One-second action deadlines and the
800ms quiet phase remain unchanged; the latter includes gate delivery.

Formatting, strict all-target SDK Clippy and the original seven plus five pending
brightness socket/C ABI groups pass. Nineteen client waits and 38 client PIPE EOFs
close, with the wrong-PID writer separate. These regressions do not exercise a
connectivity C ABI. The unchanged lifecycle default suite retains its earlier
actor checkpoint; it was not repeated for this SDK addition.

The private outer auditor initially used the wrong keys for two SDK choice
records after the product and cleanup had passed. That aggregate remains failed.
An offline schema correction validates every originally unreached predicate and
all existing wait/EOF records without repeating tests, starting clocks or
relabeling the original receipt. Root independently reviews the raw results.
The epoch closes 27 Scope waits/PIPE EOFs and eight separate regular stderr
sinks, with no adopted waits and exact container removal. Native-worker closure,
Apps waits and SDK joins are not counted as those Scope command receipts.

Six supplementary real-socket groups also pass: strict state shapes, actual
SCM_RIGHTS/wrong-PID rejection, getter expiry/responsive Stop, exact original
mutation deadline, five queued-choice cases, and sent expiry followed by fresh
readonly facts that retain uncertainty without replay. These temporary root
peers establish protocol components, independently of the actor/packet result.
Seventeen nested actual Child waits and 34 PIPE EOFs close; 16 SDK workers join
within their original 200ms bounds and leave one task. The root peer needs
SETUID/SETGID/KILL for credential transitions and failure-path child custody;
UID11050 has zero effective/permitted/inheritable/ambient capabilities, retained
three-bit bounding set and NoNewPrivs. Compiler capabilities and command Scope
receipts are recorded separately.

Two protocol predecessors remain visible: a 16MiB inherited file-size limit
prevented static-archive compilation, then an actual compiled run passed four
groups and failed two on fixture reset handling. Neither is SDK behavior RED.
The resource-only successor uses a bounded 64MiB file/output cap. The receive-only
successor retains the exact socket after ECONNRESET and keeps draining queued
packets to actual zero-byte EOF under the original cutoff. It does not discard a
reset as successful EOF: Linux can check the pending socket error before queued
packets. See the primary [Linux datagram implementation](https://github.com/torvalds/linux/blob/master/net/core/datagram.c)
and [Unix socket close path](https://github.com/torvalds/linux/blob/master/net/unix/af_unix.c).
Two observed resets precede actual EOFs. Socket EOF counts remain separate from
PIPE EOFs. No production SDK code changed for either fixture repair.

See [hash-only provenance](evidence/native-settings-connectivity-sdk-provenance.json).
Earlier actor-only, private-Main and persistent-worker epochs retain their
original evidence scopes. Formatting-only source promotion preserves the
compiled consumer and implementation; no extra functional rerun is inferred.

## Remaining gates

The normal Settings connectivity C ABI, Qt controls, installed session-owner/Main
selection, deployment and ARM integration remain open. This cohort supplies no
new normal window, screenshot, genuine Source/authentication, actual suspend,
physical Wi-Fi/Bluetooth/Internet, carrier or phone result. No physical phone was
accessed. Pixel 10 remains the first target; no phone is qualified.
