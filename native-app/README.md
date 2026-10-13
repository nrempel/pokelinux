# Native phone app adapter

`PhoneRpc` is the C++ Qt adapter for the standalone app. It invokes one
`/usr/bin/omarchy-phone --socket <path> rpc` process per request. Bounded JSON
travels over standard input; command contents are never process arguments.
Rust validates requests, peer identity and replies, and owns durable dispatch.

`request(socket, frame, token)` requires a caller token. The
`completed(status, reply, token)` signal echoes it even for failures without a
reply. These tokens route callbacks; they are separate from command IDs and
service admission tokens. One child can run per adapter instance. The adapter
limits output to 16 KiB, discards child stderr, and enforces a four-second
deadline. It does not retry. Destroying the adapter disconnects callbacks before
stopping the child. Stopping the child cannot cancel already admitted work.

CLI statuses are 0 (validated response), 2 (not sent), 3 (uncertain) and 4
(validated service error). Status 0 does not imply a connected call. Qt checks
that output is a JSON object; Rust performs semantic protocol validation.

## Session and recovery

`PhoneSession.qml` accepts the adapter and socket path. `refresh()` performs
hello followed by `fence`. The fence rotates the service's admission token and
returns unresolved journal entries under the same lock used to admit commands.
A delayed command carrying an earlier token cannot obtain a new reservation.
Work already admitted continues, and its matching receipt remains available.
The daemon session identity stays unchanged. A fence can also reject another
surface's unadmitted request; clients must never retag and resend it.

Readiness requires a validated fence response and no unresolved journal entries.
The app stores its returned admission token for future command submissions.
Read-only `recovery` remains a snapshot query and cannot establish this boundary.
Any session failure clears the admission token and service identity. Callback
routing rejects completions belonging to another query.

`recoveryKey` identifies the displayed journal, command and outcome.
`acknowledge(key)` accepts only that exact key for an unknown receipt; pending
work cannot be acknowledged. A validated acknowledgement starts a new fence.
The original receipt remains unknown with `acknowledged=true`; nothing is
replayed or changed into success.

## Offline verification

Run the isolated native ARM64 Linux suite with Qt 6.8.2:

```
tools/qt-native-suite.sh <absolute-binary-dir> <absolute-enrollment-dir> <absolute-seed-executable> <absolute-tls-peer-executable>
```

The binary directory must contain the extracted current package executables.
The seed executable is the test-only Rust `seed_pending` example, excluded from
the package. It creates an artificial pending journal entry without dispatch.

Forty-three behavior cases pass (fifty-one Qt entries with setup and cleanup). They
cover packaged daemon/CLI integration, output bounds, deadlines, session
barriers, unknown recovery, active teardown, callback correlation, the native
clock, action guards, and the complete Linux answer/dial path. Evidence is in
`artifacts/rust-native-local/qt-tests.log`, `qt-run.json` and the tested source
and binary hash lists. Test containers have no external network access.

`qt_android_peer` is a disposable test-only Rust executable. It creates fresh
mutual-TLS enrollment and accepts repeated observation reads and exactly one Answer followed by one Dial.
It checks the expected call state/revision/action and offer, then
returns synthetic Android receipts. The QML integration test uses the real
adapter, extracted Rust CLI/daemon, private journal and authenticated transport.
This verifies the Linux boundary; the peer is not an Android framework emulator.

This is app logic, not the completed smartphone app. Rendered screens,
Omarchy themes, the matching shell plugin, installed app packaging and
Android-backed UI tests remain unfinished. See
`docs/pro-native-action-review.md` for the remaining action-controller matrix.

## Native actions

`PhoneActions.qml` lives at app scope with the shared session and adapter.
`observe("controls")` and `observe("dial_status")` fetch typed observations through
the Rust client. `control(callId, action)` supports the actions in the current
observation; `dial(number)` captures the current dial offer. Both claim the
local intent guard synchronously, copy exact authority into a new command, and
clear observations before starting the process. Double presses cannot create
another intent. The original command ID and journal epoch survive presentation
invalidation and uncertain results.

