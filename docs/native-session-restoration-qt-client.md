# Actual Qt client through retained session restoration

The unchanged production `MessagesClient.cpp/.h` now runs as the lifecycle-owned
UID11010 app in a private QCore host. Its real Qt thread sends native queries,
checks root reply credentials, decodes actual retained Messages pages and publishes
its normal models. The selected test observes initial and armed denial, later
restoration with two `hello` bodies, empty models after actual UID1000 relock, and
the same two message receipt IDs after clean actor reopen and fresh restoration.
Each dataset is seeded once. The Qt client source is unchanged.

This qualifies a production consumer component, not the normal QtQuick app binary.
There is no QGuiApplication, QML, Wayland window, original Source/compositor or new
capture. Authentication and typed Source remain private cfgtest inputs; the SMS
seed is authored. The actual policy worker, Messages writer/readonly child, app
identity, query/reply bytes and decoded models are real. There is no production
authentication constructor, success ingress or new production feature.
[Bounded provenance](evidence/native-session-restoration-qt-client-provenance.json)
records the separate selections.

## Actual boundary and ownership

The private host links Qt6::Core6.8.2 and the exact production client sources with
strict compiler warnings. It has no linker wrappers, clock/receive injection,
replacement service-query encoder or authored page response. The lifecycle
launcher supplies UID/EUID/GID/EGID11010, groups[] and CapEff0; the helper checks
them before constructing the client. The same actor still owns the sole policy
worker and real Messages provider.

A mode0600 private nonblocking driver accepts one root peer and one bounded
request/reply at a time. It validates peer and packet credentials, refuses extra
ancillary handles and exposes only identity or public model getters on the Qt
event thread. It cannot grant, read the service on the client's behalf, arm or
unlock. The earlier Python helper answers the model request as unavailable; it
does not fabricate a Qt model. The normal client owns its service socket/thread.

The model test requires empty conversations/messages and the normal denied status
before arming and before the later Source event. The armed observation may still
be the cached initial denied model; it does not witness a fresh post-arm exchange.
After the real policy clear it requires one conversation, two literal bodies and distinct canonical receipt IDs.
Actual idle relock syncs pending true and the same Qt process eventually publishes
empty denied models. This is observed polling behavior, not immediate withdrawal
of released bytes, queued callbacks or cached pixels at the lock event.

Normal service Stop owns all child termination/reaping. Both epochs retain policy
and Messages pidfds; actual app/storage/Messages death and proc absence follow the
service-owned drain. Both compare the sole policy worker with its original retained
pidfd. A new actor and Qt process over the same raw/inbox directories require fresh
actual lock admission/private completion/later Source and return the same IDs.
There is no reseed, replacement database or old completion replay.

The helper implements a signalfd SIGTERM path before creating the client thread,
so a future graceful quit can run the ordinary client destructor. **This selection
does not execute that path:** `Apps::retire` uses `Child::kill`, which sends SIGKILL.
Its process closure does not prove the Qt destructor or QThread join ran. The driver
does not independently signal/reap the app to manufacture such a claim. Graceful
app shutdown remains open. Numeric app/storage/Messages wait statuses are not
independently exported; the enclosing test-parent waits are independently observed.

## RED and current results

A compile-only test result-type error is retained as a prerequisite failure with
zero test bodies. The corrected frozen baseline passes the default suite, then
actually fails the new model contract with the existing headless app profile.
The real actor/history/seed setup and normal service closure still complete. This
is missing test composition, not a newly found production client defect.

After selecting the source-linked Qt helper, the exact model assertions pass;
only the app composition constructor changes in that test module. No production
client, authority, service, storage or app lifecycle source changes. The earlier
eight cases also pass in the same pinned Qt runtime image.

| Current selection | Result |
|---|---|
| Default lifecycle host tests |53 pass,9 explicitly ignored,0 filtered |
| Separately selected actual service tests |9 pass,12 successful actor case reports |
| Actual Qt client model test |1 test,2 actor/Qt epochs, identical message IDs after restart |
| Qt host build |Qt6.8.2, C++17, `-Wall -Wextra -Werror` |
| Lifecycle checks |Formatting and strict default/all-target/all-feature Clippy pass |
| ARM64 daemon/library |Crosscompiled only; no new ARM execution or Qt ARM build |

All nine test processes return actual parent wait0, output EOF and proc absence.
The owned CLI returns0; the same daemon freshly reports the exact container absent.
Root rehashes98 selected source bindings and18 runtime bindings. Existing Source
100ms, policy IO1s, test attempt20s, outer40s and per-test15s bounds are unchanged.
The production Qt client's original1s exchange,250ms polling cadence and500ms
destructor budget are unchanged; the destructor budget is not exercised here.

## Remaining gates

Normal installed QtQuick/QML/Wayland and original compositor/locker composition,
genuine whole-owner authentication/broker, graceful Qt app shutdown and secure
presentation remain open. Any new visual run must independently qualify the
Pixel10 output/window profile1080×2424,422PPI and420 logical density before a
native unscaled capture is presented. This headless component has no display proof.

The earlier six held-clear/new-lock/expiry/uncertain-Stop controls still use authored
history and the Python driver. They are not real-Qt stress qualification. Actual
Qt held-clear/expiry, early provider initialization failure, full unclean actor
reopen and whole-actor EAGAIN/completed-write timing need separate evidence.
New ARM/Google-kernel execution, suspend/wake, physical modem/carrier/audio/camera/
GPU and all Pixel hardware gates remain separate. No phone was accessed or modified;
the full smartphone goal remains incomplete.
