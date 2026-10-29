# Finite native Sounds preview

The qualified next-preview level/mute cohort passes host native checks, frontend C ABI/Qt
components and real unshown QML input through the actor/audio owner to measured
virtual PCM. Six real clicks select a route, 50%, Play, Stop, mute and Play again:
half-level tone, full quiet after retirement, then a distinct live muted producer.
That cohort’s separate cold installed generic ARM tracer also passes. Earlier
full-level cohorts retain their own sources; physical hardware remains unqualified.

## Ownership and model

Rust generates fixed four-second 440 Hz mono F32LE PCM at 48 kHz. Each Play
freezes its percentage; live mute changes only future whole frames of that
retained producer. The original percentage remains available for unmute. Applications supply no file, PCM,
executable, node or frequency. The lifecycle actor checks
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

Current State requires exactly nine fields: `connected`, `available`, `pending`,
`requested`, `applied`, `policy_settled`, `preview`, `preview_generation` and
`preview_muted`. The last two are a positive integer and boolean during Starting
or Playing, otherwise both null. Preview states are `idle`, `starting`, `playing`,
`retiring`, `completed` and `failed`. The current 17-function C ABI adds
`pr_sounds_client_preview_toggle_mute_v1`; its 5648-byte View distinguishes the
next-Play choice from confirmed live mute and the pending action. Earlier
16-function/5616-byte and 15-function executions retain their original scope;
old ABI/wire shapes have no compatibility fallback.

The [live-mute implementation and qualification](native-live-preview-mute.md)
are separate from the earlier next-preview level cohort below.

Play is never queued or replayed. Its original absolute `CLOCK_BOOTTIME` deadline
passes through app, actor and audio owner; preparation only reduces remaining
TTL. The [qualified startup correction](native-audio-startup-deadline.md) separates two immutable bounds:
`D = min(A, captured Source expiry)` for transport, admission and positive reply
publication, and `A = min(original app deadline, original TTL expiry)` for admitted
startup. The trusted owner-to-Main request carries both `until_boottime_ns` and
`startup_until_boottime_ns`; Main requires the seven-field shape and
`now < D <= A <= now + 1 second`, and rechecks D after producer creation.
Successful Starting publication retains the existing approved background lease.
The producer keeps its separate six-second work limit. Backend loss, changed
authority/route, app connection loss and revocation retire ownership; recovery
cannot replay old Play. The controlled held-Source RED→GREEN and existing Source-loss/app-expiry controls
pass. Formatting, strict Clippy and selected host component checks also pass
(56 audio, 117 lifecycle and 34 owner cases). These controlled host results do not
qualify PCM, genuine graphical playback, ARM execution or phone hardware.

Stop fences acceptance, closes PCM input and signals the retained child without
blocking the request loop. Cleanup may wait behind one read-only status under its
original deadline; during an in-flight effect, app-channel closure withdraws
ownership. Actual wait, stderr EOF and closed handles establish local reclamation.
A later successful absent-source graph observation releases the retiring slot.
Neither alone establishes acoustic silence. Whole-service Stop retains its
original bounded drain and aborts instead of detaching unfinished owners.
Independent virtual PCM now separately qualifies explicit preview Stop and quiet.

## Qualified next-preview level evidence

Rust owns a 0–100% choice, initially 100%/unmuted. The atomic setter changes no IPC,
request or deadline and is allowed offline or during read-only status. A pending
connection returns Busy; selected values survive connection changes for that client
lifetime. It refuses pending effects, queued
intent or Starting/Playing/Retiring. Play freezes route, level, mute and original
deadline; reprepare only reduces TTL. Actor and Main require integer
`level_percent` and boolean `muted`, validate 0–100 and retain immutable gain.
This controls the next authored preview, with no global mixer or call/capture gain.
Qt only presents 50%, 100% and Rust-driven Mute/Muted controls.

A healthy granted app's literal valid50 request first returns denied; both 24,000-
frame sink windows are zero and the compiled assertion exits 101 after cleanup.
The same frozen client/controller/observer/oracle then passes: speaker RMS
0.08838669987845607, peak 0.12499618530273438 and 440 Hz energy fraction
0.9991972100720329; all 24,000 earpiece samples are zero. That cohort
passes 46 audio tests (one existing ignored), 11 authority and seven input tests,
fmt and strict Clippy. The 356-member x86-64 frontend archive has exactly 16 APIs;
all seven normal Qt targets pass link/loader checks, six C ABI/seven schema/ten
preview/two cleanup groups, 31 Qt behaviors and unshown level/mute bindings.