Observation age starts before the read request. Each action checks Linux
`CLOCK_BOOTTIME` directly, including time spent suspended, with a two-second
maximum age. A timer only updates presentation; it is not the action guard.
Session changes discard queued observations. Notification wiring is now tested
on a private D-Bus instance. Actual host and Pixel suspend are not yet tested.

`lookup()` reads the existing command receipt, never resubmits. A missing receipt
is still unknown until a new barrier and subsequent lookup can exclude a delayed
unadmitted command. `recover()` now establishes that boundary through the shared session before
looking up the original command. A changed journal, failed barrier or previously
observed receipt that disappears remains unresolved. Accepted framework invocation does not mean a call is
connected. A new action requires a fresh observation after a terminal receipt.

The callback race test first demonstrated that `busy=false` could allow a new
observation before the previous completion was handled. Availability now also
requires no active job, preserving the original completion's owner.

## Live uncertainty recovery

Recovery uses hello, fence, then lookup. `PhoneSession` emits
`barrierEstablished` only after its validated fence response is applied.
`PhoneActions` holds its action guard throughout this sequence. A missing
receipt can become `not_sent` only after that barrier and only if neither a
previous receipt nor a barrier snapshot ever proved admission. Accepted work
keeps its original ID and journal epoch. A changed journal cannot clear it.

`acknowledge(key)` requires the exact displayed unknown entry for the current
intent. It delegates to the session's explicit acknowledgement, establishes a
new barrier, and reads the durable receipt. An acknowledged unknown outcome
remains `unknown` with `acknowledged=true`, while the local guard can clear.
A lost acknowledgement reply is resolved by recovery reads, without replaying
the command or the acknowledgement.

Two additional real-process tests use the packaged daemon and SQLite journal:
an unadmitted retained intent is resolved after a barrier; a seeded unknown entry
remains blocked until explicit acknowledgement. The retained app intent is set
up by the test; recovery itself uses the actual adapter and service. Fake-RPC
cases additionally cover failed barriers, changed journals, vanished receipts,
accepted/pending/unknown outcomes and lost acknowledgement replies. These are
not Android framework or physical-device tests.

## Host sleep lifecycle

`PhonePower` observes the system-bus `org.freedesktop.login1` manager. It reads
`PreparingForSleep` at startup and owner changes, and consumes authenticated
bus-name-scoped `PrepareForSleep` signals. A newer signal supersedes an older
property read. An unavailable owner, failed read or disconnected bus becomes
unknown power state. Bus connection loss is checked at 500 ms intervals. A full
bus restart currently requires app restart; owner replacement on a live bus is
handled in place.

`PhoneLifecycle.qml` connects this observer to the session and actions at app
scope. Sleep or unknown power state sets `session.suspended`, clears session
readiness and invalidates observations. Refresh and acknowledgement cannot
start while suspended. Owned command completion remains valid and retains its
original intent. Wake waits for an in-flight RPC to finish, then establishes a
fresh session barrier or recovers the unresolved command. It never replays a
command. The normal action path also checks `CLOCK_BOOTTIME` observation age;
notifications do not replace that check.

The private-bus integration test exports a synthetic login manager and uses the
actual C++ observer, QML lifecycle components and packaged Rust daemon/CLI. It
covers startup, ignored foreign-sender signals, sleep, wake recovery, owner
loss/reacquisition and bus disconnect. The test sets up a retained unadmitted
intent; all recovery requests use the real service and journal. A separate
queued-observation test proves that sleep prevents stale controls from enabling
an action. These tests do not suspend a host or prove Pixel wake sources,
Android wake locks, ringing during suspend or a power policy.

The test image includes `dbus-daemon` and an NSS entry for the unprivileged test
UID. Build it with `--build-arg PHONE_TEST_UID="$(id -u)"`; tests continue to run
without capabilities or external network access.

