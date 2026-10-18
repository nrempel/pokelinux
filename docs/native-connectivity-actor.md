# Registered app control of native connectivity

A launched UID11050 Rust app using the Settings profile can now obtain the
existing durable NetworkControl grant, Connect through the lifecycle actor and
normal root-private Rust Main, and exchange real UDP packets. The connection
survives its actual exit. A fresh app generation receives two newer backend
observations, exchanges another nonce and explicitly Disconnects; independent
address readback, exact ENETUNREACH101 and peer silence confirm removal.

This is an isolated Harbour x86_64 Linux kernel result with a fixed IPv4 virtual
backend. The app is a Rust process fixture. The normal Settings window and its
production client engine, installed service selection, ARM and phone gates remain
open. It exercises existing app launch, per-packet credentials and durable grants;
session=None plus authenticated admin lock/background settings are authored
facts. No genuine Source, authentication or graphical shell runs in this tracer.
The production actor loop runs inside the selected library-test process; trusted
helper selection and actual app-wait observation use its existing cfg(test) seam.
This earlier actor cohort does not exercise installed connectivity selection. The
later [configuration component](native-connectivity-installed-selection.md) adds
explicit Disabled/Owner selection and compiles the normal session-owner wiring;
it does not qualify normal session-owner execution.

## Authority and lifetime

App input contains only an explicit desired `connected` boolean and its original
canonical absolute CLOCK_BOOTTIME `until_ns`, bounded to one second. There are no
app-supplied grants, identities, interface indices, addresses or device paths.
NetworkControl cannot use the generic historical Prepare/Admit permission path.

The actor captures current app generation, live pidfd, policy/lifecycle/session
and backend incarnation in a nonserialized, noncloneable stamp. Shared admission
rounds remaining milliseconds up, then clamps both authority and intent back to
the original deadline and captured session lease. An explicit desired state does
not require a compare-and-swap revision from an earlier status observation.

Private wire sequence IDs differ from actor intent IDs. The backend incarnation
binds local transport generation and actual worker identity. Only readonly
Inspect can establish a new transport; Apply requires retained fresh state and
cannot carry an old intent across withdrawal. Own readonly occupancy permits a
reservation but must finish before submission. Reported remote pending work
blocks a new effect; readonly reconciliation remains possible.

The actor checks the actual requester stream for EOF or extra packets before
submission and publication. It rechecks current authority and fresh actual
readback before sending success. Losing the app, grant, session, original
deadline or incarnation retires local reply authority. Retirement only withdraws
the private client. Main retains transferred work; retirement sends no implicit
Disconnect, retry, rollback or Stop, and supplies no remote-cleanup acknowledgement.
Readonly observations retain their actual timestamps and 250ms leases; the actor
polls every50ms without renewing a user action.

## Evidence

A genuine behavioral RED precedes effect implementation. The frozen consumer
qualifies healthy Main/worker, actual UID11050 app registration, no-grant denial,
the real durable grant and fresh available disconnected status with can_control.
Connect then returns Denied. Independent empty-address/route checks, exact failed
send and quiet complete; the desired assertion fails101 only after actual app
wait0, actor join, Main/native-worker wait/EOF and peer/keeper closure.

The same frozen consumer then passes real Connect, two distinct nonce roundtrips
including one after first app exit, a fresh second generation and explicit
Disconnect. The first functional GREEN completes in about1.19s under the original
six-second case limit. Startup/actions remain bounded by their original one-second
deadlines. The original800ms quiet deadline includes gate delivery; actual
armed-to-final-EAGAIN timing is reported separately in provenance.

The staged library also passes114 default tests, with27 ignored (including the
one copied integration case). Eight new component groups use controlled
Authority/Provider facts, real Disk grant writes, a self-pidfd and actual Unix
SEQPACKET boundaries. They cover admission/deadline shapes, readonly reservation,
revocation before submission, late completion, EOF/extra input, withdrawal after
submission without Disconnect, matching publication, wrong IDs, expiry and changed
incarnation. Their logical UID/self-PID facts are separate from the actual app
credential/permission proof above. Six normal lifecycle binaries, formatting and
strict all-target Clippy pass with a same-build storage helper.

The first fixture runtime fails setup because the older packet peer requires two
packets even on the zero-packet refusal branch. Its successor explicitly requires
zero for refusal or two for success; it preserves the clocks and quiet oracle.
An unexecuted compile-budget predecessor is also retained. The first GREEN inner
and QA pass, but its outer runner shadows the container name with a phase name.
That aggregate remains failed. A separate exact-CID cleanup closes the exited
container within its original close deadline. No failed aggregate is relabelled.

A final runtime-only aggregate passes with those exact six ELFs and the unchanged
consumer, after fixing the outer runner's phase-variable collision. Its functional
case takes1.166s under6s. All19 current Scope waits/PIPE EOFs close (13 outer, six
runtime), while two actual Apps waits and native worker wait9/EOF remain separate.
The actual armed-to-final-EAGAIN span is799424473ns; the selected800ms deadline
includes gate delivery. The exact container is waited, removed and absent on the
same daemon. QA is the preceding qualified build, not a repeated test claim.

[Hash-only provenance](evidence/native-connectivity-actor-provenance.json) binds
these cohorts and [published source](../tools/native-connectivity-actor/README.md).
App stdout/stderr are null: actual Apps child waits do not imply PIPE EOF. Scope
wait/EOF receipts, Main's native-worker wait/EOF, independent worker pidfd and
actor join remain separate observations. No process reap implies vendor cleanup.

## Remaining gates

Next integrate the normal Rust Settings client and explicit controls, then normal
app/window and installed service deployment. Keep Source/authentication/full-shell/
capture investigations parked. Missing frozen schemas gate five downstream
compile checks; eleven pass, and all sixteen lock updates preserve registry
versions/checksums while adding only three local packages and the lifecycle edge.
Those checks execute no apps and add no historical integration result.

Wi-Fi association, IP acquisition, DNS/Internet, Bluetooth, tethering/USB, actual
suspend/wake, restart/recovery, arbitrary uninterruptible kernel calls, portable
hardware backends and all physical-phone gates remain open. Retain matched Google
kernel/modules/firmware for Pixel10 first; this host tracer proves no phone boot.
