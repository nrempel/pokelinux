# Closed-outcome components on an ARM64 kernel

The unchanged library tests from `a020e99` pass under an actual generic Arch ARM
kernel7.1.6-1-aarch64-ARCH. All62 host/ARM test names match:59 pass and3 explicit
ignored helpers are invoked through their parent regressions. This is system
QEMU with a real ARM kernel, separate from host execution, cross-compilation,
user emulation, matched Google images and physical devices.
[Bounded provenance](evidence/native-auth-closed-outcome-arm-provenance.json)
binds the source, actual ELF, inventory, adapters, logs and Root review.

The guest drops groups/GID/UID directly in Python and records UID/GID1000,
groups[] and CapEff0 before executing the exact test ELF. It uses no runuser or
PAM session and exposes only loopback, with no QEMU NIC. Both the full and
ignored-only test listings exactly match the frozen host inventory; no filters,
source changes, silent exclusions or deadline extensions are applied. The
unchanged suite completes in4.54s.

The actual ScopeChild regressions use ordinary owned test peers. The live-peer
wait/EOF gate, issued-token Stop/expiry refusal and five malformed-stream
quarantine cases pass. Each negative case requires inner actual natural
wait0/EOF/dead pidfd/proc absence before its marker and occupied Drop abort6,
then exact outer wait6/EOF/death. Those nested markers are captured inside the
parent tests: the external log supplies passing final assertions bound to the
executed ELF/source, not an independent raw nested wait journal. The original
Conversation helper separately prints its actual closure boundary. All10 pinned
boundary checks match.

The nested timeout fallback still cannot establish an independent inner wait;
it remains failed infrastructure. Container teardown is not direct nested
reaping or native scope cleanup. Response capture still compares only an
expected-length public prefix. PID100, worker waits/ECHILD/EOF/populated0 and
candidate records remain synthetic. Worker/issuer extraction controls are
models, not actual Attempt or PAM ownership.

The guest listing and component processes are actually waited0 with stdout EOF
and proc absence. QEMU naturally waits0 after29.528s within its original360s
bound; CLI and container wait0. Root freshly verifies both build/run exact CIDs
absent on the same Docker daemon. Root independently rehashes132 unique bindings,
including22 source pins in every source copy, the ELF, kernel/initrd/base,
private16GiB clone, adapters and execution artifacts. All match. No failed label
or external teardown is counted as success.

Only the library test ELF runs. The fixture binary, actual Attempt/Terminal
loop, original PAM/helper, authentication cgroup workload, effective faillock,
broker/actor result, unlock, durable clear and Messages reopen remain unexecuted
in this selection. The earlier [ARM40](native-auth-components-arm.md) keeps its
older source. The [closed-outcome host report](native-auth-closed-outcome.md)
retains the separate refusal RED. No matched Google kernel or phone is accessed.
