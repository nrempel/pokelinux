# Retained Rust parent/worker channel

The shared Rust process owner now exposes bounded copied messages over its
existing authenticated READY/GO SEQPACKET pair. This is a process-custody
prerequisite for the installed radio command path. It is not radio dispatch,
application permission, vendor cleanup or a complete native IMS transport.

## Current contract

`Supervisor::send_message` and `receive_message` require the exact current Ready
`ProcessIdentity`. `BoundParent` supplies the reciprocal methods on the worker
main thread. Both guards remain non-Send/non-Sync; no descriptor escapes, second
socket, reader thread, internal queue or retry is added.

A call performs at most one nonblocking I/O. Send accepts1..8192 bytes and copies
them before returning; receive returns an independent owned Vec. Actual send
backpressure returns WouldBlock; an empty receive returns None. Unexpected
credentials, any transferred rights, empty/closed and truncated records are
refused. Every received right is closed on refusal, including ancillary overflow.
The8KiB bound leaves room for a4096-byte future radio response plus its provenance.

Fresh CLOCK_BOOTTIME samples and live exact retained pidfds bracket I/O, including
backpressure/error results. Each caller supplies its original absolute operation
or session deadline. The three-second bootstrap cutoff remains startup-only.
Stop revokes admission before signaling; occupancy remains until actual wait.
The durable admission marker never clears. Per-call expiry does not silently
kill the generic role; the caller owns Stop and disposal.

A post-I/O error may follow delivery or consume received bytes. It never proves
nondelivery or permits an automatic resend. The process owner decodes no role,
operation, application grant or vendor payload.

## Actual host boundary

A genuine original control first launches through Supervisor/BoundParent,
reaches READY/GO, verifies fd0 replacement, stops, observes actual stdout EOF and
signal9 wait, and proves unchanged-journal successor refusal. The frozen target
then fails to compile on the missing production constant/methods. An initial
mixed fixture compiler failure is preserved separately; only the corrected
missing-interface failure is RED.

The implemented library passes five real Linux groups:

- Copied1-byte/8192-byte round trips over the retained pair after the bootstrap
  cutoff, with source overwrite/drop and independent receive storage.
- Parent refusal of a distinct actual sender PID, one right,64 rights and an
  oversized record; real FIFO EOF and a live fd-table check qualify disposal.
- Reciprocal BoundParent refusal of the same four actual injections.
- Original deadline expiry in both directions with queued records, without
  relying on a timer callback or another bootstrap poll.
- Actual kernel EAGAIN, wrong identity, Stop before wait, queued-output refusal,
  occupied capacity, actual wait and durable successor refusal.

The target observes13 actual child waits and11 stdout PIPE EOFs. Two forged
senders have stdout redirected to null and supply no PIPE EOF claim. The original
control is separate. Nine shared tests,68 radio tests with three ignored, strict
production Clippy, formatting and shared/radio Release builds pass separately.
Fixture-only compiler/style prerequisite failures remain recorded rather than
being labeled functional failures or passing checks.

The [frozen executable consumer](../tools/native-parent-channel-contract/Cargo.toml)
uses actual Linux processes, pidfds, credentials, rights and queues. Its original
work/cleanup bounds are fixed before each spawn; use a bounded owning external
runner, as in the recorded90-second container selection. No unfinished child is
joined or detached on success. See [curated provenance](evidence/native-process-channel-provenance.json).
This host proof is not ARM user-mode process emulation, an ARM Binder execution,
suspend/wake delivery or physical-phone evidence.

## Installed radio remains a separate gate

A separate generic ARM64 kernel run of the unchanged original radio executables
reaches real installed Main/worker/service-account status and an independently
authored Binder peer. Service-account command03 is unsupported. Root is refused
at the original peer UID/GID check before parsing; a client reset label does not
prove root command parsing. Independent captures remain zero105. Native worker
wait9, PID1 Main0, unchanged journal, restart78 without another child/lookup/
registration/request, and complete guest/container/input closure qualify.
All25 direct guest waits and13 actual PIPE EOFs are separate from Main's nested worker wait.
This is an expected missing-capability RED, not an operator-command GREEN.
See [its separate provenance](evidence/native-radio-operator-baseline-provenance.json).

Next implement command-specific root admission, immutable pre-start configuration,
one prewarmed owned durable-intent writer and radio worker messages. Main must
remain responsive while writing. The writer must send its matching sync ACK and
remain alive until Main's one-use EXIT instruction; otherwise live-peer checks
could reject an ACK left by an already-dead sender. Only the retained matching
ACK plus actual normal writer wait0 and fresh Stop/incarnation/attempt/original
deadline checks can release the radio permit. EXIT delivery errors grant no
retry. Stop fences both roles and retains both through actual wait; manager
fallback is neither native cleanup nor vendor cleanup. Request history needs
its own revision and immutable first response, including the Stop snapshot.
The operation slot and uncertainty markers remain permanent.

Carrier IMS, provider/APN authority, deployed app permission, physical calls,
Pixel transport/driver behavior and all remaining phone gates stay open.
