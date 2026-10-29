# Native Permissions app

A focused Sounds audio permission surface: Allow and Revoke go through the native
lifecycle authority. Rust owns the authenticated request, deadline, cached policy
version and confirmation state. Qt/QML renders the shared Omarchy appearance and
forwards activity and clicks through a narrow C ABI.

The normal executable requires UID/GID11045 and exactly six absolute arguments:
private home, private runtime, owned Wayland socket, appearance socket, lifecycle
app socket and shared appearance directory. Install it through the session owner’s
explicit scoped controller profile; launch through `permissions-launch` and the
installed desktop file. Neither the desktop request nor opening the UI grants access.

The client uses nonblocking Unix SEQPACKET with root peer/packet credentials and
900ms BOOTTIME request deadlines. There are no background worker threads. An
explicit choice can retire an in-flight read-only refresh while its cached policy
version remains fresh; it cannot replace or replay a pending write. Focus loss,
expiry or a missing reply clears confirmation. Regaining focus reads current policy
without repeating a write. Reallow never sends Play.

## Build and stage

With Qt6 development packages and the locked Cargo dependencies cached locally:

```sh
cmake -S native-permissions-ui -B build/permissions -DCMAKE_BUILD_TYPE=Release
cmake --build build/permissions --parallel
DESTDIR="$PWD/permissions-stage" cmake --install build/permissions --prefix /usr
```

CMake builds the Rust clients offline. Cross builds require matching-target static
archives and their SHA256 values; see `POKELINUX_PERMISSIONS_CLIENT_ARCHIVE` and
`POKELINUX_PERMISSIONS_CLIENT_ARCHIVE_SHA256`, plus the shared appearance CMake
contract. The UI stages its executable, desktop file and shared appearance QML;
the session-owner Makefile stages the separate desktop launcher.

[Evidence and limits](../docs/native-permissions-ui.md): credentialed Rust tests
and actual unshown QML button clicks pass. They do not prove the full Omarchy
consent/audio flow, phone operation or native display-density qualification.
