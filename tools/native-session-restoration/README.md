# Isolated service restoration integration

The fourteen ignored `service::restore_tests::` library tests run the actual lifecycle
event loop, policy storage child, launcher and a persistent UID11010 app read
connection. Source facts and authenticated-and-closed completion are authored test
inputs. The initial six use authored history; two later cases use the actual
Messages Projector with an authored SMS seed. A ninth case launches the unchanged
production Qt client in a private QCore model host. Four further cases qualify
normal Stop, TERM-ignore escalation, forced Terminate and held authored Context.
There is no production authentication producer or new success socket. Genuine
Source IPC, original locker/authentication, PAM and phone behavior remain open. The fourteenth test separately
executes the normal QtQuick GUI in the recorded ARM graphics selection.

Build the ordinary lifecycle daemon, launcher and library-test ELF from one
frozen source selection; the private cfgtest service seam is included in the library tests.
This directory does not build or run them. In a disposable root container using
the recorded Python runtime image, install explicitly:

```sh
NATIVE_LIFECYCLE_ISOLATED=1 python3 tools/native-session-restoration/prepare-root.py \
  --daemon /bins/lifecycle-daemon --launcher /bins/lifecycle-launcher \
  --install /opt/service-restoration
```

Set `NATIVE_LIFECYCLE_ISOLATED=1`,
`NATIVE_LIFECYCLE_RESTORE_INSTALL=/opt/service-restoration`, and
`NATIVE_LIFECYCLE_RESTORE_RESULTS=/results` (an existing writable directory), then
execute the exact recorded library-test ELF with:

```text
--ignored --exact RECORDED_TEST_NAME --test-threads=1 --nocapture
```

All fourteen integration tests are explicitly ignored by default. Record their exact
inventory and selected execution separately from ordinary unit tests. The headless host root
profile needs SETUID, SETGID, CHOWN, KILL and DAC_OVERRIDE after dropping all other
capabilities, a read-only root/source and private writable `/opt`, `/run`,
`/var/lib` and `/tmp`. No privileged container or host directory is needed.
`app.py` supplies a persistent app-driver, an ordinary owned locker identity
child, and the direct UID1000 JSON lock client. Credential drops use Python
groups/gid/uid calls; no PAM or runuser participates.

The first case expects arming alone to preserve denial, then a later exact typed
Source event to cause a real synced false write and an actual allowed history
reply. Without normal service restoration scheduling it must fail this behavior
after closing the service normally. The second case pauses the first real
Replace after sync using existing `pause:1`, admits a new UID1000 lock, then
requires a true write by the same retained worker and clean restart denial. The
100ms Source lease and original one-second storage deadlines are unchanged.

Each case writes a private JSON receipt and prints `RESTORE_SERVICE_BOUNDARY`.
Receipts distinguish body failures from closure failures and retain real app
identity/read packets. Storage pidfd death/proc absence are observed after the
service's owned drain; these do not independently expose a storage wait status.
Failed or unclosed ownership never becomes a passing case. The initial six cases pass
under the [recorded host selection](../../docs/native-session-restoration.md);
production authentication and full installed-app composition remain open.

The later coverage-only selection adds three ordered Source/completion lifetime
cases and one held-clear Stop/storage-reopen case. A private observation barrier
confirms normal Provider→Authority processing without adding auth authority.
Actual Source100ms expiry does not prevent Snapshot response; the loop retires
and reaps the app. Stop owns the stopped worker without driver resume/kill; its
unclean history is refused by a fresh actual storage worker. That final reopen
is not a full actor restart. Production daemon/launcher bytes are unchanged.

Two later `retained_history_tests::` cases add actual Messages pages and same-ID
relock/clean-reopen/fresh-restore. Install both optional helpers together:
`--messages /bins/messages-service --seed /bins/messages-seed-fixture`.
The fixture uses the existing isolated seed process once per retained dataset and
the normal Messages binary only for its writer/read worker dispatch. It runs no
second policy actor. The native lifecycle dev-dependency's facade bridges Provider
crate identities; successful auth remains private cfgtest. That checkpoint inventory is
53 default passes plus8 ignored cases separately selected and passing. See the
[history report](../../docs/native-session-restoration-history.md) for exact scope;
the earlier6 stress cases still use the authored history provider.

