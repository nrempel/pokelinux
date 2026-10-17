# Installed audio and appearance services

Production systemd units now select the native Rust audio and appearance owners.
A frozen actual ARM PID1 test first fails for both missing units, after verifying
the original353-package guest. The same probe now passes after explicit image
installation and in a separate fresh boot: PID1 automatically starts both units,
each answers its real IPC, and both Stop with Main exit0 and empty cgroups.
Domain readiness remains unavailable: there is no theme or selected PipeWire
endpoint in this test.
No physical phone was accessed.
A later [private audio-backend deployment](native-installed-audio-backend.md)
separately qualifies a real available backend in a fresh ARM boot, independent
of this original degraded pair. It retains the same audio owner ELF and adds an
explicit VM backend target, private daemons and Rust socket-readiness helper.

The audio [SIGTERM repair](native-audio.md#sigterm-enters-owned-stop) passes its
unchanged real-process regression,25 active tests and host/ARM release builds.
The installed test selects that new ARM ELF, not the older frozen audio provider
used by the prepared normal-window fixture. Appearance retains its separately
qualified current service source.

## Units and configuration

Both units use `Type=exec`, `Restart=no`, `KillMode=mixed` and a five-second Stop
limit. PID1 sends SIGTERM to Main first. Each owner must drain its own work;
manager admission or an empty cgroup does not prove the owner's domain is healthy.
No unit enables itself during staging.

| Input | Installed path and role |
| --- | --- |
| Appearance executable | `/usr/libexec/pokelinux/pokelinux-appearance`, root0:0/0755 |
| Audio executable | `/usr/libexec/pokelinux/pokelinux-audio`, root0:0/0755 |
| Unit files | `/usr/lib/systemd/system/pokelinux-{appearance,audio}.service`, root0:0/0644 |
| Appearance tmpfiles rule | `/usr/lib/tmpfiles.d/pokelinux-appearance.conf`, root0:0/0644; creates the shared `/run/pokelinux` parent without assigning it to one service's runtime cleanup |
| Audio sysusers rule | `/usr/lib/sysusers.d/pokelinux-audio.conf`, root0:0/0644; selects the dedicated `pokelinux-audio` account at UID/GID11040 |
| Appearance site configuration | `/etc/pokelinux/appearance.json`, root0:0/0644, with explicit desktop `home` and `runtime` |
| Audio site configuration | `/etc/pokelinux/audio-profile.json`, root0:0/0644, with explicit supported sources/routes |
| Audio role/backend environment | `/etc/pokelinux/audio.env`, root0:0/0600; mandatory `CALL_UID` and `APP_UID`, plus any explicitly selected private PipeWire connection environment |

Audio owns `/run/pokelinux-audio` and its credentialed public socket. Its owner,
call and application UIDs must remain distinct. Applications receive service
access through that socket; these units do not provision the unfinished PCM/FD
broker or grant access to a private PipeWire graph. No default Pixel routes or
carrier/call identities are invented by installation.

Appearance runs as root and retains its UID1000 collector. Its configured desktop
home/runtime must already have the required ownership and permissions. A missing
theme publishes schema3 `available:false` and `appearance:null`; that does not
supply a graphical login, focus, unlock or permission grant. No session-owner
readiness is synthesized in this headless service test.

## Explicit build staging

Assemble a finite bundle containing the selected executables, unit/support files
and site configuration, with a schema1 manifest mapping each canonical relative
installed path to its exact `bytes`, `sha256` and numeric `mode`. The support
sources are under `native-audio/packaging` and `native-appearance/packaging`.
No examples or host configuration are selected implicitly.

```sh
make -C native-session-owner install-native-services \
  NATIVE_SERVICES_ROOT=/absolute/site/bundle \
  NATIVE_SERVICES_MANIFEST=/absolute/site/manifest.json \
  DESTDIR=/absolute/new/image-stage
```

The target uses the existing authenticated deployment stager. It rejects
`DESTDIR=/`, verifies selected bytes/modes before writing, and performs no user
creation, unit activation or live-root installation. Image preparation separately
installs those staged files, runs the selected sysusers/tmpfiles rules and
chooses enablement. Host staging and native ARM service execution are separate
observations; the original353 guest has no Make package.

The VM site uses `CALL_UID=0`, `APP_UID=11042`, the dedicated audio owner11040,
an explicit virtual `vm-speaker` catalog and an empty UID1000 desktop home.
There is no installed call daemon or working audio device inferred from those
numeric roles. Neither missing theme input nor unavailable PipeWire is hidden by
a fake success response. Pixel audio/backend selection, a working graphical
session, modem/IMS, physical suspend and the complete phone remain open.

## Recorded integration

The unchanged probe first observes both real units as `not-found`, with actual
start exit5 and two failed assertions. Host GNU Make4.3/Python3.12.3 then execute
the production staging target for ten authenticated files. The ARM preparation
guest installs those selected bytes/modes, provisions the dedicated account and
site runtime directories, and verifies the unchanged353-package inventory. This
is host packaging followed by native guest execution, not ARM Make execution.

The first preparation reaches both active owners and independent IPC, then
fails its Stop-result assertion: the immediate final properties show
`ExecMainCode=0` and an empty invocation ID, not a demonstrated Main exit0.
No image or cold pass is retained from that failure. Its successor enables both
units while stopped before the unchanged probe, checking MainPID0 and the loaded
multi-user target's references. The probe then retains actual PID1 code1/status0
for both. This is consistent with the documented
[unit garbage-collection behavior](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Unit%20Garbage%20Collection);
the first failure's specific cause is not independently proved. No production
unit, binary or probe expectation changes supply that correction.

Preparation shuts down with both enabled services stopped. Only after actual
QEMU wait0 is its image layer retained. A distinct boot creates a fresh overlay
over the readonly prepared/raw chain. Before the probe, it performs no install,
reload, enable or start operation. Both units are already active with fresh
invocation IDs. The guest rechecks all ten installed file hashes/modes before
and after the unchanged cold-autostart probe.

Appearance publishes schema3 null/unavailable state after actual missing-theme
collection, with an idle empty worker slot. Audio publishes its exact virtual
catalog and a fresh unavailable observation with no links or applied intent.
Each authenticated endpoint belongs to its actual PID1 Main and expected UID.
Each Stop then records inactive/dead, MainPID0, Result=success, ExecMainCode1,
ExecMainStatus0, empty cgroup and retained-process death. The guest does not
claim direct parent waits for PID1 services. The separate native audio signal
test supplies its own direct-parent/backend-reap evidence.

The successful preparation and cold boots close28 and19 inside direct waits/
merged PIPE EOFs; host staging closes another3. Each VM's outer CLI wait and two
inspection waits remain separate, with regular console logs. Both QEMU exits0,
exact container absence, same daemon and original input closure pass under the
original bounds. [Hash-only provenance](evidence/native-installed-services-provenance.json)
retains the original RED, failed preparation and both successes. Independent
closed review confirms the installed files, compiled sources and both boots. The
[public frozen probe](../tools/native-installed-services) is unchanged from RED.

This is two installed owners in a generic Arch ARM VM. It does not qualify a
healthy graphical phone session, working theme rendering, an audio backend,
physical routes, suspend, modem/IMS or Pixel hardware.
