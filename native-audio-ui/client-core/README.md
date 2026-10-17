# Sounds app-local client engine

This Rust static library owns the request stage, compact packet, original
one-second BOOTTIME deadline, reply validation, pagination, accepted UI model and
one copied queued route/reconnect gesture. It reuses the internal Queue and shared
BootClock. Safe engine/protocol/queue/layout modules forbid unsafe code; only the
narrow borrowed-pointer FFI handles C storage.

Qt retains QObject/QML presentation, nonblocking SEQPACKET I/O, native peer and
per-packet credentials, SCM_RIGHTS closure and notifier/timer ownership. Rust
creates no socket, worker, permission or provider lease. Each handle belongs to
one event thread without concurrent call/free; no caller pointer survives a call.
A copied send plan is local intent. Only the native edge reports an actual send.

From the repository root, with Linux Qt6 development packages, Cargo and the
locked Rust dependencies already cached:

```sh
cmake -S native-audio-ui -B /tmp/pokelinux-sounds-build -DCMAKE_BUILD_TYPE=Release
cmake --build /tmp/pokelinux-sounds-build --parallel
ctest --test-dir /tmp/pokelinux-sounds-build --output-on-failure -R 'audio-client-boundary|sounds-client-boundary'
```

CMake invokes locked offline Cargo on every build so Cargo tracks transitive path
dependencies. An existing archive requires both `POKELINUX_SOUNDS_CLIENT_ARCHIVE`
and its actual `POKELINUX_SOUNDS_CLIENT_ARCHIVE_SHA256`. Cross compilation requires
that explicit archive built for the matching target. The obsolete queue-only
header/functions are removed; there is no compatibility alias.

The permanent byte-identical independent six-group C++ consumer exercises the
actual C ABI and clock, with assertions active in Release via target-only
`-UNDEBUG`. The unchanged30-case Qt suite retains real socket/credential checks.
See [the component record](../../docs/native-sounds-rust-client.md) for actual
source/link/loader checks and the fresh whole-engine prepared GUI/PCM pass with
192,000 measured frames. Installed cold startup, ARM and phone gates remain open.
Historical queue-only results remain in their
[original record](../../docs/native-sounds-rust-queue.md).