References: [systemd login manager interface](https://github.com/systemd/systemd/blob/main/man/org.freedesktop.login1.xml),
[Qt D-Bus signal subscription](https://doc.qt.io/qt-6.5/qdbusconnection.html).
The observer requests no sleep operation or inhibitor. Final Pixel suspend
ownership remains part of the platform integration work.

## Service health and reconnection

A session with a known service identity attempts a read-only hello every two
seconds when the adapter is idle. Matching journal and service identities keep
the existing admission token. A failed check or different identity makes the
session unavailable and discards old control observations. Health queries do
not rotate the global admission token. Actions and acknowledgement cannot start
while the session owns an unfinished query.

While the host is awake, the lifecycle component checks unavailable sessions
for reconnection every three seconds. Reconnection uses a fresh barrier; an
unresolved local intent uses its original ID and recovery path. Commands are
never retried by these timers. Intervals are scheduling intervals, not hard
failure-detection deadlines: an active bounded RPC can defer a health query.
The separate observation-age check still applies at action time.

The real-process test stops and restarts the extracted daemon with its existing
journal. QML detects the loss and reconnects without a user refresh, verifies a
new service identity with the same journal, and preserves the prior command
outcome. Another test changes the hello identity while controls are cached and
verifies that those controls cannot execute.

Harbour was rechecked after this change and still required Tailscale SSH
reauthentication. The waiting check was stopped. These results are from local
isolated native ARM64 Linux containers; no new Android framework or Harbour
validation is claimed.

## Automatic pending-receipt completion

The app lifecycle reads a pending command's original receipt every 500 ms while
awake and connected. It does not queue reads behind another adapter or session
request. Sleep, unknown power, unavailable session, a terminal result or an
unknown outcome stops this timer. This is receipt observation only; no command
or acknowledgement is replayed. Unknown outcomes still require the explicit
recovery and acknowledgement paths described above.

The answer/dial integration test no longer invokes lookup from its test driver.
QML performs automatic receipt checks through the real packaged client/service
and journal after each authenticated synthetic-peer operation. The test first
failed with the action stuck pending; it now reaches `control_submitted` and
`dial_invoked` autonomously. Neither outcome means a connected call. A separate
lifecycle case verifies no receipt polling during sleep and no further polling
after an unknown receipt. The suite remains 31 behavior cases, with these
existing cases expanded to cover the actual automatic path.

## Independent call and dial observations

The action controller now exposes separate `controlsSnapshot` and `dialSnapshot`
values with independent query-start timestamps. Fetching or failing to fetch
one does not remove the other. Each action checks the relevant snapshot's age
at press time. A newer dial snapshot cannot extend an old call control's life.
A command, session change or Android generation change invalidates the relevant
shared authority; a generation change discards the other snapshot before the
new observation is installed. There is no compatibility alias for the old
single-observation interface.

The authenticated integration peer returns a busy dial status between the
incoming-call observation and Answer. The real native app/service path retains
the incoming controls and submits the original call revision correctly. Unit
cases additionally cover a failed dial read, independent expiration and Android
generation replacement. All 35 behavior cases pass. This supports simultaneous
call-card and dial-form presentation; it does not prove background ringing,
Android framework execution or Pixel hardware behavior.

## First deterministic standalone view

`PhoneView.qml` implements the selected stacked layout with Qt Quick Controls:
incoming call card, state-dependent Answer/Decline/End, a validated number
field, Call, and an optional keypad. `IntentButton` captures its context on
press and rejects release after that context changes. The view accepts a theme
object; it does not copy Quickshell internals or contain a fixed production
palette. The current palette values used by tests are synthetic fixtures.

The software-rendered native QQuickView test drives Qt touch and keyboard
activation, changes call identity between press/release, verifies disabled
dialing, checks the dialable input hint, and switches dark/light palette values
without losing the entered number or submitting another command. Screenshot
artifacts: `phone-incoming.png` and `phone-incoming-light.png` in the current
native evidence directory. Both are labeled as synthetic QML test data.
No QML warnings remain in the passing run.

This is a tested presentation component, not a packaged application. Its
rendered input uses a fixture action model; earlier service tests exercise the
real action controller separately. Full view-to-Android integration, automatic
observation scheduling, Omarchy theme file/event loading,
standalone entry point and installation remain unfinished. Input-method
composition, virtual keyboard occlusion, rotation and assistive-technology
interaction still need installed graphical tests. Offscreen synthetic touch is
not Hyprland/Pixel touch hardware proof.

## Visible recovery and actual touch acknowledgement

The view now offers one context-bound recovery button. It selects Check result
or Acknowledge uncertainty from the current session/intent state. For a retained
local intent it delegates to the action controller; startup-discovered journal
entries use the session recovery path. A changed recovery key during a press
cancels release rather than acknowledging a different entry.

Graphical fixture tests verify that these gestures invoke only recovery or
acknowledgement, never a call command. The recovery screenshot uses synthetic
incoming-call data and is labeled accordingly. A separate real integration test
injects the actual session/actions/CLI into the same view, seeds an unknown
journal entry without dispatch, and uses Qt touch input to acknowledge it. A
subsequent real CLI lookup proves the durable receipt is still unknown and is
acknowledged. No new command ID is created. The rendered model is destroyed
before its service state at teardown; TypeError warnings fail this test.

This proves the visible acknowledgement through the production local adapter
and journal. Visible Answer/Dial through Android, full application packaging,
real Omarchy theme loading, and Pixel input/power behavior remain unfinished.

## Rendered touch actions through the authenticated transport

The graphical integration now injects the real PhoneSession, PhoneActions and
PhoneLifecycle into PhoneView, with the extracted Rust CLI and daemon. Qt touch
events activate Answer; keyboard events enter the fixture number 5550100 and a
second touch activates Call. Automatic journal lookup resolves both receipts.
The authenticated synthetic peer verifies the call revision/state/action and
the dial generation/offer/number, accepts exactly the expected five operations,
and exits successfully. Distinct command IDs and the exact receipt outcomes
are asserted. The suite passes without QML warnings.

`phone-incoming-tls.png` captures the view using the real local adapter and an
incoming observation received from that synthetic TLS peer. It is still labeled
synthetic data. The rendered test now schedules observations automatically. A standalone
installed entry point, real theme loading,
real Android framework UI execution, Wayland input and hardware remain outside
this proof. `control_submitted` and `dial_invoked` do not establish call connection,
carrier IMS, ringing, or audio.

## Open-app observation scheduling

`PhoneObservations.qml` alternates call controls and dial status at 500 ms
scheduling intervals. It requires an explicit `active` binding from its owner,
a ready session, and an available action controller. It does not queue work
behind an active request, dispatch call commands, or acknowledge uncertainty.
It stops new reads while inactive, asleep, unavailable, or holding an unresolved
command. Hiding the owner invalidates observations, so an in-flight reply cannot
restore controls after the view is hidden. Returning to a ready active session
starts with call controls.

A still-fresh snapshot remains visible during its own refresh. Its original age
still applies, and actions are unavailable while the adapter owns the read. A
failed read discards that snapshot; it does not erase the independent other
observation. Component tests cover alternation, unresolved-command blocking,
sleep/hide with in-flight replies, and preservation during refresh. This adds
four behavior cases, for 43 total.

The scheduler now participates in the rendered TLS integration test; an
installed app entry point is still required. CPU/wakeup cost remains to be
measured. This component is open-app
refresh only; background ringing requires independent service-owned observation
and notification delivery, which remains unfinished.

## Automatic rendered integration checkpoint

The graphical Answer/Dial test now creates PhoneObservations and no longer calls
`observe()` from its driver. It waits for automatically received controls and
dial state, then supplies only user input: Answer touch, number keystrokes and
Call touch. Production lifecycle code performs receipt polling.

The test peer now allows up to 32 observation/command operations while enforcing
one Answer followed by one Dial, distinct IDs, required prior observations and
exact command fields. Its synthetic scenario ends the incoming call after
Answer so the next dial offer becomes available. Repeated observations are
allowed; repeated or unexpected commands fail. It reports two authenticated
commands verified only after both checks succeed.

All 43 behavior cases pass with no QML warnings. This closes the automatic
open-app observation gap in local rendered integration. It does not establish
background ringing, an installed Omarchy app, the actual Android framework,
carrier behavior, audio or Pixel hardware.

## Installed theme file reader

`PhoneThemeFiles` reads the selected theme's `colors.toml` and optional
`shell.toml` under `~/.local/state/omarchy/current/theme`, plus the optional user
`~/.config/omarchy/shell.toml`. These paths match the pinned Omarchy shell source
in `sim/omarchy-source/shell/Commons/Color.qml`.

It exposes one raw-text snapshot, watches files and existing parent directories,
and reloads after a short event debounce. It follows theme symlink replacement,
atomic file replacement, initial directory creation and user override deletion
without periodic polling. Reads are limited to 256 KiB per file and valid UTF-8;
a missing/invalid required file marks the source unavailable while preserving
the last complete snapshot. Optional file absence is represented by empty text.

The reader resolves the selected directory before reading and checks that the
selection is unchanged before publishing. This prevents mixing files from two
different selection targets during a normal theme switch. It does not claim an
atomic transaction across independent in-place edits of several theme files.
The debounce coalesces filesystem events; it is not a transaction protocol.

This component only loads trusted local appearance data and owns no call state.
The four initial file integration cases were a historical file-reader checkpoint.
Palette/view integration and original Omarchy execution are now qualified below;
installed native service composition remains a separate gate.

The full native suite passes 47 behavior cases / 57 Qt entries after this
addition. The reader's red test failed on missing loading behavior; a second
red test exposed incomplete UTF-8 acceptance by the streaming decoder. Both
are fixed and pass with the full suite. Logs and matching source hashes are in
`artifacts/rust-native-local/`. The run uses fresh Rust debug binaries and a
synthetic TLS peer, not a new installed package or Android framework run.

## File-to-view palette integration

`PhoneTheme.qml` now consumes `PhoneThemeFiles` through `ThemePalette.js`.
It reads the pinned shell's line-based appearance format, including explicit
base colors and numbered-palette fallbacks. Theme shell settings are overlaid
by user shell settings. The app uses popup background/text/border roles for its
surfaces, the base accent and urgent colors, and green/color2 for Answer when
present. Missing positive color uses the accent. Color-reference cycles and
unknown roles fall back without recursion failure.

For this opaque app window, a gradient surface uses its first color stop and
alpha is composited over the theme background. This is deliberate app mapping,
not a claim to reproduce every shell gradient or control token. All palette
values are replaced together. Missing required base colors preserve the previous
palette and mark the source unavailable. The default is a neutral startup
palette until valid installed theme data exists.

The view now uses the original shell's per-token font sizes and spacing. One
owned fixed `/usr/bin/fc-match` child resolves Omarchy's `monospace` alias after
fontconfig changes; the concrete family updates the existing Qt window. Invalid
resolution retains the last family. Admission and completion use `CLOCK_BOOTTIME`,
and uncertain teardown aborts before relinquishing an occupied QProcess. This
qualifies appearance; it grants no service or user-action authority.

The rendered integration test changes real theme files while the existing view
holds an entered number and a synthetic unresolved command. It verifies the
rendered background, retained view identity/number/intent and zero submitted
commands. Screenshots: `phone-theme-files-dark.png` and
`phone-theme-files-light.png` under `artifacts/rust-native-local/`. The call model
in those pictures is synthetic. The separate Answer/Dial test still uses the
real Rust service and authenticated synthetic Android peer.

Palette tests also check explicit accent precedence, user override precedence,
invalid-file recovery, numbered-color fallbacks, unquoted colors supported by
the pinned shell, bounded reference cycles, unknown roles and alpha composition.
The later dedicated original Omarchy proof executes unmodified v4.0.4 theme,
font and text-size commands in an ARM64 Hyprland guest. Eight original/native
comparisons cover two themes, two fonts, token overrides and retained draft/intent
state. Six actual Qt/process cases separately qualify the font worker, including
late completion with timers undelivered and explicitly injected uncertain wait
observations. See [appearance integration](../docs/native-theme.md) and
[owned worker](../docs/native-theme-worker.md).

The renderer uses production view/theme components with synthetic call data.
Installed native service/app composition in the same real Omarchy session, user
plugin activation, complete desktop provisioning, arbitrary themes/wallpapers,
actual suspend and physical display/input remain open. Android tests are reference
evidence rather than a production theme dependency.

## Standalone entry point

`omarchy-phone-app` now embeds the production QML components as Qt resources.
`Main.qml` composes the theme, session, action owner, lifecycle observer,
open-window observation scheduler and call view once per app instance. The
default daemon socket is `$XDG_RUNTIME_DIR/omarchy-phone/bridge.sock`, matching
the user service's runtime directory. The app requires an owned private runtime
directory and a user D-Bus session. Its Rust CLI comes from the same binary
directory as the app, without a PATH or shell lookup.

The session-bus name `org.omarchy.Phone` permits one instance. A second launch
calls `Activate` on the existing process and exits. The exported interface only
shows/activates the window; it exposes no call command, number or journal data.
There is no command-line dialing path. Window focus requests remain subject to
the compositor's activation policy.

CMake installs the executable and `org.omarchy.Phone.desktop`. The native suite
stages these files under a temporary prefix, supplies the matching Rust CLI,
and starts the installed executable on a private D-Bus bus. It checks one owner
across two launches, successful activation, rejection of a missing runtime
directory and absence of QML startup errors. A separate complete-window test
loads the same Main composition, establishes a session through the real Rust
daemon and verifies loss of action readiness during simulated sleep. The
process activation test has no Android service; the composition test uses a
synthetic power source. These are separate evidence scopes.

After a successful suite, the built ARM64 app and desktop entry are retained in
`artifacts/rust-native-local/native-app/`, with hashes in
`qt-installed-app.sha256`. This is a staged CMake installation, not a pacman
installation or updated Arch package. Package dependencies, actual Omarchy
launcher/Wayland activation, full font integration, background ringing and a
current real Android framework run remain unfinished.

## Current complete native runner

`tools/qt-native-suite.sh` now takes five absolute fixture paths: the directory
containing the matching Rust daemon and CLI, a disposable enrollment directory,
`seed_pending`, `qt_android_peer`, and `activate_phone`. Build these binaries
together from the current Rust workspace (`cargo build --locked --offline
--bins --examples`). No old journal schema or fixture binary is supported.

The runner builds the Qt app from current source, configures `/tmp/stage` as
its actual install prefix, and tests that staged installation with the matching
CLI. It supplies the installed D-Bus service directory and Rust activation
harness to CTest. The generated activation descriptor therefore points at the
actual staged executable; using `cmake --install --prefix` alone would not
rewrite a previously configured absolute Exec path. The runner retains the
app, desktop file and activation descriptor and hashes all tested executables.

The synthetic TLS peer now answers the production daemon's independent `watch`
requests as well as the explicit Answer/Dial scenario. Watch replies are
immediate synthetic snapshots; they do not model Android framework service
activation, long polling, modem state, audio or physical suspend.

The current complete run passes all seven CTest executables: 55 behavior cases
plus 14 Qt setup/cleanup entries. Before repair, the full runner failed on the
missing activation fixture and old journal/observation fixtures. This new run
uses current Rust binaries, not the older archive-extracted package. It proves
local ARM64 Debian/Qt integration and does not prove installed Arch/Hyprland
or Android behavior. Evidence: `artifacts/rust-native-local/qt-runner-result.json`.
