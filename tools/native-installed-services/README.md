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
