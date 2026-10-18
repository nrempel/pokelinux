# Isolated real QML preview input and PCM test

This authored test joins real unshown SoundsView mouse input, the normal Rust
client and lifecycle/audio owners, and independently measured virtual PCM. It
is not a production launcher or a normal visible-app test. Run only in an owned,
dedicated network-none container or disposable guest with no host devices,
D-Bus, audio socket or services shared into it. The explicit root fixture creates
only its private synthetic graph, profile and app directories.

Build the two test Rust binaries with the selected current repository and its
offline cache; normal audio and lifecycle-launcher remain separately built
production providers:

```sh
cargo build --manifest-path tools/native-audio-preview/fixture/Cargo.toml \
  --locked --offline --release --bin preview-actor --bin preview-qml-assert
cmake -S tools/native-audio-preview -B "$build" -G Ninja \
  -DPOKELINUX_SOURCE_ROOT="$repo" -DPREVIEW_QT_VERSION="$qt_version" \
  -DSOUNDS_ARCHIVE="$sounds_archive" -DSOUNDS_ARCHIVE_SHA256="$sounds_sha256" \
  -DAPPEARANCE_ARCHIVE="$appearance_archive" \
  -DAPPEARANCE_ARCHIVE_SHA256="$appearance_sha256"
cmake --build "$build" --target sounds-qml-input-client
```

The C++ target links current AudioClient, shared appearance and production
SoundsView resources. It uses exact one-forward send tracing, real native mouse
pairs, unique visual-item discovery and one public layout-polish pass. It never
shows/exposes a Quick window, calls an effect slot or emits a fake click.

Supply an absolute provider JSON path with exactly these fields:

```json
{
  "schema": 1,
  "ELF_machine": 62,
  "roles": {
    "preview-actor": {"path": "/providers/preview-actor", "bytes": 1, "sha256": "<actual SHA256>"},
    "sounds-qml-input-client": {"path": "/providers/sounds-qml-input-client", "bytes": 1, "sha256": "<actual SHA256>"},
    "pokelinux-audio": {"path": "/providers/pokelinux-audio", "bytes": 1, "sha256": "<actual SHA256>"},
    "lifecycle-launcher": {"path": "/providers/lifecycle-launcher", "bytes": 1, "sha256": "<actual SHA256>"},
    "preview-qml-assert": {"path": "/providers/preview-qml-assert", "bytes": 1, "sha256": "<actual SHA256>"}
  },
  "appearance": {"directory": "/providers/appearance", "bytes": 1, "sha256": "<actual PhoneAppearance.qml SHA256>"}
}
```

Replace every example size/hash with the selected actual bytes. Stage the
0644 PhoneAppearance.qml and five matching-architecture ELF files read-only.
The runtime needs existing Python, dbus-daemon, PipeWire/WirePlumber, pw-cat,
pw-record, pw-cli, pw-dump, pw-link and the selected Qt plugins/imports. No
provider search, automatic build, download or package installation occurs.

Inside the dedicated runtime, with original absolute CLOCK_BOOTTIME cutoffs
selected before entry, call the public entrypoint once:

```sh
python3 tools/native-audio-preview/run.py --isolated-fixture \
  --providers /providers/host.json --output /out \
  --work-until-ns "$work_until" --close-until-ns "$close_until"
```

The caller owns container/VM creation, name/CID identity, original outer
deadlines, actual wait/EOF and final absence. This entrypoint creates neither.
Use a fresh empty output directory and private writable /tmp, /run, /opt and
/var/lib; other inputs can be read-only. Reserve the original host8CPU/3GiB
runtime budget. Phase240/270s and controller120/150s clamp caller cutoffs.
Public scope custody is imported from ../native-installed-services/owned.py;
regular stderr closes and adopted waits remain distinct from PIPE EOF.

The fixed fixture uses audio UID/GID11040, Root actor and app UID/GID11042.
It installs protected copies under /opt/preview-case, sockets under
/run/preview-case and policy under /var/lib/preview-case. The one-app profile
is org.omarchy.Sounds with exactly three test-component arguments: app socket,
private control socket and protected appearance directory. The private graph
at /tmp/pokelinux-native-audio has virtual sinks and no fixture Media producer;
normal audio Main owns the fixed four-second preview. No app supplies PCM,
a path, frequency, producer command or PipeWire node.

Four actual route/Play/Stop/new-Play clicks must join the same actor FD to
Playing→Retiring→Idle→Playing. All three full paired PCM windows share ONE
original six-second cutoff beginning at observer readiness before Play1,
including active controls/status/telemetry/identity observations. Each sink and
stage discards9600 frames then measures24000. Tone, actual old-producer/graph
retirement, full quiet and a distinct new explicit producer remain independent
requirements. Native positive assertion runs only after controller cleanup and
phase wait/PIPE EOF/handle closure/adopted ECHILD. Setup/control failures and
compiled assertion101 are distinct; no implicit retries or deadline renewal.

An ARM invocation is separate: build/select AArch64 ELF183 providers and target
Qt/PipeWire closure, then call the same entrypoint inside its separately owned
guest using /providers/arm64.json. Host binaries/SDK/results do not qualify ARM.
This child-owned fixture also does not substitute for installed PID1 service
custody. The public locked/offline Cargo and explicit-archive CMake commands now
pass, followed by a host run of this actual entrypoint using all three fresh test
ELFs. Four real QML clicks and 144,000 measured frames pass under one original
six-second cutoff; the final native assertion runs only after full cleanup. The
earlier prebuilt-role entry remains separately recorded. The C++ consumer is a
test fixture; production policy remains Rust.
Separate host and installed ARM evidence remains in
[the preview report](../../docs/native-sounds-preview.md).
No visible window, normal Main, display profile/capture, theme switch, full
shell, physical audio or phone result is implied.
