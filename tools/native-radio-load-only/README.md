# Public native radio load-owner containment precursor

Only public authored harmless children may run in this precursor. No original
linker/Bionic/vendor library, factory entry, registration or physical device is
admitted. The first selection uses the unchanged production `Supervisor` to
expose missing filesystem/Unix-IPC containment. Its existing responsive stop,
actual wait and durable uncertainty behavior remain the ownership baseline.

The experimental owner now passes six actual Arch ARM64 kernel7.1.6 groups in
`public-containment-green-3`. The preceding unchanged-owner RED is preserved as
`public-containment-red-1`: actual outside-root read and independently accepted
Unix connection, with existing stop/actual reap/durable refusal succeeding.

The final public controls qualify:

- Held child with responsive native status/stop, retained pidfd and direct waits.
- UID/GID12020, empty groups/capabilities, final-credential parent-death readback,
  no-new-privileges and architecture-checked kernel seccomp.
- Private mount/PID/network/IPC namespaces, readonly minimal root/proc/dev,
  no device nodes or outside-root descriptors; only FD1/2/3 survive startup.
- Actual denied outside-root/IP/Unix/device/namespace/filesystem access. Unix
  socket creation is allowed; connect/bind/listen/accept/sendmsg are forbidden.
- RLIMIT/cgroup readback,512MiB mmap refusal,61 readonly opens before EMFILE,
  and15 threads plus root before task refusal, followed by actual thread waits.
- A traced second-exec attempt killed before dispatch; `execs` counts attempted
  syscall entries rather than successful executable replacements.
- Queued readiness drained after the original absolute CLOCK_BOOTTIME deadline
  remains inadmissible. Parent death after final credentials produces SIGKILL
  and the independent PID1 driver actually waits the orphan.

The maximum recorded status/stop sample is2.413108ms, within the declared250ms
observed budget. This is instrumented QEMU scheduling evidence, not a hard realtime
promise. One original5s startup deadline and first-terminal2s cleanup deadline
remain unchanged; no SDK call or blocking ptrace/child wait runs in the actor.

Two failed aggregate labels remain failed. `public-containment-green-1` passed
pre-exec containment readbacks, then the ordinary public Rust executable entry
aborted before its checkpoint. The public C-ABI Rust entry preserves closed FD0
and absent `/dev/null` and passes. `public-containment-green-2` passed all six guest
groups, but its outer wrapper incorrectly checked exit visibility immediately
after stdout EOF. A bounded real QEMU wait is its sole final wrapper correction;
all final parent/child bytes are unchanged from that guest-pass/wrapper-fail run.

Build with `bash tools/native-radio-load-only/run.sh build`, freeze a fresh label
with `run.sh freeze LABEL`, then coordinate the single timing-sensitive guest slot
before `run.sh green UNIQUE-RUNTIME-LABEL LABEL`. Inputs/backing are readonly with
an explicit small qcow2 overlay; the canonical readonly input lock spans actual
Docker wait and post-input hashing. Never reuse an output or freeze label.
`python3 tools/native-radio-load-only/verify.py` checks preserved bindings and
closure without executing children; an optional path creates a new safe summary.

Successful local kill/reap never establishes remote semantic cleanup or permits
a successor registration. Original linker/Bionic/vendor execution, loader
namespace binding and maps, optional-input/property behavior, factory/registration,
installed ownership and Pixel policy/hardware remain separately gated. The public
controls do not authorize the next original-runtime stage.
