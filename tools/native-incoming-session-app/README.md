# Isolated frozen V3 incoming/session/Qt qualification

This trusted fixture composes the existing current core with original Hyprland,
the unchanged normal Qt app and the explicitly authored V3 Binder peer. It has
no production backend selector, physical-phone access, ACK/send/reset or source
revival. The full close/reopen matrix currently fails at the original100ms source
lease during cold Qt reopen. Its six narrow checks are described in
[the report](../../docs/native-incoming-session-installed-app.md).

The first runtime red precedes this fixture source and uses the unchanged incoming
service's absent session provider. This is a desired new boundary assertion, not
an old policy bug. Historical proof labels remain frozen. The current runner's
red branch is a reproduction; only the original chronological selection is
claimed test-first evidence. The required cache-path argument was added after
the selected runtime freeze and has not been rebuilt or replayed as runtime proof.
Historical driver sources and build commands keep their original byte bindings.

Run on the authorized Harbour test host with the already retained pinned images,
readonly public frozen AIDL/Cargo cache, original private Bionic runtime and
existing receiver VM backing. `build.py` requires `--cache-path` naming an existing
mutable Cargo target directory. For reproduction on this retained host, pass
`/dev/shm/pokelinux-incoming-cargo-cache-20261013`; the former main combined disk
target is retired and must never be written. Do not point the build at hardlinked
immutable proof targets. Use unique labels and never overwrite selected outputs.

```sh
python3 tools/native-incoming-session-app/build.py --phase red --label new-incoming-red-build --cache-path /dev/shm/pokelinux-incoming-cargo-cache-20261013
python3 tools/native-incoming-session-app/run.py --phase red --label new-incoming-red-arm --build new-incoming-red-build
python3 tools/native-incoming-session-app/build.py --phase green --label new-incoming-composed-build --cache-path /dev/shm/pokelinux-incoming-cargo-cache-20261013
python3 tools/native-incoming-session-app/run.py --phase green --label new-incoming-composed-arm --build new-incoming-composed-build
```

The expected red exits1 after an actual assertion and reaping. The composed run
also exits nonzero if its current-source availability checks fail. Existing failed
results are evidence; do not rerun until lucky or relax source/deadline checks.
`run.py` verifies every frozen source/bin binding and runs one no-NIC readonly
receiver VM under6 CPU/8GiB. Its guest test facade uses the existing receiver boot
inode; it does not change that image or add production compatibility behavior.
Every source-image preparer must take the same input lock exclusively.

A distinct `--case source-loss` keeps the first stable app window, commits a third
callback before holding the readonly child, unloads the exact original source
plugin, waits for actual reader reap, then tests callback4 under the same writer
and actual peer death. It never closes/reopens the app and therefore cannot
qualify the failed close/reopen matrix. The native service/core build is unchanged.

```sh
python3 tools/native-incoming-session-app/run.py --phase green --case source-loss --label new-incoming-source-loss-arm --build new-incoming-composed-build
```

The dedicated case records BOOTTIME hold start, busy observation, unload invocation,
source failure and actual read reap separately. The original request and arrival
deadlines are enforced by the unchanged native implementation; the harness does
not replace them with an aggregate cap beginning before a read request. Its
hold/close/reap intervals do not measure the exact active ReadStamp start/until
or establish a hard cancellation or cleanup bound.

The root runner alone supplies the inherited source socket/pidfd and exact original
compositor identity; their descriptor numbers are distinct before adoption. The
fixture forwards session diagnostics without changing provider outcomes. The sole
writer uses the existing V3 backend, one durable attempt marker and retained actual
death recipient. The actual readonly worker is supervised independently. These
paths are isolated test tooling and are not installed production service entries.
