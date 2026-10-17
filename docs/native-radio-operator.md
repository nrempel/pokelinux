# Installed one-shot radio operator

The installed radio Main, finite intent writer, transport worker and status client
are Rust. They join the recovered typed add-PDN encoder to one explicitly
privileged operation. This backend still depends on the Samsung radioExternal
provider; native modem-facing replacement, APN ownership and semantic IMS remain
separate work.

The original installed ARM baseline demonstrated healthy transport and RAW137,
then missing root command admission and zero105. Its service-account03 was
unsupported; root was refused before parsing. Normal Stop, native worker wait,
PID1 Main0 and unchanged-journal restart78 qualified separately. See the
[baseline provenance](evidence/native-radio-operator-baseline-provenance.json).

## Ownership and admission

Startup copies the strict root-provisioned typed configuration before accepting
commands and prewarms two separately owned child roles. Registration, writer
launch and operation markers are permanent uncertainty fences. An existing marker
refuses startup before config input, spawning or Binder lookup; Stop and restart
never erase them or register again.

Only a live connected root0:0 origin may send command03. The command fixes schema1,
current incarnation, attempt1, serial1, request105, slot1, cached payload SHA256 and
an original future BOOTTIME deadline bounded by the session. It accepts no payload,
config path or arbitrary request code. Service-account Status/Stop cannot admit a
request; root does not acquire Status/Stop authority from its operator role.
Exact connected and per-packet credentials, retained socket-origin pidfd, descriptor
rejection and final liveness checks remain required.

Main reserves its one operation slot before sending anything. A dedicated writer
creates the exact12-line metadata record with O_EXCL/NOFOLLOW/0600, writes it,
syncs file and directory, then acknowledges over its authenticated retained role
channel. Main validates that live acknowledgement and sends the sole matching
Finish. Only normal writer exit0 observed through its actual native wait, followed
by fresh original operation/session/route/worker gates, permits one dispatch.
Acknowledgement, marker existence or child exit alone is insufficient.

Configuration and durable regular-file work is confined to startup and the finite writer. Main's live
loop uses bounded nonblocking messages, snapshots and child observations. Stop
fences both roles before replying and shares one original two-second disposal
budget; an unreaped child forces manager fallback rather than detachment. Process
reaping grants no vendor cleanup or safe retry.

## Current status protocol

RADS version2 carries one complete typed Status or StopAck snapshot, within16384
bytes. The prior production layout is removed. `last_observation` remains ordinary
raw/registration-wire diagnostic history; malformed declared lengths may remain
visible there. Independent `request_history` has its own revision and exact intent,
incarnation, route, digest, deadline, conservative outcome and optional immutable
first response. A wrong serial/slot/code/length cannot become that response.
A later duplicate cannot replace it. Accepted response bytes/error are history,
without modem-success or APN/IMS meaning. No response crash persistence is claimed.

An expired permit positively observed before any native invocation can report
NotSent under the still-live session deadline. Stop before consumption retains
OutcomeUnknown. Neither outcome releases capacity or permits an implicit retry.
StopAck carries retained history; the client separately observes exact Main exit,
and PID1 separately owns Main's actual wait.

## Execution evidence

Separate frozen host consumers qualify the operation codec, actual synced writer
Finish/waits, current frontend and real Linux credentials. The complete selected
build passes83 radio tests (nine explicitly Binder-required tests ignored), nine
shared process tests, formatting, strict Clippy and six actual ARM64 Release
binaries. Five existing termination groups separately qualify the shared signal
guard, including an actual child signal15 wait and both stream EOFs.

All six fresh installed generic ARM64 kernel/Binder cases now pass. The original
five-case consumer is unchanged; manager Stop is a separate sixth consumer.

| Case | Required observation | Guest waits / PIPE EOFs |
| --- | --- | --- |
| Accepted | Durable sync, matching Finish, actual writer wait0, exactly one663-byte105; four wrong callbacks unattributed; first response survives duplicate and Stop |47 /31 |
| Writer held | Three responsive status visits, stable transport revision, no sync/permit/105; Stop retains unknown outcome |33 /17 |
| Expired before admission | Root refusal, no operation history or record, zero105 |31 /15 |
| Permit expires | Held worker resumes after the original operation deadline, reports NotSent without105; late response remains unattributed |37 /21 |
| Stop before consumption | Held worker never consumes the permit; zero105 and unknown outcome retained |32 /16 |
| Manager Stop with writer held | Actual systemctl Stop, no socket StopAck; both native signal9 waits, Main0 and unit success |34 /18 |

Across these epochs,214 actual direct guest waits and118 actual PIPE EOFs precede
their original cutoffs. Regular output files are not counted as PIPE EOFs. Twelve
separate native transport/writer waits, six PID1 Main0/success outcomes and six
PID1 restart78 refusals qualify independently. Every refusal retains exact marker
inode/bytes and unchanged peer counts, with no new lookup, registration or105.
All six foreground container CLIs exit0; their exact owned CIDs are absent and
selected sources, ELFs and locked boot inputs are unchanged.

The five socket Stop caller operations take574–645ms through the helper wait and
850–905ms through both native waits and PID1 observation. The separate manager
Stop takes484ms through its helper wait and396ms through native/PID1 observations.
These are individual generic ARM VM measurements, timed before caller Popen/RPC;
no exact kernel packet latency is retained. Original five-second Stop and
once-only two-second native cleanup budgets are unchanged.

Earlier failures remain failed. The first accepted run reached105 and actual
writer wait, then failed an overstrict diagnostic-length check on the deliberate
malformed callback. Another setup failed when a proc descriptor disappeared
between diagnostic reads. Two later runs reached native waits/Main0 but failed
unit health because the old ExecStop attempted a second RPC against the exited
Main. The repair retains the shared single-thread Rust SIGTERM guard from before
spawning through actual cleanup, checks it at final admission/permit gates,
removes ExecStop and uses KillMode=mixed. Later omitted CLI-property setup failures
led to exact typed systemd GetUnit/Service.ExecStop queries; they supplied no
functional GREEN. A first audit also failed on its own incorrect assumption that
the existing custom termination harness emitted Cargo test-summary lines; its
exact five-group output is now checked without inventing such a summary.

The actual Release build used harness3. The separately pinned runtime harness5
changes only the guest show() observer to typed systemd queries; all source bytes
outside that function, eight compiled fixture inputs and six ELFs match. This
explicit admission bridge is not a new build or an inherited runtime result.
The [bounded provenance](evidence/native-radio-operator-provenance.json) binds
actual Cargo streams, source hashes, all six results and the independent closed
audit. Post-build packaging README edits are documentation only; the other80
selected inputs remain byte-identical.

## Remaining gates

This is a privileged backend tracer, with authored peers on a generic ARM kernel.
App permissions and phone APIs, production provider identity and APN acquisition,
subsequent operations, vendor cleanup/reset, native IMS/calls, matched Pixel
policy, suspend/wake and physical hardware remain unqualified. Registration stays
unavailable. No physical phone has been accessed.
