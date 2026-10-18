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
