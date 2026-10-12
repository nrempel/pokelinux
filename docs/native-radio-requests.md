# Native radio request ownership

`TransportSession` now owns one bounded request worker on the exact Binder
transport used for registration. A non-clone `PreparedRequest` captures its owner
allocation, session key, opaque request key, routing fields, payload and absolute
deadline. Submission rejects a permit from another owner. The payload budget is
4096 bytes; this is a deliberate local bound for the qualified raw/two-part path,
not a recovered maximum for every vendor operation.

Preparation reserves one receipt with `Queued`. Each part takes dispatch
permission under the same gate that admits callbacks, checks session phase and
`CLOCK_BOOTTIME`, and records `OutcomeUnknown` before calling Binder outside the
mutex. Cancellation, stop, death or expiry before the first permission records
`NotSent`. After permission, none can prove the remote effect did not occur.
Repeated cancellation preserves the first terminal reason and receipt revision.
Dropping a queued permit records NotSent; dropping dispatched work leaves unknown.
Receipt revisions are separate from raw-observation revisions; observer/watch
notification integration remains open.

The worker sends each original-qualified part once, in order, checking permission
again before every part. Its report counts successful one-way submissions, while
an error can still leave the failing part uncertain. A successful one-way return
is only local submission. Even a matching late raw callback may become a raw
observation without accepting the operation: response metadata/IMS semantics are
not yet qualified to finalize it. There is no successful modem-outcome variant or
implicit replay in this milestone.

One registration and one request attempt are allowed per supervised process.
Serial1 is never reused in that scope. Request retirement, subsequent operations
and production correlation require a qualified immutable final receipt and safe
wire-key allocation. Local cancellation does not reopen the request slot. A
request receipt also prevents direct owner replacement from erasing uncertainty.
The existing synced supervisor admission marker remains registration-level
uncertainty; it is not a durable request ledger.

Registration and request work share occupied capacity. Local stop and snapshots
never join either call. Completion remains in the owned JoinHandle and is joined
only after it finishes, independently of callback failure. An unfinished request
worker is dropped before the death watch so its abort guard precedes any possibly
blocking unsubscribe. A finished Binder transaction, thread join or process reap
is not vendor cleanup. Permanent request-call blockage and driver async-queue
pressure still need their own actual Binder qualification; the existing permanent
registration and generic worker/process tests remain separate evidence.

## Executed boundaries

Strict runtime TDD first registered through actual Binder, prepared a request,
advanced the controlled absolute clock to its deadline while withholding timers,
and observed the independent peer receive one raw request. The expected count0
assertion failed with exit101. The immutable failing source, client/APK binaries
and log remain ignored under `artifacts/native-radio/request-red`.

After guarded admission, the same scenario observes0 calls and NotSent. Five
additional actual SDK37 x86 scenarios qualify cancellation and stop before
permission, actual service death before submission, a complete4096-byte two-part
send, and a held response after successful one-way return. The latter cancels
before releasing the peer's bounded barrier, then publishes its actual raw echo
without changing OutcomeUnknown. Kernel Binder and SELinux enforcing are used;
the Java endpoint and control transactions111..113 are synthetic test infrastructure.

Five component tests cover expiry between part permissions, dropped/cancelled
queued work, exact owner identity and first-terminal receipt preservation. They
pass on x86 Linux, ARM64 user-QEMU and under an actual ARM64 Arch Linux kernel.
The full prior real-Binder registration, publication, cancellation/death and
permanent-registration supervisor suite passes too. Updated generic worker abort
and synced-admission failure tests also pass under the ARM64 kernel. ARM64 Binder,
matched Pixel kernel/policy, vendor discovery, native IMS/call state, modem
acceptance, durable operation admission and actual carrier behavior remain open.

Reproduce with `bash tools/build-native-radio.sh build`, then
`bash tools/framework-suite.sh radio-ipc` on Harbour. The network-disabled
`tools/native-process-vm.py` runner executes exact ARM64 test artifacts under a
snapshot Arch guest with no NIC and a read-only project mount. No physical phone,
proprietary execution or credentials are involved. The [provenance](evidence/radio-request-provenance.json)
pins authored code, compiled artifacts, immutable red, actual Binder outputs and
separate ARM64 user/kernel results.
