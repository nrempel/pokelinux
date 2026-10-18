# Installed native service probe

This is the unchanged black-box probe used for the actual two-unit ARM deployment
RED. Run it only inside the explicitly prepared disposable Arch ARM guest, with
its frozen original353 package inventory and selected production service bundle.
It requires genuine PID1 systemd, root credentials and the exact package map.
It is not a host installation or physical-phone runner.

Copy these three source/input files together into the fixture's readonly source
selection. The enclosing VM runner supplies one original absolute BOOTTIME cutoff
and a fresh private result pathname. `owned.py` retains actual command handles,
waits and bounded PIPE output; no daemon Main is relabelled as a guest-owned child.

```sh
python3 /results/source/service-probe.py --mode start-stop \
  --until-ns "$POKELINUX_VM_ORIGINAL_UNTIL_NS" \
  --output /results/service-start-stop.json
```

Before implementation, both real `systemctl start` commands fail for missing
units and the probe exits1. After explicit image installation, this same mode
checks each service's PID1 identity, independent degraded IPC, actual Stop result
and empty cgroup. `--mode cold-autostart` additionally requires enabled-at-boot
units and never calls start before its checks. The preparation and cold boot
must be different QEMU invocations with separately sealed image/selection inputs.

The test site selects unavailable appearance (no theme) and unavailable audio
(no selected PipeWire endpoint), with an explicit virtual route catalog. It does
not qualify audio routing, a graphical session, permissions/Source, physical
hardware or carrier behavior. See [deployment](../../docs/native-installed-services.md)
for unit/configuration selection and the recorded outcome.

`backend-probe.py` is a separate frozen availability-only scenario. On the
original installed353 image it must fail after a completed real Rust observation
reports `available:false`. With the explicitly selected VM backend, the same
probe requires all four enabled PID1 services, a credentialed private PipeWire
socket, the actual virtual graph and WirePlumber client, then one whole-target
Stop with retained process deaths and successful PID1 outcomes. It requires a
selected `BACKEND-TOOLS.json` containing exact installed tool hashes. It never
requests a lease, route, permission grant, capture or PCM playback. Its package
transaction and separate cold boot are enclosing fixture responsibilities.


`messages-history-probe.py` separately tests the normal production Messages
history-only unit. Its unchanged original353 run fails the actual missing-unit
requirement before an expected-data file or socket is opened. After explicit
raw-only preparation, two distinct cold boots require real PID1 startup,
authenticated admin status, actual writer/readonly database ownership and exact
authored raw→Inbox IDs/provenance/text. Both Stop through the production helper
and require Main/ExecStop0, empty cgroup and retained process deaths. Supply a
private expected JSON with schema1, authored_seed_only:true and raw_rows from
one actual native isolated-seed execution. Never start Messages in preparation;
require Inbox absent. Neither cold boot may reseed or manually start services.
There is no incoming reception, permission grant, app launch or carrier claim.


`battery-probe.py` is the separate frozen x86_64 KVM battery scenario. It requires
the selected Arch 7.2.9 kernel and exact `test_power` module, the installed
production service/unit/configuration manifest, root credentials and PID1
systemd. Run it only in that disposable guest, with this repository's
[`owned.py`](owned.py) alongside it. The recorded boots used the exact probe/helper pair
pinned in the [kernel provenance](../../docs/evidence/native-battery-kernel-provenance.json):

```sh
python3 /results/source/battery-probe.py \
  --manifest /results/source/deployment-manifest.json \
  --until-ns "$POKELINUX_VM_ORIGINAL_UNTIL_NS" \
  --output /results/battery-service-probe.json
```

It loads the real synthetic kernel driver, changes 73%/charging to
51%/discharging and then absent, and requires matching kernel attributes
and credentialed snapshots from one Main. It verifies PID1 normal Stop, actual
reader retirement and empty cgroup, then unloads the driver. The probe performs
no installation, enable or start; those belong to explicit preparation. The
separate cold wrapper performs none of those operations before this same probe.
Boolean controls use `true`/`false`; the raw module getter is recorded while
kernel PRESENT=0 and unavailable/null publication remain required. See the
[battery evidence](../../docs/native-battery.md) for the preserved parser and
fixture failures, repaired seven-test provider and actual two-boot result. This
is neither an ARM/Pixel reading nor app, Source or power-policy evidence.


The current `owned.py` is a later timeout-custody repair. The battery kernel
boots above used historical helper `350577b4…`; their result is not reassigned
to this successor. Five focused real-child cases separately qualify the current
helper: two timely controls, two overdue commands and a shorter first-finish
deadline with regular stderr. Cleanup obtains actual same-parent wait and PIPE
EOF within its separately selected original close budget, while the failed
command's wait/EOF/stderr timing stays failed. Unresolved custody retains the
Popen and descriptors with an explicit error. A regular stderr file is hashed
and closed after actual wait; it is never counted as a PIPE.

```sh
python3 tools/native-installed-services/owned-timeout-tests.py \
  --output-dir /tmp/pokelinux-owned-test-NEW
```

