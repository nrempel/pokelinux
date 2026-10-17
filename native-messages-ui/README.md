# Native Messages UI

The [Rust client engine](../docs/native-messages-rust-client.md) owns requests,
validation, models, pagination, generations, scheduling and worker-lifetime
decisions. Qt keeps its native socket/thread edge and QObject/QML presentation.
The normal UID11010 QtQuick app reads bounded local history through the lifecycle
actor and receives schema3 appearance through `PhoneAppearanceClient` and the
installed `PhoneAppearance.qml`. Main and CMake select that shared typed path;
no HOME theme-file reader or fallback is selected. Appearance availability does
not grant Messages access, and Messages denial remains distinct from unavailability.

The fixed installed profile supplies six absolute arguments in this order:

| Argument | Meaning |
| --- | --- |
|1|Private UID11010 HOME|
|2|Private UID11010 runtime|
|3|Wayland path; the actor supplies its fresh restricted per-app listener|
|4|Authenticated public appearance socket|
|5|Session actor `app.sock`|
|6|Protected installed appearance-QML directory|

Normal main requires exactly these six arguments and real/effective UID11010.
The desktop entry calls the fixed native `messages-launch` client at
`/run/pokelinux/session/launch-messages.sock`; the request supplies no grant or
executable. The session owner's explicit history mode supplies the existing
Projector; the normal target does not attach an incoming backend or send messages.

```sh
cmake -S native-messages-ui -B messages-build -DCMAKE_BUILD_TYPE=Release
cmake --build messages-build
DESTDIR="$PWD/messages-stage" cmake --install messages-build --prefix /usr
```

CMake installs the normal app, desktop file and shared `PhoneAppearance.qml`.
`MESSAGES_ISOLATED_OBSERVER` defaults off; its capture controls belong only to
explicit disposable qualification builds.

Current host qualification builds/stages the full Release app with Qt6.8.2 and
warnings as errors. Nine frozen C ABI groups, seven original worker controls,
six current-Qt modes/40 rows, five real-thread ordering groups and13 permanent
CTest cases pass. All29 shared appearance cases are rerun. Actual UID11010
clients, installed QML, live appearance and denial/literal-text controls pass
without a window, as do four normal-main argument/UID refusal controls. The
successful six-argument normal-main GUI path is separately qualified in the
[later prepared x86 session](../docs/native-messages-prepared-tracer.md).

Native default builds invoke locked/offline Cargo; dependencies must already be
cached. An explicit archive requires its SHA256, and cross builds require a
pinned matching-target archive. A separate manual ARM Qt6.12 build and component
run now pass. A later native ARM production CMake Release build installs the normal
app and protected QML with the explicit matching archive and passes the nine-group
consumer. A normal ARM window and ARM default Cargo remain open; see the
[CMake/install result](../docs/native-messages-rust-client.md#native-arm-production-cmake-build-and-installation).
Those ARM builds and earlier prepared captures retain the pre-appearance-Rust-port
cohort. The [shared appearance port](../docs/native-appearance-rust-client.md)
separately qualifies fresh host full-app/default-Cargo/install checks. A
[new prepared Messages session](../docs/native-messages-appearance-rust-tracer.md)
now passes the same four visible groups with both Rust engines, one app/window
and original native captures. Direct normal QWindow/DPR and the current ARM Qt
cohort remain open. CMake compiles both public headers; it does not install an FFI SDK.

Explicit provider selection, including cross builds, needs BOTH matching-target
archive/SHA pairs (replace the paths and digests with the selected artifacts):

```sh
cmake -S native-messages-ui -B messages-build -DCMAKE_BUILD_TYPE=Release \
  -DPOKELINUX_MESSAGES_CLIENT_ARCHIVE=/absolute/libpokelinux_messages_client.a \
  -DPOKELINUX_MESSAGES_CLIENT_ARCHIVE_SHA256=ACTUAL_MESSAGES_SHA256 \
  -DPOKELINUX_APPEARANCE_CLIENT_ARCHIVE=/absolute/libpokelinux_appearance_client.a \
  -DPOKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256=ACTUAL_APPEARANCE_SHA256
```

The permanent native Qt regressions use authored root peers in a disposable test
environment. Build and run their13 CTest cases there:

```sh
cmake -S native-messages-ui/worker-tests -B messages-worker-build -DCMAKE_BUILD_TYPE=Release
cmake --build messages-worker-build
ctest --test-dir messages-worker-build --output-on-failure
```

The original seven clock/wait worker controls retain their separate classifier
in `tools/native-messages-ui-worker-test.py`; expected abort controls are not
ordinary exit-zero tests. The full [report](../docs/native-messages-rust-client.md)
separates actual C ABI, Qt, default-build and hardware evidence.

See the [session composition report](../docs/native-messages-session-composition.md)
for the separate current-owner release completion, host components and historical
evidence boundaries.
The later prepared native x86 session qualifies the normal launcher/current
actor/per-app Wayland/real Source/grant→read/live theme/revoke path. Cold startup,
source-loss composition in this current app, ARM window availability and phone
gates remain open.
Any future visual run must qualify the actual [Pixel10 display profile](../docs/native-phone-display-profile.md)
and native capture dimensions before presenting a screenshot.
