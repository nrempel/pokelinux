# Persistent connectivity worker fixtures

Authored runtime source for the isolated host [worker checkpoint](../../docs/native-connectivity-worker.md).
The lifecycle case exercises a real `Service` Main library and the normal worker
binary; the held fixture selects `expiry` or `stop`. No proprietary input,
credentials, captured packets, ELF or raw private runtime output is included.
The original fixtures retain their development comments and exact executed bytes.

These manifests deliberately preserve the measured build layout. Copy either
fixture directory to a fresh `worker-fixture/` beside exact copies of
`native-connectivity-control/`, `native-network-state/`, `native-process-owner/`
and `native-kernel-identity/`. Do not build a manifest directly in this nested
repository location or silently regenerate its lock. Use the selected offline
registry/toolchain and exact Cargo `compiler-artifact` JSON to select the
`driver` test executable (`harness = false`, test profile). Build the normal
`pokelinux-connectivity-service` worker binary separately (`profile.test = false`).
A standalone driver is needed because cooperative startup requires the actual
single main thread; the normal Rust test harness thread is not that boundary.

Use a trusted fresh exclusively administered private current namespace, UID0 and
initial NET_ADMIN/SYS_ADMIN/SETPCAP. Set `POKELINUX_CONNECTIVITY_ISOLATED=1`.
Loopback-only inventory is a sanity check, not exclusivity proof. Never run the
fixture in a shared host namespace or against a phone. The controller creates
and retains the second namespace. The short-lived provisioner is actually waited
and output-closed before the capability-free peer starts; Main loses all
capabilities and only the Rust worker retains NET_ADMIN. Both namespaces outlive
all owned child waits and stdout EOFs.

Mount the selected `controller.py` as `/runner/controller.py`, the existing
`tools/native-connectivity-control/peer.py` as `/runner/peer.py`,
`tools/native-installed-services/owned.py` as `/runner/owned.py`, and
`tools/native-audio-preview/closure.py` as `/runner/closure.py`. After the owned
build scope closes completely, invoke the controller with fresh absolute paths:

```sh
python3 -B /runner/controller.py --driver-elf /absolute/selected/driver \
  --worker-elf /absolute/selected/normal-worker --case lifecycle \
  --output /absolute/fresh/output --work-until-ns ORIGINAL_WORK_BOOTTIME_NS \
  --close-until-ns ORIGINAL_CLOSE_BOOTTIME_NS
```

Select `expiry` or `stop` with the held controller. The retaining outer supervisor
supplies the original absolute work/closure clocks and retains its exact
container/CLI custody through actual wait, PIPE EOF and removal. Each case starts
one six-second functional aggregate; it does not reset that cutoff during setup.
Lifecycle has original at-most-one-second actions and an800ms peer quiet cutoff.
Held cases stop the actual worker through a retained pidfd before enqueueing a
100ms command. Expiry samples snapshots without polling for350ms, then resumes
the same worker under the original450ms resume/1250ms monitor cutoffs. Stop fences
an outstanding dispatched command immediately, before polling or resumption.

Only literal trusted fixture principals/grants enter `Service::submit`. This is
not application authentication, OS permission enforcement, real Settings,
installed service startup, arbitrary uninterruptible kernel reclamation, actual
suspend, ARM or phone evidence. The crate's authored-cache/real-child regression
is a separate component test, not privileged kernel startup. See the report's
hash-only provenance for source cohorts, failed setup aggregates and RED/GREEN
qualification; raw nonces and process traces remain private.

## Normal CLI error-context consumer

[`error-context/consumer.py`](error-context/consumer.py) is the exact8290-byte
consumer executed unchanged for the [error-context RED and GREEN](../../docs/native-connectivity-worker-error-context.md).
It runs only the normal compiler-JSON-selected x86_64 Release worker, using its
existing cooperative selector and four trusted arguments. It requires no hidden
worker flag or special capability: the actual parent/child cohort is UID1000,
all capability sets0 and NNP1. No Main or virtual-link setup is exercised.

