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
The native owner fences raw-response publication. Remaining work includes owned
transport workers, lifetime deadlines, native IMS semantics and phone-service
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
checkout. The other commands use a prepared Android emulator and real Binder
with a synthetic peer. They do not prove real modem, carrier, camera, GPU or
power behavior. No physical phone access is authorized.

Coordinate changes through Git. Review untracked files before staging; private
inputs and raw audit outputs must not be published.
