# Root-private connectivity Main and client

Current normal-Main/Rust-client packet execution passes on Main source
`fca5b442…`, alongside the actual UNIX accept-error regression, two library tests,
eight owner tests, formatting and strict Clippy.

Earlier packet GREEN on `40c5a4fd…` remains a separate historical cohort.

A protected root-owned 0700 directory and 0600 seqpacket endpoint connect the Rust
client to a dedicated Main, separate from the app-launching actor. Requests are
explicit Inspect or fixed-profile Apply with an original BOOTTIME deadline.
Transport epoch 1 identifies trusted-root delegation; it is not an application
grant. Requester close fences its reply while transferred work and the connection
remain with the background service. There is no effect replay, implicit
Disconnect, rollback or replacement.

The compiled baseline reaches healthy startup, receives fresh same-worker
Refused/configured=false, then performs another Inspect and independent empty
address readback. Its positive assertion fails 101 only after Main 0, nested
worker wait/output EOF, retained-pidfd terminal observation and backend cleanup.
The expected RED qualifier passes while product passed remains false.

The same consumer now observes Completed/configured=true, two nonce roundtrips
across private requester close, two newer idle observations through a fresh
client, and explicit Disconnect with independent address removal. Current
functional work completes in 988802450 ns under the original six-second case.
The quiet phase has an original 800 ms cutoff including delivery; the actual
peer-armed-to-final-EAGAIN span is 799602717 ns, with zero datagrams. The earlier
40c5 cohort's armed span was 799637005 ns; neither is labelled a full 800 ms
armed-only measurement.

UID 11050 receives actual EACCES 13, with effective/permitted sets zero and NNP 1,
while its bounding set retains the initial five capabilities. Main, driver,
controller and packet peer have all capability sets zero; only the worker keeps
NET_ADMIN. This is endpoint access denial, not actor permission evidence.
Current packet execution has 21 Scope waits/PIPE EOFs and two regular compile
stderr closures. Its Main-derived worker wait 9/output EOF and independent
retained-pidfd terminal witness are separate from those Scope receipts.

A distinct ordinary UNIX accept-EINVAL RED showed an existing private request
processed after Stopping. The one-guard repair passes the unchanged socket test:
no reply is sent after Stopping and peer custody is retained until close. That
case uses an authored unavailable/unoccupied Service, with no normal Main/worker
startup or network creation. Its full QA has 18 Scope waits/PIPE EOFs, three
regular stderr closures and one separate cache-helper NativeChild wait/read-0
EOF. Formatter compile failures remain setup evidence, not behavioral REDs.

[Hash-only provenance](evidence/native-connectivity-private-ingress-provenance.json).
The [source-only fixture](../tools/native-connectivity-private-ingress/README.md)
preserves the measured staging layout. Installed startup, public app permissions,
Settings, ARM, Wi-Fi, DNS, Internet and all phone gates remain open.
