# Native Settings UI

This UID11050 `org.omarchy.Settings` application presents display brightness and
coarse local network status. Brightness operations use the lifecycle actor's
public credentialed `app.sock`. The app has
explicit 25% and 75% buttons and displays current `observed_percent`; a 75% request
may correctly report 74%. It does not connect to the private brightness daemon
or select a backend path. Settings cannot grant itself permission.

The brightness Rust engine owns strict JSON, root:root peer and exact per-packet credentials,
state validation, fresh CLOCK_BOOTTIME expiry, one queued explicit choice, bounded
nonblocking socket I/O and one owned thread. A queued click retains its revision
and original one-second local deadline. An accepted unsent choice now survives
fresh same-revision readonly pending work and waits for fresh usable state before
its first send. A changed revision, lost freshness, expiry or Stop cancels it;
a refusal after first send is terminal. No actor busy refusal is retried. The wire
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

Native default builds use locked/offline Cargo. Explicit matching-target Settings,
appearance and network-status archives each require their SHA256; cross builds
require all three explicit matching-target archives. CMake installs the normal
executable, desktop file and shared appearance QML. The Settings view is a compiled Qt resource; CMake does not install an FFI SDK.

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
backlight remain open. These published GUI images retain the preceding engine.
The repaired Rust archive now passes native x86 and ARM normal Qt build/install
and offscreen component execution. A new x86 touch attempt fails the first75%
transition: one input, no raw change, cause unknown. The earlier success is not
inherited. The persistent-outcome successor now passes its own native x86 Qt
build/component, then reaches four touch states before the final25 change fails.
A later fresh trace with the repaired Rust actor reservation/readiness now passes
all five states in the same current normal app and installed service. Root reviews
five original Pixel10-profile captures; exactly four inputs include two disabled
no-mutation controls. Earlier failures retain their own evidence and unknown
causes. Cold full-shell startup, ARM UI and hardware remain open. See the
[current trace](../docs/native-brightness.md#current-readiness-owner-five-state-normal-settings-trace-passes).
Future captures must qualify the actual
[Pixel10 profile](../docs/native-phone-display-profile.md); no resized capture is valid evidence.

The accepted-choice repair has a genuine compiled real-socket RED: the ABI accepts
75%, but same-revision pending→usable status produces no Set. Four new cases
cover retention, changed revision, original expiry and Stop before first send.
The frozen focused five (including the existing happy control) pass after the
Rust-only repair, then the original seven groups pass once:11 distinct groups.
All19 client waits/38 PIPE EOFs and the separate wrong-PID writer close; Stop
actually joins within its original200ms. Format, strict all-target Clippy and
normal host/ARM Release archives pass with six strong C APIs each. These archives
are build results; the actual host C ABI fixture executes a debug test binary,
and ARM execution/new normal Qt integration are not inferred from those Rust tests.
Separate native Qt execution is recorded below.

The first post-repair attempt exposed a fixture clock-reference defect after the
correct first Set: it compared sender TTL with later receive time. The precise
test-only successor samples before releasing usable Status, retains the original
cutoff, and fails against the archived original engine before passing unchanged
against the repair. Earlier failed-attempt child receipt gaps remain explicit.
See [hash-only provenance](../docs/evidence/native-settings-pending-choice-provenance.json).

The new platform join compiles the production Settings targets with the repaired
Rust archive, links each matching Settings/appearance archive once, verifies
six/eleven strong C APIs and native loader resolution, then installs root-owned
files. The same component ELF is copied into a guest UID11050 runtime and checked
before/after actual execution; host private directories stay private. Both native
x86 and ARM consumers pass actual C ABI/refusal/200ms join and normal Settings plus
shared appearance QML construction without showing a window. ARM reproduces the
exact signed cached eight-tool addition353→361, preserving all353 versions and
all50 selected Qt libraries/plugins. Its34 command waits/merged PIPE EOFs close
under original bounds; the x86 join closes29 waits/36 EOFs. ARM normal main/window,
Source/brightness composition and graphics remain unqualified. The earlier x86
component exec-path permission failure is retained as setup, with no runtime RED.

The updated x86 app then fails the first75% touch transition within the original
five seconds. Root views initial25 ready→unavailable/pending→25 ready;116 independent
raw samples stay100/max400/actual120, with no mutation events. Session and app stay
healthy, but Qt.clicked/first wire Set/reply are not observed. Only the initial
state passes; the actor/service/VM/container close. No second input is sent. The
lost outcome is a separate usability gap, not evidence identifying the original
failure. See [platform and failed-trace provenance](../docs/evidence/native-settings-qt-platform-provenance.json).

The Rust client now stores the last explicit choice outcome separately from the
current readonly state. A fresh status may restore the current percentage and
usable buttons while preserving “Brightness choice not sent,” “Brightness change
unconfirmed,” or “Change unconfirmed: unavailable.” The last wording records an
observed actor Unavailable after a full Set packet; it does not prove that no
device effect occurred. A new accepted explicit choice or its matching positive
completion clears the old outcome. Refusing a second click does not replace an
active choice, and Denied/Stop retain their privacy and display priority.

The final intent check, single nonblocking Set send and sent marker share one
brief model lock, released before any polling or waiting. This prevents a
concurrent getter from calling an already sent request “not sent.” No write
retry, wire operation, request deadline, readonly cadence or Source policy is
added. New C status constants preserve the current View layout and six exports;
the Qt adapter only maps them to visible text. The previous native ARM build and
five-state touch evidence retain their historical engines.

The outcome qualification preserves two compiled assertion REDs: the final frozen
fixture fails all four new outcome checks plus the intentionally changed
unsent-expiry status assertion on the old engine. The repair passes all15 distinct
requirements across the focused and prior socket/C ABI groups, with25 actual
client waits/50 PIPE EOFs and the separate wrong-PID writer closed. That aggregate
then fails only import formatting; an exact import-order-only successor passes
format, strict all-target Clippy and normal host/ARM release archives. Functional
tests are not repeated after this nonsemantic change. The original five-second
client and200ms Stop/join bounds remain. No deterministic send/expiry interleaving
reproduction or new Qt/GUI result is inferred. See
[hash-only outcome provenance](../docs/evidence/native-settings-choice-outcome-provenance.json).

The current outcome archive now compiles and installs through normal x86 Qt6.12
CMake, with the same unprivileged offscreen component assertions passing. Its
normal app then reaches25→explicit75→unavailable→readonly75 in one app/window and
brightness Main. Root reviews those four original Pixel-profile images plus the
immediate pending frame. A new25 tap fails its original five seconds:116 raw
samples stay300/max400/actual120, with no mutation events and no retry. The original
first-tap-only diagnostics do not capture this later failure, so its outcome and
precise stopping boundary remain unknown. The whole trace fails; no Set ACK is
inferred from rendered status or current raw readback.100 direct waits/107 PIPE
EOFs, separate adopted wait-6, PID1 brightness Main0 and VM/container closure stay
distinct from application success. See
[current native evidence](../docs/evidence/native-settings-outcome-native-provenance.json).

## Current native ARM build and component

A separate [native ARM CMake epoch](../docs/native-current-arm-apps.md#current-settings-and-messages-with-battery)
now builds and installs this current Settings600 frontend with the matching
Settings/appearance archives. The new normal AArch64 ELF is59fb0ef1; it is
separate from the earlier6f9 app. Actual UID11050 component execution passes
layout/refusal/200ms Stop/actual join and constructs both real unavailable
adapters, embedded Settings QML and installed shared appearance QML without
showing a window.17 strong C APIs and native loader resolution match. This does
not execute the normal main/six arguments, a positive brightness service, Source
or visible UI; default ARM Cargo and combined-image execution remain open.
See [hash-only evidence](../docs/evidence/native-current-settings-messages-arm-qt-provenance.json).

## Read-only local network card

The normal view now includes “Local network status”, interface/address-record
counts and “Counts do not indicate internet access.” Missing or stale observations
show dashes. These are public coarse counts, with no addresses, interface names,
Wi-Fi controls, connection icon or reachability claim. The new required
`networkStatusClient` property is supplied by the real `PhoneNetworkStatusClient`.
Its [Rust engine](../native-app/network-status-client-core/README.md) owns the fixed
root-peer socket, strict six-field parser, nonblocking reads and BOOTTIME expiry;
Qt provides scalar bindings and shared Omarchy typography, spacing and colors.

The normal launch arguments remain the six listed above. Native default CMake
uses locked/offline Cargo for the new dependency. An explicit build additionally
selects `POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE` and its required
`POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE_SHA256` for the matching target.

Actual compiled missing-card RED precedes the card. The permanent
`settings-network-card-unavailable-test` retains the original brightness/ABI/
200ms Stop/join assertions, then requires the four exact plain-text items.
`settings-network-card-service-test` uses the real normal daemon and Rust adapter
in one offscreen QML instance: healthy0/0 → actual service Stop → getter expiry
with Qt events withheld → dashes → fresh service/connection →0/0. Its external
fixture verifies only-loopback kernel state and owns both daemon waits/PIPE EOFs.
The test takes two stdin ordering messages, `L\n` after the first daemon's actual
closure and `R\n` after the fresh service is ready; they are test-fixture controls,
not production network-service requests. Both run as UID/GID11050 with the actual
protected shared appearance QML; neither shows a window.

The current host Qt6.8.2 CMake Release build and these components pass. A separate
native ARM Qt6.12 build/install now links all three matching Rust engines and
passes the unchanged dynamic component against the actual installed PID1 service.
PID1 owns both service Mains and their Stops; the fixture parent actually waits
only its component and commands. All54 direct waits/merged PIPE EOFs close within
their original bounds. The normal ARM app3a8b8aac is built/installed only. No
normal-main/window, new screenshot, default-Cargo CMake or phone result follows.
The earlier ARM59fb0ef1 and five brightness screenshots predate this card; their
own source/binary cohorts remain separate. See the
[network-status report](../docs/native-network-status.md).

The later [combined ARM development image](../docs/native-arm-development-image.md)
installs normal3a8b8aac alongside the three native services. The unchanged network
component passes in both fresh boots, including after runtime-only reinstall,
while the original battery/brightness owners remain running. The normal app main
and window are still unexecuted in this cohort.