Use a fresh output path. These tests launch only their own finite children and
access no service, power-supply driver or phone. Actual RED, the deadline-field
correction and final five-case GREEN are recorded in the
[hash-only provenance](../../docs/evidence/native-owned-timeout-provenance.json).


The recorded353 generic ARM battery/brightness pair uses the
[historical power consumer](https://github.com/nrempel/pokelinux/blob/286ef79cdbef0ad32efea483dce9adbd1a3ea4f4/tools/native-installed-services/power-observation-probe.py).
The current `power-observation-probe.py` changes only the inventory filename and
labels to393 for the separately qualified seven-service joined image.
The earlier recorded VM selects the historical source unchanged as
`/run/pokelinux-pair-inputs/source/cold-consumer.py`, with `owned.py` alongside.
The input root also contains `bundle-manifest.json`, the original353 inventory
at `source/expected-installed353.txt`, and the separately compiled
`fixture-bin/brightness-readonly-probe`. Only the enclosing preparation installs
and enables the seven-file bundle; this consumer never does so.

```sh
python3 /run/pokelinux-pair-inputs/source/cold-consumer.py \
  --output /results/cold-consumer.json \
  --until-ns "$POKELINUX_VM_ORIGINAL_UNTIL_NS" \
  --close-ns "$POKELINUX_VM_ORIGINAL_CLOSE_NS"
```

The same command on the original image must first report two missing installed-unit
assertions. Only a distinct fresh boot of the prepared layer qualifies enabled
PID1 autostart. Battery must complete truthful unavailable/null state; the real
GPIO backlight must disappear and recover at increasing Client revisions in the
same Main. Both units must Stop normally with absent sockets and empty cgroups.

`brightness-readonly-probe.rs` is the exact formatted source compiled for this
consumer. Select it as a private bin named `brightness-readonly-probe` in a
disposable copy of the current `native-brightness` manifest; keep its normal
features empty and build ARM Release with the retained locked dependencies. It
uses the production typed Client and accepts only an original absolute BOOTTIME
deadline of at most one second. It has no Set mode or device-path input. See
[the pair report](../../docs/native-installed-power-observation.md) for exact
source/artifact hashes, build checks, two-boot results and limits.

## Local network status

`network-status-probe.py` is the exact positive installed-boundary consumer used
for missing-unit RED and fresh ARM cold GREEN. It requires the selected generic
353-package guest, actual PID1, root credentials, the authored fixture payload
and `/results/selection.json`. The caller supplies original work/close BOOTTIME
deadlines and a fresh result path. It performs read-only observations followed by
normal Stop of the selected service; it never installs, enables or starts it.
The [service report](../../docs/native-network-status.md) distinguishes this root
Python consumer from the host Rust reader and offscreen Settings tests.

## Combined ARM development-image consumer

The current `development-image-probe.py` selects the seven-service393-package
cohort: battery, brightness, local-network status and the four-unit audio stack,
plus protected Settings/Sounds assets. Presence-only mode reaches real
installed-boundary RED; run mode passes one fresh installed cold boot with serial
network, power and real-input/PCM audio recovery and all-seven normal shutdown.
The [joined report](../../docs/native-arm-development-image.md) records the exact
cohort and limits. The earlier
qualified three-service353 consumer remains at
[commit286ef79](https://github.com/nrempel/pokelinux/blob/286ef79cdbef0ad32efea483dce9adbd1a3ea4f4/tools/native-installed-services/development-image-probe.py).

Use an explicitly prepared disposable generic ARM guest with PID1/root and
original BOOTTIME work/close bounds. Supply `/results/selection.json` and the
selected `/run/pokelinux-development-inputs` payload, exact393 inventory, Qt50
baseline, fixture binaries and fourteen-file production manifest. Copy
[`owned.py`](owned.py), [`installed_component.py`](installed_component.py) and
[the existing audio closure helper](../native-audio-preview/closure.py) alongside
it. The payload selects the current393 inventory successor of the power probe;
the historical353 pair keeps its own frozen source and results.

```sh
python3 /run/pokelinux-development-inputs/source/development-image-probe.py \
  --mode check-only \
  --output /results/joined-presence.json \
  --until-ns "$POKELINUX_VM_ORIGINAL_UNTIL_NS" \
  --close-ns "$POKELINUX_VM_ORIGINAL_CLOSE_NS"
```

`check-only` collects all seven service and both app-file boundaries before
asserting. It runs before build or installation and does not require a future
normal Sounds artifact. The executed baseline reports missing battery,
brightness and network units and missing app files, with all four audio units
already active. A missing installed-boundary assertion is distinct from setup
failure. `--mode run` additionally verifies the exact installed
manifest and executes existing network/Settings, power and audio consumers.
Only the tested readonly services are explicitly restored between completed
cases; unrelated services must retain their Main identities. Audio runs last
under its original ARM12-second aggregate and six-second window caps. Its Stop
verification keeps the original cutoff, followed by normal Stop of the remaining
three services. PID1 outcomes stay separate from fixture-parent waits.

Installation, enablement, matching VM device tree and fresh boot belong to the
outer fixture. Battery may be honestly unavailable; GPIO readback is not
luminance and network counts are not connectivity. No normal app window,
Source, matched-Pixel or physical-phone behavior is qualified by this source.
