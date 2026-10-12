# Development handoff

## Goal and constraints

Finish the full Omarchy smartphone system described in `smartphone-goal.md`.
The earlier call-bridge goal was expanded to native Rust phone userspace: calls,
cellular/IMS, SMS, camera, power, audio, connectivity, sensors and background/lock
behavior, with secure local controls, packaging and Omarchy theme integration.
Pixel 10 is the first target; common services must support separate hardware
backends. Retain matched Google kernel/modules/firmware initially. Android/Java
components are reference and test infrastructure, not the intended final runtime.
Use open source first and REA for closed interfaces. No backwards compatibility.

Build and test everything possible offline. Do not access or modify a physical
phone. Keep synthetic emulator, original-code, ARM64/QEMU and physical evidence
separate. Do not equate registration, submission or API success with carrier
acceptance, delivery, native call state or working physical hardware.

## Current stopping point

The native raw-response publication gate, absolute suspend-aware session expiry,
owned registration worker/death watch, and local process reclamation are implemented.
They are not native IMS semantics or a complete phone service.

- `RadioOwner` fences session/revision/preparation identity and checks Linux
  `CLOCK_BOOTTIME` expiry under the gate. Withheld timer delivery cannot permit
  late activation or publication. Candidate/operation lifetimes remain open.
- `TransportSession` owns one registration worker and retained death watch. Stop
  and snapshots do not join or unlink. Cancellation/expiry retain occupied worker
  capacity; completion is independent of the failed callback stream. Late success
  cannot activate, and every second attempt is refused.
- `Supervisor` revokes process output before SIGKILL and retains ownership until
  reaping. Its exclusive, synced private admission marker persists uncertainty
  before launch and survives supervisor restart. It never removes that marker or
  treats reaping as remote cleanup. No successor registration is permitted.
- Real Binder passes held cancellation/expiry, death before/after activation,
  permanent blocked-child termination/reaping, stale-tag refusal and durable
  restart refusal. Component red/green tests include worker detachment, death-result
  ordering, output revocation and stalled reaping. Existing IPC and six Binder
  dispatch-wrapper tests also pass on Harbour with SELinux enforcing.

Evidence: `evidence/radio-deadline-provenance.json`,
`evidence/radio-worker-provenance.json`, `evidence/radio-supervisor-provenance.json`.
Java changes are synthetic peer controls only; no physical phone was accessed.
Harbour now has authenticated `gh` write access and the repository-local existing
author identity. Authentication remains outside source control.

ARM64 user-mode QEMU passes component state, journal and provenance tests, but
its failed-exec subprocess test unexpectedly returns a successful child for a
missing executable. The two process execution tests are now compiled for ARM64
and explicitly ignored by the user-mode suite. `tools/radio-process-vm.py` runs
them under an Arch Linux ARM guest kernel in system-QEMU, as the unprivileged
`alarm` user. Both pass, along with the component tests. See
`evidence/arm64-radio-process-provenance.json`. This closes the process-specific
gate; ARM64 Binder, matched Pixel kernel/policy and physical hardware remain open.

## Next work

1. Recover and test the actual vendor cleanup/reset and registration-overlap
   contract. `clear` is one-way. Do not remove uncertainty markers, retry, use a
   recycled numeric registration ID for delayed cleanup, or call local reaping
   vendor cleanup. Keep registration unavailable without proven remote recovery.
2. Integrate these primitives into an installed Rust service and its service
   manager. Define authenticated permissions and observer IPC with process/session
   fencing, bounded output, safe durable admission and bounded diagnostics.
   Qualify ARM64 Binder and matched Pixel kernel/policy separately; the owned
   ARM64 system-QEMU process gate now passes.
3. Extend ownership to request transport, operation receipts, absolute candidate
   and operation deadlines, timer delivery, notification watches and aggregate
   budgets. Recover producer identity before enabling fragmented semantic output.
4. Recover native IMS state/response contracts with original-code comparison and
   integrate the shared phone service. Continue calls, SMS, camera, power, audio,
   connectivity, sensors, background/lock behavior, permissions, apps, deployment
   and genuine Omarchy integration in the full smartphone matrix.

An advisory Pro worker-lifecycle consultation was stopped at the user's handoff
request before a terminal answer. It is not a completed review or accepted design:
https://chatgpt.com/c/6acd2fad-19c0-83e8-b059-f88452119b16
Earlier completed owner reviews are recorded in `native-radio-owner.md`.

## Verification and publication

Use the existing prepared Linux checkout and its private inputs. Follow
`AGENTS.md`, strict red-green TDD, and `remote-development.md`. Update README and
affected docs each work turn when implementation or evidence changes. Run:

```sh
bash tools/build-native-radio.sh build
bash tools/framework-suite.sh binder-kernel
bash tools/framework-suite.sh radio-ipc
# In the prepared network-disabled graphics simulation container:
python3 tools/radio-process-vm.py
```

These are focused native-radio gates, not proof that the full smartphone goal is
complete. Keep scope-specific evidence and explicit remaining gates. Public source
is on `main` at https://github.com/nrempel/pokelinux. Commit and push source/docs
as work progresses, after checking staged files. Preserve private artifacts and
pre-existing untracked files; never stage them wholesale. Check GitHub write
authentication on a new host without copying credentials from another machine.