That cohort’s [public test](../tools/native-audio-preview/) passes six native mouse
pairs through the same app FD: one route, two Plays and one Stop. Its 144,000
measured frames share one original six-second cutoff. Half-level speaker tone
and quiet earpiece precede Stop; actual producer/graph retirement precedes full
paired quiet. New muted Play stays quiet with a distinct live attributed producer,
active selected link and public Playing state throughout capture. Silence alone
supplies no muted-Play proof. The final assertion exits 0 after cleanup/ECHILD.
All 56 direct waits/75 real PIPE EOFs pass; one adopted wait and four regular
stderr closures remain separate, without individual log-close timestamps.
Preserved style-check and build-wrapper/file-limit failures and exact hashes are
in [level provenance](evidence/native-preview-level-provenance.json).

That cohort’s Rust 1.98.1 AArch64 crossbuild also passes: normal audio/launcher and
three fixture ELFs, plus a 452-member/16-API Sounds archive. All 16 host waits/merged
PIPE EOFs and 18 separate shell waits with regular-file output close.

Later generic ARM preparation passes all five Rust loaders and native Qt6.12
consumer build/link/load with 16 Sounds and 11 appearance APIs, each archive linked
once. The existing 393 package versions and 50 selected Qt libraries/plugins stay
unchanged. Four normal PID1 Stops precede the stopped update; all 52 direct waits/
merged PIPE EOFs close, with PID1 outcomes counted separately. Export follows
actual QEMU exit 0/EOF and closed handles.

One fresh cold installed generic ARM run now passes four new PID1 invocations,
six real QML mouse pairs and 144,000 measured frames: half-level tone, actual Stop/
retirement/full quiet, then a distinct live muted producer with an active selected
link under the same Main/graph identity. Local 50%/mute clicks send no packets;
all three controls are disabled during Starting/Playing/Retiring. Its upfront
ARM aggregate is 12 seconds, each window retains six seconds, and measured work
is 6.832403559 seconds; the host's six-second aggregate is separate. Each Play
retains its original one-second deadline and Stop retires before Play+3s.

All four units Stop normally. Two independent terminal records share the same
original cutoff and each needs one observation; repeat polling remains unexercised.
The final compiled assertion exits 0 after controller/prior parent Scope closure;
QEMU exit 0/EOF, closed handles and outer absence pass. All 67 direct waits/79 real
PIPE EOFs close, with four regular stderr files and two DEVNULL streams separate;
individual log-close timestamps are unrecorded. Root recomputes all six raw files;
an independent review passes. ARM tone's raw hash differs from the host, while
measured half-level values match. Normal app main/window, Pixel integration,
display/capture, live-theme, physical audio and phone gates remain open.

## Earlier full-level evidence

| Boundary | Qualified result |
| --- | --- |
| Earlier offscreen Qt adapters/QML | Missing-controls RED→same frozen-test GREEN: construction and two unavailable-state buttons under UID 11042, root-owned 0644 shared QML, unshown window. No input action executes. |
| Sounds C ABI/Qt sockets | Required-seven-field RED→six existing, seven schema and seven preview groups plus two cleanup cases pass; 30 existing Qt behaviors plus setup/cleanup pass. |
| Real actor/audio owner, original PipeWire 1.4.2, authored lifecycle facts/virtual sinks | Initial denial and later zero-PCM failures precede an `unsupported_channel_layout` diagnostic. Sole producer-port initialization change makes the same frozen fixture pass: 24,000 frames per sink, 440 Hz speaker content and exactly zero earpiece samples. |
| Same-session explicit Stop and new Play | The live first producer retires before its natural end; source/client/links disappear and its retained pidfd is terminal. Both full sink windows then measure zero. Only a new explicit Play starts a distinct producer and restores selected-speaker tone. |
| Natural completion | Exactly one Play and no preview Stop: original producer retirement, graph absence and measured quiet while Main stays live; same app FD reports `completed`, explicitly distinct from `failed`. |
| Permission revocation | Revoke retires the live producer before natural completion. Regrant alone stays disconnected; one explicit Reconnect yields available idle/unselected state and full quiet. Only a new Play creates a new producer/tone. |
| Native app close/relaunch | Old app and producer retire before natural completion, followed by full quiet. A distinct same-profile app retains permission but starts unavailable; one explicit Reconnect and another full quiet precede a new Play. |
| Backend-worker loss/recovery | One exact worker SIGKILL retires the producer before natural completion. Full quiet precedes and follows one privileged reconcile; recovered actor state is `failed` with `applied:null`. Only new Play starts a distinct producer on the same app FD. |
| Real unshown QML input→actor/audio→PCM | Route, Play, Stop and new-Play mouse pairs each emit exactly one real click. Public model/QML text passes Playing→Retiring→Idle→Playing on the same app FD. Three paired windows measure 144,000 frames: selected-speaker tone, full quiet, then new-Play tone. |
| Finite producer owner | Same frozen SIGABRT RED→GREEN after Process-only repair. Delayed polling reaps the already-dead helper; no suspend or PCM inference. |

