# Normal Main idle receive

Normal connectivity Main now passes a controlled native x86_64 empty-idle
regression with the same worker and a newer fact after resume.
The unchanged desired consumer fails against the preceding normal providers and
passes against the candidate. A consumed 48-byte record still expires on its
original 50 ms envelope; the parent-death control also passes. This is host process
and private-namespace evidence, without Settings, ARM execution or phone access.

Main checks a non-consuming readiness hint before starting a receive envelope.
Empty input defers that start; it does not renew a started envelope or change
its 50 ms duration, action cutoffs or 250 ms observation lease. Received frames
retain their original authentication and expiry checks. The four production
source inputs and all artifact hashes are recorded in the
[curated evidence](evidence/native-connectivity-main-idle-receive.json).

| Controlled case | Baseline | Candidate |
| --- | --- | --- |
| Empty Main receive | Actual private EAGAIN, normal loop sleep and next gettid precede one 80 ms hold. Main records receive_failure/TimedOut; desired same-worker/fresh observation fails. | A source-qualified three-descriptor zero-readiness poll supplies the empty anchor. The same hold preserves Main and worker; after worker SIGCONT, a newer available/unconfigured/not-pending fact arrives. Only later external Stop is latched. |
| Consumed record | Main consumes 48 bytes, then one 80 ms hold crosses its original 50 ms envelope. Retirement qualifies the negative control. | The same 48-byte expiry remains a receive_failure/TimedOut, with no second receive, late success or retry. |
| Parent death | Actual Main kill/wait/EOF precedes adopted wait of its known worker, signal 9. | The same ownership control passes. This is not an ordinary Main-owned worker wait. |

The worker is safely parked after its completed observation send and postchecks,
at loop-sleep entry, before tracing Main. Its retained pidfd, stopped state and
`TracerPid=0` confirm silence throughout the Main hold. Source-role qualification
binds the three-descriptor readiness poll and distinguishes the two-descriptor
identity poll; descriptor count alone is not the proof. Neither frame contents,
process memory nor runtime pollfd arrays are read, and syscall results are not
changed. Main detaches at an actual nonterminal stop before Scope resumes normal
terminal-wait ownership. Both traced cases admit zero Apply requests.

Each epoch closes three fresh containers before one in-process desired
assertion. Baseline owner passed=true qualifies the expected RED; its desired
assertion still fails with the exact marker. Candidate desired assertion passes.
There is no separate consumer-child exit. Actual waits/PIPE EOF, namespace
handles, exact container removal/absence, daemon identity and unchanged inputs
qualify both aggregates. Root1 remains a closed selector setup failure, outside
behavioral RED: the corrected fixture accepts connected abstract/no-path
SEQPACKET while retaining exact socket-inode selection and excluding ingress.

Candidate QA passes 45 library and 8 owner tests, with 1 library test ignored.
Formatting, strict all-target Clippy and host/ARM compilation also pass.
ARM component tests remain compile-only. The separate [actual fork control](../tools/native-parent-channel-fork-readiness/README.md)
rejects an inherited readiness hint while the original parent remains usable;
it supplies no Main/GUI or fork-baseline RED claim. The original held G4/G5
[action-expiry and Stop controls](evidence/native-connectivity-main-readiness-held-controls.json)
now pass with the qualified compiler output and unchanged bounds. The first
compile failed with `E0463` using an incomplete artifact set; the complete
selected Cargo artifact set compiled. The compiler image then lacked Python and
an expiry launch exited 127 before the fixture started. Neither setup failure
executed a held test or supplied RED.

The earlier [worker receive control](native-connectivity-idle-receive.md) consumes
19-byte commands; this Main control consumes 48-byte observations. Their source
cohorts remain separate. This controlled empty-input defect does not establish
that graphical root4 was empty, consumed a record, or hit a particular pre/post
guard. Root4's timing cause and root2/root3 remain unexplained. No CPU/IPv6
attribution or complete graphical repair follows; the new GUI candidate has not
executed. Continue with the unchanged graphical slice and installed cold-boot
composition.

[Exact Main fixture and reproduction](../tools/native-connectivity-worker/main-idle-receive/README.md).
