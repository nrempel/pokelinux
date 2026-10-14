# Native scope conversation I/O

This report retains the19-component selection published at `4ca9a97`. The later
[parent forwarding selection](native-auth-parent-forwarding.md) passes40 host
components and implements both fixture forwarding loops in source. Their full
process composition and original PAM remain unexecuted in that selection.

Nineteen ordinary host components now pass after three actual runtime failures.
The exact private conversation implementation uses real Linux pipes; the full
Attempt, cgroup workload, PAM and ARM guest do not execute. The earlier
[13-component wire/parser selection](native-auth-conversation.md) remains separate.
The [bounded provenance](evidence/native-auth-conversation-io-provenance.json)
binds the unchanged original Rust wire, frozen source, logs and retained ELFs.

The extraction first preserves the existing behavior. Its RED passes16 tests and
fails3, with one helper entry ignored in the parent suite. Failed initialization
leaves a retained stdout blocking; its exact ordinary test subprocess times out.
The parent sends exact-pidfd SIGKILL, actually waits9 and observes stdout EOF,
pidfd death and proc absence before failing. A queued public dummy response also
writes22 bytes when the next direct call begins after expiry. A late terminal
frame enters pure protocol candidate state. The pre-existing outer Attempt poll
deadline gate is not bypassed by a full Attempt test here.

Conversation now records successful nonblocking setup of both endpoints before
either can be used. A failed setup refuses later I/O without inventing EOF or
changing the retained blocking descriptor. It owns the original immutable
BOOTTIME deadline, bounded protocol buffers and the complete zeroizing response
frame. Admission and I/O check expiry; a second sample follows writes and data
reads before parsing. Retirement clears response, prompt and candidate state
before the owner performs cgroup work. Attempt still owns the actual Child,
pidfd, pipes, anchored leaf, direct/adopted waits and closure requirements.

The fresh GREEN passes all19 components plus formatting and strict default and
all-target/all-feature Clippy. The setup subprocess exits0 naturally and is
actually waited, with EOF, dead pidfd and proc absence. Calls begun after expiry
write0 bytes and buffer no candidate. The existing Rust wire completes two
hidden-prompt/response rounds and a denied terminal ACK/EOF through joined
private-pipe execution. A full nonblocking pipe establishes actual EAGAIN before
cancellation; draining it afterward cannot release the cancelled response.
Wrong sequence and duplicate queued response calls retire locally without output.

Both deadline controls begin after expiry. They do not force suspend or a
deadline crossing within a syscall. A post-write check cannot retract bytes
already written; an AGAIN/INTR read returns without a second clock sample and
adds no bytes or candidate. Actual partial response writes, EINTR, fcntl errors,
missing-FD writes and write-error injection remain untested. Zeroizing Drop is
an ownership property, not an allocator-scrubbing measurement.

All14 frozen input bindings match, the test/container waits complete and a fresh
check on the same Docker daemon confirms exact container absence. The first RED
preparation stops at duplicate-module Clippy; the next RED and first GREEN stop
at formatting. Each remains failed with zero runtime tests. The qualified RED
and fresh GREEN have separate immutable selections. An AArch64 release fixture
also compiles and has a qualified AArch64 ELF header; it is not executed.

The six new controls call Conversation with real pipes and pure Barrier state.
They do not construct Attempt or establish actual workload closure. The crate
remains isolated-feature-only; the parent/supervisor prompt-response channel was
unwired in this historical selection. Actual Attempt composition, original PAM with preserved effective
faillock, broker/actor one-shot receipts, exact-object unlock, synced durable clear
and a normal Messages reopen remain open. Earlier ARM scope and helper-profile
results retain their earlier source selections. Matched Google policy, secure
frames and physical-device behavior remain gates. No phone was accessed.
