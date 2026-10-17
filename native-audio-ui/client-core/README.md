# Sounds queued-intent core

This app-local Rust static library owns one unsent logical route or explicit
reconnect gesture, its original one-second BOOTTIME budget and cancellation.
`queue.rs` and `wire.rs` forbid unsafe code; `ffi.rs` is the narrow borrowed-pointer
C ABI. It creates no socket, worker, permission or provider lease.

Qt copies its current validated actor status/catalog into `PrSoundsFreshV1` and
requests a plan before each actual send attempt. A plan retains its token and
original deadline across EAGAIN/EINTR. Only a complete native send permits `sent`;
closing or refusing the connection cancels. One Qt event thread owns each handle,
all input/output storage and exclusive calls/free; no borrowed pointer survives.

From the repository root, with Linux Qt6 development packages, Cargo and the
locked Rust dependencies already cached:

```sh
cmake -S native-audio-ui -B /tmp/pokelinux-sounds-build -DCMAKE_BUILD_TYPE=Release
cmake --build /tmp/pokelinux-sounds-build --parallel
ctest --test-dir /tmp/pokelinux-sounds-build --output-on-failure -R 'audio-client-boundary|sounds-queue-boundary'
```

Native CMake invokes locked offline Cargo on every build so Cargo tracks changed
transitive path dependencies. An explicit existing archive requires both
`POKELINUX_SOUNDS_QUEUE_ARCHIVE` and its actual
`POKELINUX_SOUNDS_QUEUE_ARCHIVE_SHA256`. Cross compilation requires that explicit
archive built for the matching target; a host archive is not silently selected.

The permanent six-group C++ consumer exercises the real C ABI and clock. The
unchanged Qt suite checks actual app-edge socket behavior. Its target keeps
assertions active in Release with -UNDEBUG. The app linked with this core also
passes the prepared GUI/PCM tracer. See the [execution record](../../docs/native-sounds-rust-queue.md)
for separate scopes and remaining cold installed/ARM/phone gates.