The same producer is alive before and after capture; its parentage, server
identity, graph cookie, serials and raw format match. The positive assertion, audio Main and
actor exit 0. One source observation at 65.461018 ms is not a latency guarantee.

That host Rust cohort passes 40 audio tests (one existing ignored), nine media-
authority and six media-input tests, formatting and strict Clippy. Its port-config
provider separately repeats all 40 audio tests and checks. Normal Release
builds/loaders pass; that Sounds build has 356 x86-64 archive members and exactly 15 APIs.
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

The QML case uses current AudioClient, shared appearance and real SoundsView
Buttons, with all top-level Quick windows unshown/unexposed. Native mouse press/release
pairs drive the scene centers; no direct effect calls, model writes or synthetic
`clicked` emissions supply the result. Actual send bytes retain Rust deadlines.
The three paired windows share one original six-second cutoff. Fixture-only C++
visual discovery and layout polishing precede success; earlier object-lookup and
zero-click setup failures remain separate, without a production RED/repair claim.

Raw windows are independently recomputed; actual waits, PIPE EOF, regular-log
closes and adopted-child waits retain separate provenance. Those added cases
change no production code. Their separate Rust 1.98.1 crossbuild produces audio/
launcher plus three fixture binaries as AArch64 ELF and a 452-member/15-API Sounds
archive.

Later generic Arch ARM/QEMU preparation passes all five current Rust loaders and
a native Qt6.12 consumer build/link/loader check with 15 Sounds and 11 appearance
APIs, each archive linked once. Available IPC from the old active PID1 daemon
lacks required `policy_settled`; the assertion exits 101, without legacy fallback.
All four old units Stop successfully with empty cgroups and terminal retained
pidfds. Eight cached package additions retain the original 385 versions and 50
selected Qt libraries/plugins, yielding 393 packages. The current daemon and three VM site configs are
explicitly installed while stopped. All 62 direct waits/merged PIPE EOFs close;
four PID1 outcomes remain separate. QEMU exit 0/EOF and closed handles precede
export.

The first cold generic ARM run passes schema/QML/PCM but fails whole-service
terminal verification after one Stop command and transient sequential samples.
Its final compiled assertion never runs; later console poweroff does not qualify
that Stop. A second cold boot now passes four fresh automatic PID1 invocations,
current schema, four real QML clicks and three paired PipeWire 1.6.9 PCM windows:
144,000 measured frames of tone/quiet/tone under the original ARM 12-second
aggregate and six-second window limits. The same app FD carries 41 actual sends;
each Play completes once under its original one-second deadline.

One target Stop has two independent terminal records under the same original
30-second cutoff: all four units have MainPID 0, PID1 code/status 1/0, empty
cgroups, terminal retained pidfds and absent proc entries. Both records contain one
observation, so the new repeat-poll branch is unexercised and no causal repair
is claimed. The final compiled QML assertion exits 0 only after cleanup. All 68
actual waits, 81 real PIPE EOFs, QEMU exit 0 and outer closure pass separately.
Regular stderr files and DEVNULL streams retain separate accounting. No feature C++/Rust
change supplies this result.

The earlier [repository test entry and build commands](../tools/native-audio-preview/)
pass build→run on the host: locked/offline Rust and explicit-archive Qt
builds supply the three new test ELFs executed by actual public `run.py`. Four
real clicks and 144,000 measured PCM frames pass under one original six-second
cutoff; the native assertion exits 0 only after cleanup. Actual 58 direct waits/
79 real PIPE EOFs and one separate adopted wait close; four regular stderr files
have no individual close timestamps. The earlier prebuilt entry keeps its own
provider mapping; normal audio/launcher retain their separate original builds.
These C++ additions are test fixtures; production policy remains Rust. Pixel-
kernel, normal app main/window, display-profile/capture, live theme switching and
physical-audio gates remain open. No physical phone was accessed.

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
