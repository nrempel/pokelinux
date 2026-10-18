# Finite native Sounds preview

One registered/granted host actor client now plays the Rust-owned preview through
original private PipeWire to independently measured virtual PCM. Host Rust and
frontend components also pass. A separate same-session case now proves explicit
Stop, producer retirement, full measured silence and one new explicit Play.
Separate cases now pass natural completion and permission revocation, including
quiet after regrant/explicit reconnect until a new Play. Native app-close/relaunch
also passes retirement, full quiet and explicit reconnect before a new Play.
Backend-worker loss and explicit recovery stay quiet until a new Play on the same
app FD. Actual Qt input joined to PCM remains open. Earlier Sounds GUI/PCM results
retain their own sources.

## Ownership and model

Rust generates fixed four-second 440 Hz mono F32LE PCM at 48 kHz. Applications
supply no file, PCM, executable, node or frequency. The lifecycle actor checks
current app generation, MediaRoute permission and foreground authority before
Play. Qt retains thin QObject/QML presentation, the shared Omarchy theme and the
credentialed socket edge.

Normal audio Main owns the producer, exact child/pidfd and bounded nonblocking
pipes. The existing backend alone owns links. It validates server-authenticated
PipeWire PID/UID/GID, graph cookie and node/client serials. Applications gain no
private-graph access. Selecting a route without a producer can settle policy
while `applied` remains null: selected and idle.

