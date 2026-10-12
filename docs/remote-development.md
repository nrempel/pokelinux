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
Binder qualification remains x86-64 Android emulator evidence.

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
