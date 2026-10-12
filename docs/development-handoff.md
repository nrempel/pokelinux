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

The raw-response owner in `native-radio/src/owner.rs` fences publication with
opaque session identity, revision and a preparation token. Callback failures,
cancellation and replacement withdraw current state. This is not a native IMS
semantic decoder or a complete service lifecycle.

The owner now requires an absolute boot-time session deadline. Four component
regressions failed before locked expiry checks and then passed. Real Binder also
passes `expire-during-registration`: a controlled clock reaches expiry with timer
delivery withheld while registration stays held, and late successful ID 42 cannot
activate. The owner checks expiry at every gate acquisition; this does not prove
hardware wake or candidate/operation lifetimes. See
`evidence/radio-deadline-provenance.json`.

The earlier test adds a bounded synthetic registration barrier. The native client
stops the real owner while `RadioTransport::register` remains blocked in Binder,
then rejects activation when the independent peer returns successful ID 42.
`held-registration-provenance.json` records the source and binary hashes and
red/green results. Those historical scoped-thread results have now been extended: production
`TransportSession` owns one worker and its death watch, retains capacity after
stop/expiry, reports late success without activation, and refuses another attempt.
The owned death scenarios also pass before/after activation. See
`evidence/radio-worker-provenance.json`. Permanent-hang supervision, durable
uncertainty and the real vendor cleanup/reset contract remain unfinished. The Java change only adds test
barriers. Existing owner, registration-death and ordinary IPC scenarios also pass.

## Next work

1. Read `native-radio-owner.md`, `native-radio-integration.md`,
   `device-portability.md` and the completion matrix in `smartphone-goal.md`.
2. Extend owned registration work to the remaining transport operations,
   candidate/operation deadlines and permanent-hang recovery. Absolute suspend-aware
   session deadlines are implemented; timer delivery remains unfinished. Avoid detached
   blocked threads, an indefinite join in Drop, or releasing worker capacity
   merely because local state was cancelled. Keep stop and snapshots responsive.
3. Establish the process supervision contract before claiming bounded recovery.
   Reject obsolete process/session output. Reaping a process proves local
   reclamation; it does not establish vendor cleanup or permit an uncertain
   registration to be retried. `clear` is currently one-way. Recover and test the
   real vendor cleanup/reset contract instead of assuming it.
4. Integrate recovered native IMS semantics and the phone service. Continue the
   full smartphone matrix; do not reduce completion to the radio harness.

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
```

These are focused native-radio gates, not proof that the full smartphone goal is
complete. Keep scope-specific evidence and explicit remaining gates. Public source
is on `main` at https://github.com/nrempel/pokelinux. Commit and push source/docs
as work progresses, after checking staged files. Preserve private artifacts and
pre-existing untracked files; never stage them wholesale. Check GitHub write
authentication on a new host without copying credentials from another machine.
