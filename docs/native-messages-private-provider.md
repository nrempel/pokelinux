# Private Messages history owner

The normal Messages daemon now owns the sole history Projector, including its
writer and separate readonly child. The graphical session selects
`Messages::Owner { socket }` and connects through `read_service::Remote`; it no
longer opens raw/Inbox paths or starts another Projector. App permission and
Source checks remain in the session actor at dispatch and publication. The
normal daemon takes only runtime, raw and Inbox directories; typed admin Stop
still uses `messages-stop` and actual daemon/child closure.

The root-private read connection authenticates peer credentials and pidfd
liveness, carries an exact generation/token/original BOOTTIME cutoff and bounds
one read to at most one second. Received file descriptors are rejected and
closed. Closing a local connection preserves the background owner; local timeout
closure explicitly retains uncertainty about server-side reader retirement.
Cancellation acknowledgement follows actual reader reap, preserving the writer;
recovery requires an explicit request and a fresh readonly generation.

The host evidence uses real normal daemon/storage processes with authored local
records, not a substitute storage peer:

| Epoch | Actual result |
| --- | --- |
| Normal-daemon RED | Exact independent history and normal typed Stop/waits pass; the required private socket is absent. The frozen consumer exits 101 after cleanup. |
| First GREEN build | Compile failure; no behavioral result. |
| Normal-daemon GREEN | The same 10,386-byte Rust consumer reads both exact distinct receipt IDs/text, closes/reconnects with the same daemon generation, then completes typed Stop/waits. Test exit 0. |
| Restart admission RED | A second queued Restart is incorrectly accepted; the test exits 101. The aggregate runner also fails because it expected different panic text. |
| Focused GREEN | One bounded Restart case, one SCM_RIGHTS refusal/closure case and four actual Remote cases pass. These cover exact history/local Stop, held-reader cancellation/recovery, original-deadline fencing and daemon replacement. |

The frozen consumer source hash is
`77bb49719dc642a7aad69feacb993972f0a47db9a755494e9c9c2db186206c75`.
The later current consumer adds a post-decode deadline check plus lint/format
changes and passes separately; its different hash is recorded in provenance.
The old `test_sha256` fields name different objects: CONTRACT hashes Rust source,
while results hash the compiled test ELF. The GREEN runner also inherits incorrect
`functional_RED_qualified: true` and `production_unchanged: true` labels. The
actual exit-0 log and separate qualification record establish the GREEN scope;
those original records and both failed epochs remain preserved in the
[curated provenance](evidence/native-messages-private-provider-provenance.json).

Four broad-QA aggregates retain their failures: formatter invocation, a test
Clippy lint, Config-fixture cleanup and a missing setup-helper environment.
Separate closed scopes pass formatting/strict Clippy; Messages unit 2, projection
9, raw consumer 1, Remote 5 and direct-worker 3; and Config 15. The later corrected
completion passes Messages configuration 10, brightness 4 and connectivity 4,
then builds 12 normal Release ELFs: six x86_64 and six AArch64, optimization 3
and no optional features. These scopes remain separate from the failed aggregates.
The missing-provider setup helper exits 0 within its original four-second bound,
retains original `NotFound`, observes Source ECHILD and creates no session
databases; it exports no numeric Source child exit status. This is failure cleanup,
not successful normal Main/Source/UI startup.

This checkpoint qualifies no session-owner Main execution, new ARM/PID1 cold
boot, normal Source, graphical UI, incoming SMS or physical phone. The
[earlier ARM cold boots](native-installed-messages-history.md) remain historical
six-argument-service evidence. Historical service/recovery fixtures,
`native-messages-guest.py`, display-app-generation and installed-app guest
harnesses require their recorded sources and ELFs. `run-native-messages-vm` is
not a current preview launcher for the new three-argument daemon.

The next integration is one installed background owner plus the session app actor
in the reproducible VM preview, followed by a separate new installed ARM/PID1
cold-boot qualification. Current normal Source and UI behavior must be measured
with that selection before it becomes a working preview claim.