The private graph supplies no session-manager port policy. The producer sets
`adapter.auto-port-config` at creation to a JSON string with `mode:"dsp"`,
`monitor:false`, `control:false` and `position:"preserve"`; linking remains separate.
This initialization is documented by the [PipeWire property reference](https://docs.pipewire.org/page_man_pipewire-props_7.html)
and implemented in the [exact 1.4.2 adapter](https://github.com/PipeWire/pipewire/blob/1.4.2/spa/plugins/audioconvert/audioadapter.c#L1810-L1892).

State requires exactly seven fields: `connected`, `available`, `pending`,
`requested`, `applied`, `policy_settled` and `preview`; old five-field replies are
refused. Preview states are `idle`, `starting`, `playing`, `retiring`, `completed`
and `failed`. The explicit 15-function C ABI includes Play/Stop, without earlier-ABI
compatibility. Historical thirteen-function executions retain their original scope.

Play is never queued or replayed. Its original absolute `CLOCK_BOOTTIME` deadline
passes through app, actor and audio owner; preparation only reduces remaining
TTL. Starting must settle before that limit. The producer has a separate
six-second work limit. Backend loss, changed authority/route, app connection loss
and revocation retire ownership; recovery cannot replay old Play.

Stop fences acceptance, closes PCM input and signals the retained child without
blocking the request loop. Cleanup may wait behind one read-only status under its
original deadline; during an in-flight effect, app-channel closure withdraws
ownership. Actual wait, stderr EOF and closed handles establish local reclamation.
A later successful absent-source graph observation releases the retiring slot.
Neither alone establishes acoustic silence. Whole-service Stop retains its
original bounded drain and aborts instead of detaching unfinished owners.
Independent virtual PCM now separately qualifies explicit preview Stop and quiet.

## Evidence and remaining gates

| Boundary | Qualified result |
| --- | --- |
| Offscreen Qt adapters/QML | Missing-controls RED→same frozen-test GREEN: construction and two unavailable-state buttons under UID 11042, root-owned 0644 shared QML, unshown window. No input action executes. |
| Sounds C ABI/Qt sockets | Required-seven-field RED→six existing, seven schema and seven preview groups plus two cleanup cases pass; 30 existing Qt behaviors plus setup/cleanup pass. |
| Real actor/audio owner, original PipeWire 1.4.2, authored lifecycle facts/virtual sinks | Initial denial and later zero-PCM failures precede an `unsupported_channel_layout` diagnostic. Sole producer-port initialization change makes the same frozen fixture pass: 24,000 frames per sink, 440 Hz speaker content and exactly zero earpiece samples. |
| Same-session explicit Stop and new Play | The live first producer retires before its natural end; source/client/links disappear and its retained pidfd is terminal. Both full sink windows then measure zero. Only a new explicit Play starts a distinct producer and restores selected-speaker tone. |
| Natural completion | Exactly one Play and no preview Stop: original producer retirement, graph absence and measured quiet while Main stays live; same app FD reports `completed`, explicitly distinct from `failed`. |
| Permission revocation | Revoke retires the live producer before natural completion. Regrant alone stays disconnected; one explicit Reconnect yields available idle/unselected state and full quiet. Only a new Play creates a new producer/tone. |
| Native app close/relaunch | Old app and producer retire before natural completion, followed by full quiet. A distinct same-profile app retains permission but starts unavailable; one explicit Reconnect and another full quiet precede a new Play. |
| Backend-worker loss/recovery | One exact worker SIGKILL retires the producer before natural completion. Full quiet precedes and follows one privileged reconcile; recovered actor state is `failed` with `applied:null`. Only new Play starts a distinct producer on the same app FD. |
| Finite producer owner | Same frozen SIGABRT RED→GREEN after Process-only repair. Delayed polling reaps the already-dead helper; no suspend or PCM inference. |

The same producer is alive before and after capture; its parentage, server
identity, graph cookie, serials and raw format match. The positive assertion, audio Main and
actor exit 0. One source observation at 65.461018 ms is not a latency guarantee.

Host Rust passes 40 audio tests (one existing ignored), nine media-authority and
six media-input tests, formatting and strict Clippy. The current port-config
provider separately repeats all 40 audio tests and checks. Normal Release
builds/loaders pass; Sounds has 356 x86-64 archive members and exactly 15 APIs.
Seven frontend targets each link the selected archive once. Preserved failed
epochs, assertion-preserving bridges and independent raw recomputation remain in
[hash-only provenance](evidence/native-sounds-preview-provenance.json).

The Stop case measures 144,000 frames across three paired stages under one
original six-second cutoff. The natural case measures 96,000 across two stages
under its own new original eight-second cutoff; the six-second per-window maximum
is unchanged. Its discarded leading speaker frames contain a tail and are not
claimed quiet. Its first preparation stays unexecuted; four prelaunch fixture
corrections preserve providers, bounds, active identity and final retirement/
`completed`/PCM requirements.

The permission-revocation case measures 192,000 frames across four paired stages
under one original six-second cutoff: tone, revoked quiet, regrant and explicit
reconnect quiet, then new-Play tone. It uses exactly two Plays, one Reconnect and
zero preview Stops. Regrant alone cannot reconnect or replay; only the new Play
starts a distinct producer.

The app-close case measures 192,000 frames in four paired stages under one
original six-second cutoff. Old app pidfd death, `/proc` absence, producer/graph
retirement and full quiet precede first Play+3s. The fresh app has a distinct
PID/starttime under the same actor. The old app's quit reply records zero as intent;
numeric app ExitStatus and actor generation are unmeasured. Generation progression
is inferred from source.

The backend-loss case measures another 192,000 frames in four paired stages
under one original six-second cutoff. Main remains live; no worker replacement
occurs before one explicit privileged reconcile. Actor state progresses from
unavailable/`retiring` to available/`failed`, with `applied:null` and full quiet.
Exactly two Plays, zero Reconnects and zero preview Stops execute. Recovery cannot
replay the original Play. This does not qualify vendor-specific cleanup.

Raw windows are independently recomputed; actual waits, PIPE EOF, regular-log
closes and adopted-child waits retain separate provenance. These added cases
change no production code and claim no new behavioral RED. Actual Qt Play/Stop
clicks joined to PCM remain open. QML attempts stop in setup, first at control
socket readiness and later at first-route reply validation. A separate bounded
diagnostic observes Button object lookup returning no match before mouse dispatch;
visual delegate presence remains unmeasured. Earlier failed epochs and the backend
setup failure are preserved. No deployment,
ARM execution, normal window/cold graphical session or physical-audio
qualification follows. No physical phone was accessed.

## Reproduce the Linux process tests

Compile the separate fixture explicitly, then run from the repository root:

```sh
preview_fixture_dir="$(mktemp -d)"
rustc --edition=2024 -C opt-level=3 -C debuginfo=0 \
  native-audio/src/preview_process_test_child.rs \
  -o "$preview_fixture_dir/preview-process-test-child"
env POKELINUX_AUDIO_PREVIEW_PROCESS_FIXTURE="$preview_fixture_dir/preview-process-test-child" \
  cargo test --manifest-path native-audio/Cargo.toml \
    --release --locked --offline -- --test-threads=1
```

The frozen tests require this helper; production services never install it.
