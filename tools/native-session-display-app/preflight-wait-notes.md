# Actual service-parent wait4 after preflight fencing

This fresh kernel boundary selects the same final static service ELF
`1a4d606e41e85422c999a16c3adda7072eaccb2999fe3d25bf9b61cf89e13c57`
from `owned-display-preflight-fence-build-2`. It changes no native/shared/fixture
source and reruns no normal UI suite. It retains the same actual original
compositor, genuine admin bind `EADDRINUSE`, initializing SDK and SDK-before-writer
signal boundary. No SDK Ready or exhausted cleanup-budget claim follows.

The new `preflight-wait-guest.py` additionally traces the owned service's actual
ARM `wait4` syscall entry/exit and reads only its kernel-written four-byte status
word. The [primary Linux generic UAPI](https://github.com/torvalds/linux/blob/v6.6/include/uapi/asm-generic/unistd.h)
and installed header both confirm wait4 number260. `PTRACE_GET_SYSCALL_INFO` is
unchanged. The helper neither follows children nor rewrites registers, syscall
returns or status memory. Positive PID returns qualify parent reaping only when
the returned PID is the requested owned child and status is exited/signalled,
with no stopped status or WNOWAIT. Zero wait returns are recorded as not reaped.

Selected actual ARM execution `preflight-fence-wait-green-arm-v3` passes:

| Actual service343 kernel observation | BOOTTIME nanoseconds | Status |
| --- | --- | --- |
| SDK347 signal invoked before writer cleanup |30062636052 | Already-dead SDK pidfd; no successful-kill attribution |
| Writer351 SIGKILL invoked |30193563962 | Actual syscall returns0 |
| wait4 writer351 returns0 |30196145059 | Not reaped |
| wait4 returns writer351 |30200967398 | Kernel status9; terminal signal9 |
| wait4 returns SDK347 |30333578435 | Kernel status256; normal exit1 |
| Service exit observed |30426862551 | Original preflight AddrInUse exit1 |

Both terminal waits occur before the service exit is observed. The returned PIDs
match the service's already tracked direct children. Each wait uses WNOHANG1 and
the kernel-written terminal status is independently decoded. All9 unique tracked
process outcomes also pass actual direct wait or final pidfd death/process absence.
This distinguishes the service's positive parent-wait evidence from the earlier
RED/GREEN1 helpers' final absence observations. Those old196 proof files and seven
authored preflight paths remain unchanged and are not relabeled as wait4 proof.
The SDK's normal exit1 also prevents interpreting its already-dead signal target
as proof that a SIGKILL caused its death; exact SDK exit cause is not established.

An initial new v2 attempt is retained unselected: its new Python guest had an
indentation error and performed no native test. The helper was corrected and
passed AST parsing before this fresh v3 execution. No native behavior or deadline
was changed by that instrumentation correction. Current wait probe sources and
selected v3 sources have separate exact bindings.

```sh
python3 tools/native-session-display-app/preflight-wait-run.py --phase green --build owned-display-preflight-fence-build-2 --label preflight-new-terminal-wait-green
```

The separate [wait evidence](preflight-wait-evidence.json) binds terminal wait PID,
status and timestamps, signal order, both old proof references and the unselected
setup. The same immutable readonly backing/shared input lock, exclusive label
lock,4 guest vCPUs,6 CPU/8GiB container and no NIC remain. This lane commits/pushes
nothing and accesses no physical phone or new private input.
