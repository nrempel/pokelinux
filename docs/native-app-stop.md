# Owned graceful normal app Stop

An explicit normal lifecycle Stop now fences data and withdraws the display
context before giving an already Running app one bounded TERM grace. A cooperative
actual Qt client exits normally and joins its production client thread. A real
TERM-ignoring app remains owned and fenced while snapshots/repeated Stop respond,
then receives KILL and an actual owner wait. Failure, cancellation, explicit
Terminate and Preparing-app retirement remain forced.

The final host selection passes53 default tests and13 separately selected service
cases, including the earlier retained Messages/restoration regressions. This is
native Rust lifecycle behavior on Harbour's host kernel with a private QCore
consumer; it is not normal GUI/original compositor, genuine authentication or
phone proof. [Bounded provenance](evidence/native-app-stop-provenance.json) separates
source guarantees, actual host results and compile-only ARM artifacts.

## One owned retirement

Each slot retains its Child and display Context through Grace or sticky Forced
retirement. First normal Stop of a Running slot fixes an absolute BOOTTIME deadline
at `min(now+500ms, original_actor_cleanup_until-500ms)`; an exhausted cap forces
immediately. Grace does not renew on repeated requests or polls, and TERM is
attempted once. Only actual Child `try_wait` clears the child; a signal, elapsed
deadline, dead pidfd or ESRCH never releases capacity. A context remains owned
while occupied, even after the app is reaped. Occupied Drop still aborts.

The main loop polls app retirement without blocking on waits. An intentionally
stopped Context can return Terminal while still occupied; this does not cancel
normal app grace. It is still polled for closure, and a context error forces
retirement. Final fallback forces retained apps before draining any other process
owner, within the original two-second actor budget. No extra cleanup deadline
is granted after expiry.

Only the first service Stop decides clean/uncertain storage shutdown, its operation
and deadline, app retirement and reply. Further Stop requests return that memo;
they cannot replace the first shutdown or start a fresh app grace. The first
reply is admission of retirement, not a cleanup/wait or durable-close receipt.

Forced records attempted retirement. Its first withdrawal/signal failure retains
ownership and returns an error; a later direct call may return Ok without retrying
that failed signal. Such Ok means accepted already-retired state, not signal
delivery or process completion. The first service Stop retains its initial error
through the memo. Unresolved custody still reaches the original abort boundary.
No error-injection runtime result is claimed here.

## Actual RED and GREEN

Both initial actual failures close normally before assertions: existing Stop
produces an actual actor-owned signal9 wait instead of Qt wait0, and kills the
TERM-ignoring app before grace. Review strengthens the latter before repair:
first200ms must stay living with actual protected reads denied and responsive
snapshots/Stop. Repeated requests then continue until exact child death, bounded
by850ms from the adjacent first-Stop timestamp. Successful probe gaps including
death observation must stay below100ms. This would fail a deadline renewed by
each request. The final baseline reproduces both failures with these unchanged
assertions; a forced-Terminate control already passes.

After the production repair, all12 cases pass. A later coverage-only addition
uses an authored Ready→Terminal Context that remains occupied. The actual Qt child
exits and the policy worker closes, while Snapshot still responds and the actor
retains the Context. Only the fixture's later release allows full closure. This
is an authored display contract with an actual child, not a display SDK/pathname,
Wayland or compositor cleanup result. Production bytes are unchanged between
that12-case run and the final13-case selection.

| Final host result | Observation |
|---|---|
| Default lifecycle suite |53 pass,13 explicitly ignored,0 filtered |
| Separately selected service suite |13 pass,16 successful actor case reports |
| Qt normal Stop |Actual exact-app owner wait0; production destructor/QThread wait path reached |
| TERM-ignoring app |Living first200ms, protected reads denied;34 repeated probes, death observed511.570189ms, max probe gap16.248268ms; actual owner wait9 |
| Explicit Terminate |Actual wait9; death observed4.130655ms, actor remains healthy |
| Held Terminal Context |Actual Qt child closure24.606408ms; Context retained after child/policy-worker closure, then released; actual owner wait0 |
| Checks |Formatting, strict default/all-target/all-feature Clippy pass; ARM64 daemon/library crosscompile only |

The fixed500ms deadline and one-TERM property are source guarantees. Runtime
observations qualify the stated200/850/100ms bounds; they do not independently
count TERM deliveries or measure its exact kernel timestamp. A private capacity4
channel witnesses the actual normal Apps::poll Child wait for the exact app PID;
the fixture never becomes a second app reaper. Four Qt epochs have owner wait0.
Independent enclosing test-parent waits0, EOF and proc absence pass for all13.
The owned CLI returns0 and the same daemon freshly reports the exact container
absent. Root verifies98 source and18 runtime bindings.

Source100ms, policy IO1s, authentication-attempt20s, per-test15s, outer40s and the
actor cleanup2s remain unchanged. The production Qt client/helper ELF is unchanged
from the preceding client checkpoint; the real destructor's original500ms budget
is not renewed by actor grace. Host process closure is not vendor cleanup.

## Remaining gates

Preparing-launch Stop, forced upgrade during an existing grace, injected context/
signal errors and a real hung Context need separate runtime controls. The authored
Terminal Context does not qualify the original display SDK or secure presentation.
Normal installed QtQuick/QML/Wayland and the original compositor/locker path remain
open, with Pixel10 geometry/density qualification required before any new capture.

Authentication/typed Source/SMS seed remain authored in restoration cases; there
is no production success constructor or ingress. This change adds no PAM, broker,
Source IPC, new ARM/Google-kernel runtime or screenshot. Suspend/wake and physical
modem/carrier/audio/camera/GPU/Pixel gates remain open. No phone was accessed or
modified. The full Omarchy smartphone goal remains incomplete.
