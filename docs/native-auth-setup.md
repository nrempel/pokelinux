# Native authentication setup-error responsiveness

The safe Rust parent now disarms unqualified child pipes after setup failure while
retaining its Child/pidfd, sticky fault, retirement and occupied slot. An actual
Arch ARM failure precedes the minimal repair. A hardened setup-fault control and
the separate two-group normal cleanup regression pass. This remains an isolated
component, with no production broker, actor authentication receipt or restoration.

The fixed `setup-fault-fixture` feature implies `isolated-fixture`. It performs a
real spawn and pidfd_open attempt, then discards any acquired handle. Its syscall
return is not independently traced. The child self-stops in a fixed no-workload mode that cannot launch PAM. Ordinary
entries and the default crate gate retain their existing policy. This does not
claim actual syscall denial, resource exhaustion, original PAM or phone behavior.

The unrepaired setup branch returns before either child pipe receives NONBLOCK.
Its next poll enters ARM read63 on the exact held supervisor stdout pipe. The
actual RED records the matching blocking FD before and after a
293,510,421 ns probe, with no response or partial frame.
The failure is frozen before separately labeled driver9/9 teardown. All 39 inputs,
actual waits, adopted-child identity, EOF/ECHILD, empty leaf and exact-container
absence close; the aggregate remains failed. Successful normal admission never
qualified this path.

Only after that RED, the repair sets the failed owner's input/output Options to
None after Setup retirement. It does not mark EOF, report scope closure, release
occupancy, detach a worker or retry. Actual Child try_wait remains available;
potentially blocking transport is no longer used when its flags are unqualified.

The first repaired run receives a fresh occupied/faulted Setup report in
1,909,163 ns. Independent review finds a latent harness
gap: it did not mandate the initial report before queuing the command, and its
postwindow FIONREAD cannot establish a 250 ms consumption bound alone. The actual
record contains that initial report already drained before the command, so the
second identical report establishes its response. This favorable schedule does
not qualify the generic observer; its source, passed aggregate and finding remain
preserved separately.

A fresh hardened selection requires that initial-report barrier and zero residue
before `S?`, plus the original fresh-report 250 ms bound. No core ELF, attempt
deadline, resource or cleanup oracle changes. It responds in
1,329,625 ns while its child stays live T, the parent stays
occupied with flags0x41/Setup6 and the leaf stays empty. The later matching
nonblocking command pipe/FIONREAD0 sample is corroboration. Proof precedes actual
independent parent wait9 and adopted-supervisor wait9, followed by both retained
pidfds dead/proc absent, stdout EOF, empty leaf and driver/outer ECHILD. Driver
teardown is fixture closure; native cleanup and slot reuse are not claimed.

The separately compiled normal `isolated-fixture` ELF excludes fault modes. Its
guest, delegated driver and boot bytes are identical to the earlier executed
two-group [scope selection](native-auth-scope.md). Cancel responds in
2,704,973 ns, Stop/snapshot in
2,117,303 ns and a snapshot after the fresh
original15 s deadline in 1,109,046 ns. Both held
supervisors resume into native recursive cleanup: worker9, two adopted waits9,
global ECHILD, EOF and populated0, actual supervisor/parent waits0, scope flags
0x1f9c and parent0x38. Each closes five retained pidfds/proc entries without
independent SIGKILL or outer adoption. Fresh admission follows full first closure.

Root independently reviews the actual records, rehashes all 39 RED, 53 first
GREEN, 57 hardened GREEN and 111 normal-regression bindings, and verifies fresh
same-daemon exact-container absence. Seven host components, strict default/
isolated/setup Clippy and both ARM GNU feature builds pass separately. Every
attempt retains its original15 s BOOTTIME budget, the 30 s maximum and existing
30/35/80/110+5/120s driver/service/guest/VM/outer bounds. VMs use 2 CPUs/1 GiB,
containers 2 CPUs/2 GiB and a 512 MiB RAM overlay over readonly locked backing, with
no network, attached host devices or phone access.

The next gates are privileged-helper containment, original PAM with the scope,
a fixed NNP0 broker joined to the NNP1 actor, exact one-shot unlock receipts and
synced durable restoration into a normal Messages read. Root-owned cgroup DAC
alone does not contain EUID0 helpers. Non-SIGCHLD clone coverage, namespace/proc
aliases, descriptor policy and installed resources remain open. Actual suspend,
Google matched-kernel scope, secure frames and all physical models are unqualified.
Process waits do not undo PAM/vendor effects or authorize uncertain retries.
See [bounded provenance](evidence/native-auth-setup-provenance.json) and
[authored drivers](../tools/native-auth-setup/README.md).
