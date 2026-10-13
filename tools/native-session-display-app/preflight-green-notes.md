# Actual preflight cleanup fencing correction

The separately selected `owned-display-preflight-fence-build-2` static service
`1a4d606e41e85422c999a16c3adda7072eaccb2999fe3d25bf9b61cf89e13c57`
passes the same genuine actor admin-bind failure and signal-order boundary as the
[chronological RED](preflight-notes.md). Root changed the fixture only after that
RED: the combined session provider stops immediately after actor return, before
Messages drain, and fallback cleanup shares the original retained deadline.

The GREEN still returns the real actor preflight `EADDRINUSE` error/exit1. Actual
syscall order is SDK345 `pidfd_send_signal(SIGKILL)` before writer349
`kill(SIGKILL)`. The service emits Initializing/occupied, then Stopping with
admission false and occupancy retained, then Stopped after actual child reap.
All9 unique tracked process outcomes pass. At the first SDK signal, its retained
pidfd is already dead and the fixture's fence has already closed bootstrap/channel.
The trace does not attribute SDK death to successful signal delivery or establish
the exact SDK exit cause. Ownership identity was retained before those closures.

The exact RED `preflight-run.py` driver bytes are reused for GREEN. The guest
helper has only the explicitly documented post-RED instrumentation changes:
suppress repeated bookkeeping and capture child identity after the actual spawn
stdout syscall. The failed bind, native signal ordering assertion, syscall
instrumentation and actual wait/pidfd cleanup mechanisms are unchanged. No syscall
return or Provider/SDK response is replaced. The old executed RED helper and every
historical result/source remain frozen.

This is initializing-SDK preflight fencing order and actual cleanup under an ARM
kernel with original Hyprland. It does not qualify SDK Ready, app admission,
secure presentation, uninstrumented latency, exhausted successive2s budgets or
Pixel/device behavior. The shared original budget is a source property separately
reviewed by Root; this trace does not exercise its exhaustion. No blocked writer
was invented by SIGSTOP, and no app/UI/normal composition result is added here.

```sh
python3 tools/native-session-display-app/preflight-run.py --phase green --build owned-display-preflight-fence-build-2 --label preflight-new-order-green
```

Each new output label is immutable. Existing readonly VM backing/shared input
lock,6 CPU/8GiB container,4 guest vCPU and no-NIC limits remain unchanged. This
lane edits only new preflight-* tools and owned ignored outputs; Root owns the
native/shared/fixture fix and the joint agent owns separate normal regressions.
No commit/push or physical phone access is performed by this lane.
