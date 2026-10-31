# Static installed Main startup entry: proposed contract

This preparation is based on88842e2. Production still requires five arguments,
including an absolute BOOTTIME cutoff. The proposed four-path CLI is not
implemented and the fixture has not run.

The normal entry should accept only these existing paths, in order:

```
pokelinux-connectivity-main /run/pokelinux-connectivity/control.sock /etc/pokelinux/connectivity/startup.json /run/pokelinux-connectivity/admitted /usr/libexec/pokelinux/pokelinux-connectivity-service
```

The first Rust statement should capture BOOTTIME, then checked-add the existing
three-second startup budget. Argument allocation/validation, protected profile
loading, listener creation and worker startup consume that same original budget.
Pass the one absolute cutoff to the existing main_service::run, Service and typed
WorkerStart. Accept no timeout option and no fifth-argument compatibility path.
Receive/action durations and cleanup are unchanged. No unit, installer, stable
interface selector, permissions or GUI implementation belongs to this change.

The two frozen fixture invocations reuse the earlier actual normal-Main
SETPCAP-only runner. Each retains its original one-second observation and
three-second closure budget, fixed16KiB output limit, actual Main wait/PIPE EOF,
retained namespace and ECHILD check. Initial UID/GID0 has SETPCAP only, NNP1 and
no supplementary groups; Main must reduce to cap0. NET_ADMIN is absent throughout.
An authored syntactically valid absent-device profile makes the normal worker
fail capability reduction before backend access. Only readonly Inspect and one
normal Stop are admitted for an entered Main. No network mutation is performed.

`--entry static` passes four fixed paths without a clock argument. Baseline must
naturally return1 with exact InvalidInput, absent endpoint/admission journal and
complete closure. `--entry absolute` passes the old fifth argument and serves as
the baseline environment witness: actual worker failure, cap0 Main, first
startup_failure with the supplied cutoff, then Stop and complete closure. Its
desired legacy-refusal assertion fails only after cleanup.

The pair qualifies behavioral RED only when both actual consumer exits are1,
the four-path parser refusal is observed and the old five-argument control really
enters startup. Loader/profile/capability/observation/custody failure is not RED.
After a candidate exists, the same consumer must observe actual four-path
startup and an original cutoff exactly three seconds after a point within the
parent's observed entry interval; the fifth argument must instead be refused
before owned admission. Both consumers then exit0. `qualify.py` is a pure paired
oracle over those receipts and the actual observed consumer exit codes.

The cutoff interval verifies derivation. First-statement placement is source
evidence unless a later explicit initialization-delay control is actually run.
This is host normal-Main CLI evidence only. It does not qualify installed PID1
startup, successful radio startup, an operational static wireless profile,
permissions, a graphical session, ARM runtime or hardware. Namespace identity and
ifindex resolution across boots remain separate work.
