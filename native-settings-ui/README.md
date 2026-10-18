# Native Settings UI

This UID11050 `org.omarchy.Settings` application presents one display-brightness
operation through the lifecycle actor's public credentialed `app.sock`. It has
explicit 25% and 75% buttons and displays current `observed_percent`; a 75% request
may correctly report 74%. It does not connect to the private brightness daemon
or select a backend path. Settings cannot grant itself permission.

The Rust engine owns strict JSON, root:root peer and exact per-packet credentials,
state validation, fresh CLOCK_BOOTTIME expiry, one queued explicit choice, bounded
nonblocking socket I/O and one owned thread. A queued click retains its revision
and original one-second local deadline. A changed revision cancels it. The wire
TTL is the floor of remaining local time; the remote actor has its own admission
and Source fences. Local expiry prevents later send/publication and closes the
channel; no uncertain Set is replayed. Readonly status refresh may reconnect.

Every getter samples BOOTTIME. Stop retains one 200ms budget; free refuses until
actual thread join. Qt copies scalar properties, emits change notifications and
forwards button clicks. A false `can_set` means controls unavailable. Only an
explicit `denied` reply produces the access-denied state. Shared
`PhoneAppearanceClient`, its Rust engine and installed `PhoneAppearance.qml`
supply Omarchy colors, fonts and spacing.

The normal installed profile supplies six absolute arguments:

| Argument | Meaning |
| --- | --- |
|1|Private UID11050 HOME|
|2|Private UID11050 runtime|
|3|Fresh restricted per-app Wayland path|
|4|Authenticated public appearance socket|
|5|Lifecycle actor `app.sock`|
|6|Protected installed appearance-QML directory|

Main requires exactly those six arguments and real/effective UID11050. Its Qt
application name and desktop `StartupWMClass` are `pokelinux-settings`. The desktop
entry calls fixed `settings-launch` at
`/run/pokelinux/session/launch-settings.sock`; session configuration and launcher
admission are separate from this application. The normal phone window requests
full screen, without capture or test-control arguments.

```sh
cmake -S native-settings-ui -B settings-build -DCMAKE_BUILD_TYPE=Release
cmake --build settings-build --target pokelinux-settings
DESTDIR="$PWD/settings-stage" cmake --install settings-build --prefix /usr
```

Native default builds use locked/offline Cargo. Explicit matching-target Settings
and appearance archives each require their SHA256; cross builds require explicit
archives. CMake installs the normal executable, desktop file and shared appearance
QML. The Settings view is a compiled Qt resource; CMake does not install an FFI SDK.

Seven frozen real-socket/C ABI groups pass after actual compiled assertion RED:
real UID11050/root-peer status and explicit 75→confirmed 74/actual 72→25; generic
controls unavailable versus explicit denial; eight malformed/ancillary controls
(duplicate, missing nullable field, unknown field, range, canonical deadline,
excess future TTL, SCM_RIGHTS, wrong sender PID); expiry through a fresh getter;
held Set deadline/channel close/readonly recovery without replay; changed revision
while a click is queued; and responsive Stop with actual join. All 14 client
processes and the separate wrong-PID writer are actually reaped; client stdout and
stderr reach EOF. The isolated offline run adds only SETUID/SETGID capabilities.
Formatting, strict all-target Clippy and native Release archive build pass.

A fresh native x86 KVM successor now builds and installs the normal app with Qt6.12
and the two exact Rust archives. The C++ consumer passes header layout/refusal/
actual-join checks and constructs the actual adapters, normal Settings QML
resource and installed appearance QML as UID11050 with unavailable endpoints,
without showing a window. Its embedded view says “Brightness: N%” for current
attributes, including readonly recovery after an unconfirmed Set; it does not
claim that recovery confirms the earlier request. The preceding “Confirmed”
wording build remains separate evidence. Actual linking selects each archive once; all six
Settings and 11 appearance C symbols are strong definitions, and loader resolution
is complete. Installed executable mode 0755 and theme/desktop mode 0644 are root 0:0.
All owned child waits/PIPE EOF, guest shutdown, QMP socket EOF, exact container
absence, same-daemon and input-closure checks pass. The unchanged 367-package guest
and the seven Rust groups retain their separate scopes. Public provenance is
[hash-only](evidence/qualification.json); private raw logs are not published.

The [fixed Settings desktop route](../docs/native-brightness.md#fixed-settings-desktop-launch)
now has separate actual actor/CLI/staging tests. The core installer requires an
explicit Settings launcher and stages its desktop entry; this does not grant
DisplayBrightness or qualify the normal graphical session.

The [prepared Settings touch tracer](../docs/native-brightness.md#settings-touch-unavailable-state-and-recovery)
now passes25→explicit75→unavailable/no displayed percent and two no-mutation
inputs→readonly recovery75→new explicit25 through the real actor and installed
SyntheticTree service, in the same app. Root reviewed five original1080×2424
captures and external Pixel10 profile metadata. The prior failed observer and
single-tap attempts remain separate. Direct normal QWindow DPR/Qt.enabled getters,
all input races, cold full-shell startup, default-Cargo Qt, ARM UI and physical
backlight remain open. An accepted-click/readonly overlap regression is in progress.
Future captures must qualify the actual
[Pixel10 profile](../docs/native-phone-display-profile.md); no resized capture is valid evidence.
