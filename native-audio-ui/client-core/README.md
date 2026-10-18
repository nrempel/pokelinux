# Sounds app-local client engine

This Rust static library owns the request stage, compact packet, original
one-second BOOTTIME deadline, reply validation, pagination, accepted UI model and
one copied queued route/reconnect/cleanup-Stop gesture. It reuses the internal Queue and shared
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
that explicit archive built for the matching target. The normal app also needs
`POKELINUX_APPEARANCE_CLIENT_ARCHIVE` and its
`POKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256` for the same cross target. The obsolete
queue-only header/functions are removed; there is no compatibility alias.

The independent six-group C++ consumer exercises the actual C ABI and clock,
with assertions active in Release via target-only `-UNDEBUG`. Its business
assertions and the30-case Qt socket/credential assertions are retained; fixtures
now supply required `policy_settled` and `preview` State fields, and the C ABI
layout assertion includes four explicit appended fields. These fixture changes
provide no compatibility defaults for the old wire schema.
See [the component record](../../docs/native-sounds-rust-client.md) for actual
source/link/loader checks and the fresh whole-engine prepared GUI/PCM pass with
192,000 measured frames. A later [current ARM build/install epoch](../../docs/native-current-arm-apps.md)
passes the optimized engine crossbuild, native production CMake app/consumer link,
six groups, loader and protected installation with the shared appearance Rust
client. The normal ARM main/window and audio routing are not run. ARM default
Cargo, the full running ARM cohort, installed cold startup and phone gates remain
open.
Historical queue-only results remain in their
[original record](../../docs/native-sounds-rust-queue.md).


The preview source requires an explicit seven-field State. `policy_settled`
describes the selected logical route; `preview` is one of `idle`, `starting`,
`playing`, `retiring`, `completed` or `failed`. A settled selected route with idle
preview is distinct from an interrupted route. Starting is distinct from Playing;
Retiring does not assert quiet or remote child retirement.

Explicit Play requires an idle channel and a settled selected route. Rust copies
that route and an original one-second BOOTTIME deadline. Every native send plan
contains that same absolute integer deadline and a freshly reduced remaining
`ttl_ms`; EAGAIN/EINTR cannot renew it. Play is never queued behind a read-only
request and is never replayed after reconnect. Qt polling only requests status.

Stop is cleanup, independent of a fresh Play grant. It can wait behind one
read-only status under its own original one-second deadline; a fresh negative
status does not discard cleanup; a denied/unavailable envelope fences the channel
to withdraw ownership instead. If Stop cannot share an in-flight effect or
another queued intent, the engine fences the channel and Qt closes the actual
socket to withdraw ownership. Send failure, expiry and transport failure also
close the channel. Cancellation is not remote rollback, quiet or reap evidence.

The later [preview component qualification](../../docs/native-sounds-preview.md)
passes six existing C ABI, seven schema and seven preview groups plus two cleanup cases,
the30 existing Qt socket behaviors plus setup/cleanup, and the unchanged frozen
unavailable-state QML construction test under UID11042. All seven normal targets
build and resolve their loaders with the selected Sounds archive linked once.
The QML window is not shown and no Play/Stop UI action executes. Earlier AutoMoc,
unused-helper and temporary-directory setup failures remain separately recorded;
the source rename and `[[maybe_unused]]` bridge change no assertions.

The later real lifecycle actor/audio join passes one registered/granted Play
through the normal Rust-owned producer to original private PipeWire: 24,000
measured frames per sink contain 440 Hz speaker content and exactly zero earpiece
samples. The producer creation-time port configuration repairs the preceding
channel-layout refusal with the same frozen fixture. This is separate from the
component checks above. Separate native actor cases qualify explicit Stop/quiet/
new Play, natural completion with same-connection `completed`, and permission
revocation with full measured quiet. Regrant plus one explicit reconnect remains
quiet until a new Play. Native app-close/relaunch separately passes retirement,
full quiet and explicit reconnect before a new Play. Backend-worker loss and
explicit recovery remain quiet until new Play on the same app FD. A later
unshown real QML consumer passes route/Play/Stop/new-Play mouse input through
current AudioClient/SoundsView and Rust policy to real actor/audio PCM: three
paired windows, 144,000 measured frames under one original six-second cutoff.
Only test C++ setup changes; production policy and presentation remain unchanged.
A separate current AArch64 crossbuild qualifies all 452 ELF archive members and
15 APIs; ARM linking/execution remains open. Installed ARM, normal windows and
live-theme integration remain open. Historical GUI/PCM and ARM results retain
their original providers.
