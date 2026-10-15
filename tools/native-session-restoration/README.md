# Isolated service restoration integration

The thirteen ignored `service::restore_tests::` library tests run the actual lifecycle
event loop, policy storage child, launcher and a persistent UID11010 app read
connection. Source facts and authenticated-and-closed completion are authored test
inputs. The initial six use authored history; two later cases use the actual
Messages Projector with an authored SMS seed. A ninth case launches the unchanged
production Qt client in a private QCore model host. Four further cases qualify
normal Stop, TERM-ignore escalation, forced Terminate and held authored Context.
There is no production
authentication producer or new success socket, and no Source IPC, actual locker,
normal QtQuick GUI, PAM or phone claim.

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

All nine integration tests are explicitly ignored by default. Record their exact
inventory and selected execution separately from ordinary unit tests. The root
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
helper implements graceful SIGTERM through signalfd, but actual actor Stop sends
SIGKILL, so current tests qualify process closure rather than Qt destruction.
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
