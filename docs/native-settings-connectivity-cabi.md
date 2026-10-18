# Settings connectivity C ABI

The connectivity engine now has a pointer/scalar boundary in the normal Settings
Rust crate. Rust owns state, validation, permissions hints, sockets, deadlines and
worker lifetime. The header translates these to an opaque handle and a 40-byte
view; configured is -1 for unknown, 0 for disabled and 1 for enabled. This value
makes no Internet or Wi-Fi claim. The existing brightness C ABI is separate.

Seven exports cover new, view, explicit desired-state Set, deadline capture, Stop,
Join and Free. Return/status values use the existing client header constants.
The deadline helper samples BOOTTIME and adds the engine's one-second action
budget with checked arithmetic; zero means failure. Capture once at the
originating gesture and carry that original absolute deadline with the boolean.
Set validates canonical zero/one input and delegates directly to the proven
engine without renewing the cutoff. OK is local admission, not acknowledgement.

New copies path bytes. View samples current BOOTTIME and copies complete scalars;
non-OK output remains unchanged. Callers provide live aligned pointers and
exclusive disjoint output storage, with no overlapping free. Stop withdraws local
publication; a previously committed send may still reach the actor. It never
means Disconnect or remote cleanup. Join retains the original 200ms budget and
must actually join before Free can succeed. The caller retains ownership on WAIT
or refusal; unresolved join must abort rather than detach.

## Evidence

The actual C ABI consumer first compiled a refusing setter with real new/view/
Stop/Join/Free. Three readonly groups passed. Three mutation groups obtained fresh
control state and then failed the exact desired Set-acceptance assertion.
Thirteen nested Child waits and 26 actual PIPE EOFs closed, with 12 SDK workers
joined and one task after each join; the separate writer has its own wait/EOF.
The parent observes early child exit, calls the existing finish/drain under the
original five-second cutoff, and only then reports assertion failure. Missing
compilation or custody observations do not count as RED.

Only after Root independently qualified that RED did the setter delegate the
explicit boolean and untouched original deadline to the Rust engine. The same
frozen consumer, adapter and custody helper are registered as a normal crate test
in a subdirectory that prevents accidental Cargo test discovery of helper files.
The unchanged six groups now pass. The actual 17 nested Child waits and 34 PIPE
EOFs close; 16 SDK workers actually join under their original 200ms bounds and
leave one task. Formatting, strict all-target Clippy and the normal crate's own
Release archive build pass. An earlier green preparation failed formatting
before compilation or behavior; its receipt remains a setup failure. Only
Rustfmt whitespace changed before the positive run.

The separate C++ consumer includes the real header and links that exact normal
Release archive. Both compiler and consumer run as UID11050 with all capability
sets empty and NoNewPrivs. It checks the 40-byte layout, all seven function types,
BOOTTIME deadline brackets, invalid booleans, unchanged outputs, unavailable
refusal and real Stop/Join/Free. Early Free refuses; actual join leaves one task
and completes through Free in about 1.08ms under the original 200ms cutoff.
This foreign-language smoke does not exercise positive network packets. The
archive has all 13 Settings API symbols; no second Rust archive is linked.

See [hash-only provenance](evidence/native-settings-connectivity-cabi-provenance.json)
for exact sources, normal archive, separate receipts and custody counts.

This cohort uses actual exported Rust C ABI functions and real UID11050 sockets
with authored root peers. It does not execute the lifecycle actor, real network
backend or Qt window. The earlier [safe SDK actor/packet result](native-settings-connectivity-sdk.md)
retains its own evidence scope. Successful helper status is an admission hint,
not a real OS permission grant or carrier observation.

## Next boundary

Add thin Qt controls, then reuse the existing packet
oracle for normal Settings ownership and installed actor/Main integration.
Programmatic refresh must never emit a Set. Render fresh current facts while
preserving unconfirmed action outcomes. Capture deadlines before queued gesture
delivery. Window close must disable callbacks, Stop once and poll Join
asynchronously before Free; it must neither busy-wait on the GUI thread nor
trigger Disconnect. No new normal window, screenshot, deployment, ARM, actual
suspend or phone qualification follows from this component work.