Stage the consumer as `/runner/consumer.py` with the original `owned.py` and
`closure.py` helpers beside it. Use an owned UID1000, cap0/NNP1 process and a
read-only qualified worker, retaining the outer actual wait. The recorded CLI
controls execute natively on Harbour; their builds use separate containers.
Select a fresh output:

```sh
python3 -B /runner/consumer.py \
  --worker /providers/pokelinux-connectivity-service \
  --worker-sha256 ACTUAL_QUALIFIED_RELEASE_SHA256 --output /out/case
```

Before any parent poll/wait, actual child status/task metadata is retained; an
observed zombie is not called running. One original1s absolute BOOTTIME cutoff
covers natural worker exit1 and merged stderr/stdout PIPE EOF, with output capped
at4096 bytes. A separately selected3s custody cutoff cannot qualify late work.
Only exact unwrapped source errno1/PermissionDenied with full custody qualifies
missing-context RED. Unexpected output, setup or closure faults stay unconfirmed.
The desired contextual Debug structure and detection time must match. Result and
closed wait/EOF receipts are persisted before the final desired assertion.
The Debug grammar is a private component diagnostic, not a new control protocol.

## Persistent idle readiness consumer

The exact [`consumer.py`](idle-receive/consumer.py),
[`controller.py`](idle-receive/controller.py) and
[`worker_trace.py`](idle-receive/worker_trace.py) execute unchanged across the
[revised normal-provider RED and GREEN](../../docs/native-connectivity-idle-receive.md).
The controller's original RED-oriented docstring is preserved with its executed
bytes. The pure consumer performs no I/O and makes its one desired assertion
only after the outer scope has qualified all three case closures.

Stage these as `/runner/consumer.py`, `/runner/controller.py` and
`/runner/worker_trace.py`, beside the exact existing
`tools/native-installed-services/owned.py`,
`tools/native-audio-preview/closure.py` and
`tools/native-connectivity-control/peer.py`. Their hashes are retained in the
[evidence](../../docs/evidence/native-connectivity-idle-receive-provenance.json).
Each of `idle`, `consumed` and `parent-death` needs a fresh trusted Docker
network/PID namespace, network none, root read-only, no host devices and a
retaining outer supervisor. The selected fixture uses 2 CPUs/2 GiB and only
NET_ADMIN/SYS_ADMIN/SETPCAP/SYS_PTRACE setup capabilities; Main drops all
capabilities and only its normal worker retains NET_ADMIN. No physical or
shared host interface is an admissible target.

After actual loader preflight for both read-only normal providers, an owning
runner invokes:

```sh
python3 -B /runner/controller.py --case idle \
  --main /providers/pokelinux-connectivity-main \
  --worker /providers/pokelinux-connectivity-service --output /results/case \
  --work-until-ns ORIGINAL_WORK_BOOTTIME_NS \
  --close-until-ns ORIGINAL_CLOSE_BOOTTIME_NS
```

Select `consumed` and `parent-death` separately with fresh output/namespaces.
The original case is 6 seconds including at most 3 seconds for Main startup.
Inner work/close bounds are 30/40 seconds, clipped to the original outer
180/200-second work/close bounds. The outer runner retains actual create,
attach, container wait, removal, exact CID/name absence, daemon identity and
all command wait/EOF ownership before invoking the pure final oracle. Neither
a case-only success nor stopped-process status qualifies the aggregate.

Only the exact fixture-owned single worker is traced. The tracer reads syscall
metadata, never packet contents or process memory, and detaches at a nonterminal
stop before normal Main's actual terminal wait. One 80 ms empty-idle stop is
distinct from the consumed case's single Apply at the post-publish/post-sleep
`gettid` gate and 80 ms actual 19-byte receive-exit hold. The consumed case must
still produce natural worker exit 1 and `ReceiveCommand`/`TimedOut` after its
unchanged 50 ms envelope and before the unchanged 100 ms action cutoff. The
8192 syscall-stop bound is unchanged. No effect retry, late-frame forgiveness,
renewed cutoff or supervisor-kill substitute qualifies that control.
