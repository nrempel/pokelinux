# Remote development

Use a Linux Git checkout with Docker and KVM for the offline suites. Retain build
caches and separately obtained private inputs when relocating a checkout. Build
hosts are replaceable validation capacity, not phone runtime dependencies.

Codex can run on the build host. Install it with the official standalone installer,
sign in using `codex login --device-auth`, then start supported remote access with
`codex remote-control start`. Installation, account authentication and desktop
pairing are separate steps. Never commit credentials or expose an unauthenticated
app server. Keep host-specific paths and private artifact inventories outside Git.

Read `AGENTS.md`, `docs/smartphone-goal.md`, `docs/device-portability.md`,
`docs/native-radio-owner.md` and `docs/native-radio-integration.md` before continuing.
The native owner fences raw-response publication. Registration worker/death-watch ownership is now implemented. Remaining work includes
installed service supervision, qualified vendor cleanup/re-registration, other
transport operations, candidate/operation deadlines
and timer delivery, native IMS semantics and phone-service
integration. Raw responses are not decoded call state. Java peers remain test
and reference infrastructure. Physical-device behavior remains unverified.

From the prepared checkout:

```sh
bash tools/build-native-radio.sh build
bash tools/framework-suite.sh binder-kernel
bash tools/framework-suite.sh radio-ipc
```

The first command checks formatting, Clippy, x86_64 and ARM64 component tests
under QEMU, and release builds. It passed after relocating the development
checkout. Both Binder qualification commands also passed after the move: six
dispatch tests, six owner scenario markers, registration death, and all sixteen
ordinary IPC markers. The tested binary hash matches the existing owner evidence.
See [relocation evidence](evidence/remote-workspace-provenance.json). These tests
use a prepared Android emulator and real Binder with a synthetic peer. They do not prove real modem, carrier, camera, GPU or
power behavior. No physical phone access is authorized.

Coordinate changes through Git. Review untracked files before staging; private
inputs and raw audit outputs must not be published.

Owned registration and local permanent-hang process reclamation now pass on
Harbour. A durable admission marker refuses registration after restart until
remote cleanup can be established. See
[evidence](evidence/radio-supervisor-provenance.json). ARM64 user-mode QEMU tests
state and journal behavior; native process tests now also pass in an unprivileged ARM64 system-QEMU guest.
The original x86-64 Android emulator evidence remains separate from the newer
[actual ARM64 kernel Binder qualification](native-arm64-binder.md). The latter
uses the unchanged Arch test kernel and genuine retained Android NDK/Bionic
runtime; it does not qualify the matched Pixel kernel, vendor cleanup or modem.

To run the ARM64 kernel process gate after the native build:

```sh
docker run --rm --network none --cpus 3 --memory 4g --pids-limit 256 \
  --cap-drop ALL --security-opt no-new-privileges --user 1000:1000 \
  -v "$PWD:/workspace" pixel-graphics-sim:20261011-b472 \
  python3 tools/native-process-vm.py
```

The build records `owner-test-build-arm64.json`; the guest uses that exact binary.
The VM keeps the prepared disk unchanged with `snapshot=on`, and writes only its
fresh result directory. It is temporary Harbour validation capacity.

## Harbour build capacity and retained outputs

Use the build host's available CPU capacity for independent compilation and test
jobs. Measure aggregate CPU use, affinity, available memory and disk before
raising concurrency. A mostly idle host can still be waiting on a serial protocol,
graphics deadline, model response or storage; extra compilation workers do not
resolve those waits. Preserve exact selected VM and container quotas in each
run's evidence rather than retroactively assigning newer limits.

The current Harbour allocation exposes 16 logical CPUs and about 62 GiB RAM.
Current owned-session build drivers permit 12 CPUs/12 GiB; independent app and
receiver builds permit 8 CPUs/12 GiB each. Their separate ARM64 guests use 4 vCPUs
with 6 CPU/8 GiB container quotas. Coordinate critical graphics epochs and avoid
oversubscribing the original 100ms source lease. No quota change extends a
production deadline or replaces an integration check.

Completed evidence labels and their build targets are immutable. On 13 October,
byte-verified hardlink deduplication of completed private owned-focus targets
preserved 2,953 output paths and exact bytes, recovering about 4.45 GiB. The private
plan and replacement journal remain under ignored artifacts. Never rebuild or
write into those shared target inodes; every new build uses a new label and target.
Active targets, VM images, source inputs and unrelated host workloads were untouched.
Do not prune private inputs or other workloads to make space.
