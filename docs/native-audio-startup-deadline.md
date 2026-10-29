# Native audio startup deadline

A controlled delayed-backend test now passes with the Rust deadline correction.
The same test failed before the change. This is a host integration result with
unshown Qt 6.8.2 input, an authored lifecycle Source, the actual Rust actor and
normal audio Main, and a private PipeWire server. It does not qualify audible
PCM, the genuine Omarchy graphical session, ARM execution or phone hardware.

## Behavior

A Play request must be accepted and its positive Starting reply published before
`D = min(A, captured Source expiry)`. Once admitted, startup may use the remaining
original application budget `A = min(original app deadline, original TTL expiry)`.
Refreshing Source does not renew A. Main validates `now < D <= A <= now + 1 second`
and checks D again after creating the producer. The internal owner-to-Main Play
request requires both deadlines in its seven-field shape; there is no old-wire fallback.
The app protocol, existing background promotion policy and separate six-second
producer work limit are unchanged. Cancellation and loss still retire ownership.

Previously Main also used D as the asynchronous startup cutoff. In the qualified
RED, it retired the admitted producer just after D even though Source was still
fresh and the original application budget remained valid. An unpaused control
passed before that delayed case was run.

The GREEN uses the same 18 fixture sources, original positive assertion executable,
Qt client, launcher and cutoffs. Only matching normal Main and actor binaries change.
The PipeWire server is held before Play and resumed 20.326 ms after D. Starting was
published 50.147 ms before D; the same producer reports Playing 512.531 ms before A.
There is one explicit Play and no implicit retry. The positive native assertion
runs only after cleanup and exits successfully. Actual process waits, required
pipe EOF, producer/app termination and removal of the exact container all pass.

These results establish this controlled failure mechanism. They do not explain
or relabel earlier graphical audio failures, setup failures, observer failures or
the separate failed outer cleanup attempt. Those records remain separate.

## Validation and remaining checks

Formatting and strict Clippy pass. Selected host component checks pass: 56 audio,
117 lifecycle and 34 owner cases. The lifecycle default run leaves 26 isolated
cases ignored; the audio default run leaves one ignored case, which was then run
explicitly against Main. Matched normal release binaries were built offline.
The held-Source integration is GREEN. Existing Source-loss and original-app-expiry
integration controls also pass with the same fixture sources and matched binaries:
loss cancels the preview before recovery, and fresh Source cannot extend the original
app cutoff. Neither control replays Play. Both native assertions and full cleanup pass.

[Evidence and artifact hashes](evidence/native-audio-startup-deadline.json) record
the separate control, RED, GREEN and regression boundaries.

See [Sounds preview](native-sounds-preview.md) for the broader audio contract and
[development handoff](development-handoff.md) for the current deployment work.
