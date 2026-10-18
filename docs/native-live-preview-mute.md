# Live preview mute

The current implementation lets the Sounds app explicitly mute or unmute the
currently playing finite preview. It retains the original producer, selected
route, percentage, four-second asset and work deadline. This is a digital feed
control; already buffered PCM cannot be retracted.

Rust owns both sides of the control. The app freezes the observed Play generation,
inverse confirmed mute and original one-second BOOTTIME deadline. The actor
requires the approved owning session, current foreground MediaRoute permission,
the same backend incarnation and observed route. Native audio Main checks the
peer, lease, generation, Playing state and deadline again. The retained PCM feeder
changes only unsent whole frames; it preserves the remainder of any sample already
partially written. Unmute restores the frozen percentage. No node-ID mutator,
new producer, route change, asset extension or implicit retry is introduced.

The UI distinguishes next-preview choices from confirmed live mute. While a live
request is pending, the checkbox retains the confirmed value. A matching response
must name the same generation and requested boolean. An uncertain or mismatched
response fences the channel. Authority is rechecked before dispatch and reply
publication. Stop remains cleanup; withdrawal is not a remote rollback or evidence
of immediate silence.

## Qualified host behavior

A real compiled unshown QML baseline first reaches Playing with an attributed
producer and 48,000 measured frames of half-level speaker tone and quiet earpiece.
The existing mute button is actually disabled. Its enabled guard refuses input
before any mute mouse press/release: this is a behavioral RED, not a delivered
mute click or PCM mute result.

The initial retained-process tests execute eight cases: the four existing ownership
cases and four new live-control groups. A refusing stub fails the two positive
PCM-tail cases; the repaired implementation passes all eight. The tests use real
pipes and retained children, including a controlled partial-frame seam. That seam
is not evidence of a naturally occurring production short write. Actual wait,
stderr EOF and handle closure are required separately from captured bytes.

The first process GREEN attempt is a compile failure (integer conversion), not a
behavioral result. The corrected process run passes, and final combined checks
qualify the formatted sources. Current checks pass 55 audio tests (one existing
ignored), 12 media authority and 19 media input tests, formatting and strict
Clippy. Nine finite-process cases retain 13 actual child-retirement records.

The 17-API/5648-byte Rust frontend passes six existing C ABI, eight schema,
14 preview groups and two cleanup cases. All seven normal Qt targets build and
resolve their loaders; 32 Qt behaviors plus setup/cleanup and six unshown QML
incidents pass. Required State has nine fields. A stale seven-field test setup
and an overly broad offline readiness predicate remain preserved failed epochs;
neither is a product behavioral RED. The final archive/header remain byte exact
after the feeder repair, so the matching Qt proof is reused without rebuilding it.

## Actual same-producer PCM

Five real unshown QML mouse pairs select a route, choose 50%, Play, Mute and Unmute.
The same app FD emits one route, one Play and two live controls, with no Stop,
reconnect or effect retry. Each live request carries the same positive Play
generation and its own original one-second deadline; confirmed replies arrive
before those cutoffs. Local next-preview values remain unchanged.

The first integration run fails a complete quiet window after an acknowledged
Mute: speaker RMS 0.04518145167796858, while earpiece is zero. Tone remains through
frame 15871 including the fixed 9600-frame discard. The live producer and graph
identity are unchanged. Its compiled assertion exits 101 after qualified cleanup;
the controller's earlier marker exception retains a setup label, while the
separate full-window PCM result is an actual behavioral RED. Unmute never runs.

The sole runtime repair bounds the kernel PCM pipe before spawning: request 4096
bytes and refuse an actual capacity above 16384. Linux may round to a page and
returns the actual allocation, so the owner checks that result; see the
[Linux interface documentation](https://man7.org/linux/man-pages/man2/F_GETPIPE_SZ.2const.html).
The new gated-child test checks actual kernel capacity/backpressure and captured
queued-prefix/future-mute bytes. This does not bound downstream PipeWire buffers.

The exact same fixture, Qt binaries, PCM thresholds, 9600-frame discard and original
clocks then pass. Six full windows measure 144,000 frames at 48 kHz. Speaker tone
before Mute and after Unmute has RMS 0.08838669987845607 and peak 0.12499618530273438;
440 Hz energy fractions exceed 0.99919. Both muted windows and all earpiece windows
are exactly zero. One producer's PID/starttime, node/client serials, Main and graph
cookie remain unchanged and live around every capture. Final observation is
2.322333488 seconds after the original Play dispatch, before its four-second end
and the original six-second aggregate. No asset, deadline or oracle is extended.

The compiled assertion exits 0 only after controller cleanup and parent
wait/PIPE EOF/handle closure/ECHILD. All 51 direct waits and 65 real PIPE EOFs pass.
One adopted wait, four regular stderr closes and two DEVNULL streams stay separate;
individual regular-close timestamps are not recorded. Exact container removal,
same daemon and unchanged selected inputs pass. Root recomputes all six raw files.
[Hash-only provenance](evidence/native-live-preview-mute-provenance.json) retains
separate sources, failed epochs, component and runtime proofs.

No current ARM live-control, matched Pixel kernel, physical audio, suspend,
normal app window or phone result is claimed. The earlier
[next-preview level cohort](native-sounds-preview.md) retains its separate host
and generic ARM evidence. The full smartphone completion matrix remains open.