The preceding [actual Qt client selection](../../docs/native-session-restoration-qt-client.md)
passes53 default plus9 separately selected cases. Build `qt-client` with Qt6::Core
against frozen unchanged `MessagesClient.cpp/.h`, then install its
`messages-restoration-qt-client` ELF with
`--qt-client /bins/messages-client-model` alongside both Messages helpers. It is
installed as `messages-client-model`; the lifecycle launcher supplies UID11010.
The existing Python driver remains selected for the earlier8 cases. Its `model`
request reports unavailable and never manufactures a Qt model.

The private Qt driver exposes only identity/current public models, not a service
read proxy or auth ingress. This is QCore, not normal QtQuick/QML/Wayland. The
helper implements graceful SIGTERM through signalfd, but at that checkpoint actor
Stop sent SIGKILL, so those tests qualified process closure rather than Qt destruction.
Current model denial is polling-observed, not immediate secure pixel withdrawal.
No new screenshot is produced; future GUI runs require the actual Pixel10 profile.

The latest [normal app Stop](../../docs/native-app-stop.md) selection passes53
default plus13 separately selected cases. The same Qt helper ELF now reaches
normal destruction/owner wait0 through production TERM grace; the real
`--ignore-term-app` Python mode qualifies responsive fenced grace/escalation and
forced Terminate. A private capacity4 witness records the actor's actual normal
Child::try_wait PID/status; it does not give the driver wait ownership. The
additional held Context is an authored Ready→Terminal/occupied contract with an
actual Qt child, no SDK/Wayland/pathname. Initial two runtime REDs and the final
unchanged contract assertions remain separately bound. No new auth ingress,
normal GUI, ARM execution, screenshot or phone access is introduced.

The [normal GUI selection](../../docs/native-session-restoration-gui.md) separately
runs `normal_gui_tests::normal_gui_restores_history_and_stops_gracefully` on Arch
ARM with original Hyprland/Omarchy. Its fixed root-trusted
`/opt/service-restoration/normal-gui.json` contains `original_wayland`, `theme_home`,
`appearance` (absolute bounded paths) and `normal_qt_sha256`. Install already-built
normal Qt, `normal-gui-launch` and the selected current SDK worker as
`pokelinux-messages`, `normal-gui-launch` and `display-context-worker`; validate
these through the normal trusted executable check. The private worker selection
uses only the current matching `--display-context-worker` dispatch. Earlier actor,
Binder or Messages dispatches in that retained binary are never selected.

The normal launcher still drops to UID/GID11010/groups[]. Six normal Qt arguments
remain HOME, runtime, per-instance Wayland, narrow theme-files HOME, actor app
socket and appearance QML directory. `normal-gui-launch.rs` creates one new0600
private diagnostic sink in runtime0700, sets only WAYLAND_DEBUG and directly execs
the fixed normal Qt path. It creates no child, authority API or signal behavior.
It refuses an existing log; provision a fresh runtime per test case. Root binds
postexec ELF SHA/inode, pidfd/credentials and stderr inode, then qualifies actual
window protocol and independent native captures before any preview.

The capacity32 cfgtest witness records exact peer/connection/time/bytes only after
successful actual send, with sticky observer failure through closure. It does not
prove decoding or pixels. The root observer acknowledges exact three-phase
sequence records under the existing results directory while the test drains this
queue. Its60s observation bound does not change any authority/cleanup deadline.
The full normal app first has genuine owner wait15; after its production signalfd
repair the same test/harness reaches owner wait0. All13 existing host cases and
53 defaults still pass. Native captures use1080x2424; direct Qt API, live theme,
secure presentation, GUI same-ID reopen, genuine auth/Source and phone remain open.
