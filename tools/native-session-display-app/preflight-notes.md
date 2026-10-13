# Actual actor-preflight cleanup signal-order probe

`preflight-run.py` freezes the selected ELF/source inventory and runs a separate
no-NIC readonly ARM guest. It does not edit the Root fixture, shared implementation
or joint guest/runner. The guest launches actual original Hyprland, transfers its
owned pidfd and inherited session socket to the Root fixture, and supplies a
trusted root-owned profile. No app launches and no session source frame or SDK
reply is fabricated.

A root-owned regular `admin.sock` file causes the actor's actual bind syscall to
return `-98` (`EADDRINUSE`). Both the display supervisor and Messages projector
have already spawned their actual children. The SDK is explicitly initializing;
no Ready context, Wayland application or secure presentation is qualified here.

The guest traces only its owned service using Linux `PTRACE_TRACEME`,
`PTRACE_SYSCALL`/`PTRACE_O_TRACESYSGOOD` and `PTRACE_GET_SYSCALL_INFO`. It follows
neither child, changes no register/syscall return, and supplies no fake Provider,
SDK roundtrip or completion. Every actual signal target is resolved against
service-owned child command lines; pidfd targets additionally use kernel fdinfo.
The tracer's retained actual child pidfds establish liveness at syscall entry and
death/process absence after the service's own cleanup. Direct processes have real
wait results. Tracing changes scheduling, so this proves signal order rather than
uninstrumented latency, hard cancellation timing or exhausted cleanup budgets.

The chronological selected RED uses `owned-display-context-build-1` service ELF
`c76d4d96395d1c537838d08210583c5970c41859817b75c4b6ddedc298a29321`.
Actual preflight bind fails before the first signal. `kill(348,SIGKILL)` then
returns0 while initializing SDK346 is still live. Only later does Root invoke
`pidfd_send_signal(...SDK346...,SIGKILL)`. The original service exits1 with
`AddrInUse`; both children are actually reaped. This violates the desired SDK
fence-before-first-Messages-drain ordering. It does not demonstrate two exhausted
successive2s cleanup budgets or a held writer that survives SIGKILL.

The RED result has13 cleanup records for9 unique PIDs; four already-waited setup
commands were recorded again during final bookkeeping. All9 actual process
outcomes pass. Subsequent helper-only instrumentation suppresses those duplicates and retains
actual SDK ownership after the emitted spawn line, before its channel can close.
This permits target resolution even if an SDK exits during its own fence. The
current helper bytes are not retroactively the executed RED helper.
Frozen old helper sources/results remain unchanged.

Reproduce with a new label and the retained old build/backing:

```sh
python3 tools/native-session-display-app/preflight-run.py --phase red --build owned-display-context-build-1 --label preflight-new-order-red
```

The desired RED exits1 only after its genuine bind/signal/liveness assertions and
actual cleanup complete. A later separately selected corrected fixture can use
`--phase green`; it must put the SDK first. No new runtime build is implied by
changing this probe's bookkeeping. Existing labels are immutable.

Kernel UAPI references (checked13 October2026):
[ptrace syscall information](https://github.com/torvalds/linux/blob/v6.6/include/uapi/linux/ptrace.h)
and [AArch64 generic syscall numbers](https://github.com/torvalds/linux/blob/v6.6/include/uapi/asm-generic/unistd.h).
These define the tracer interface; execution is the actual Arch ARM7.1.6 kernel,
not Pixel kernel/KMI or physical-device evidence. Runs retain the existing readonly
production backing under its shared input lock, exclusive per-label output lock,
4 guest vCPUs and6 CPU/8GiB container limits. No original/private binary is published,
no VM image is cloned, and no phone or unrelated workload is accessed.
