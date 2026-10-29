# Native preview startup diagnostics

Normal Rust Owner and audio Main now record local preview cutoffs and the selected
retirement predicate. The Source clamp, deadlines, guards, probe order/counts,
state transitions and retry policy are unchanged. The
[Sounds report](native-cold-omarchy-sounds.md) retains earlier failures and the
separate graphical/audio evidence.

## Recorded fields and bounds

- `Authority::begin_media_preview`: captured Source, original app and effective
  forwarded cutoffs; existing request/check samples and current-authority result.
- `audio_owner::play`: forwarded cutoff only after actual successful send.
  WouldBlock/Busy emits nothing and retains the existing Unsent phase.
- Main: existing entry/post-start samples, startup/work cutoffs and admission
  result. `poll_preview` distinguishes work expiry, startup expiry, negative
  `alive()`, and completion while Starting. Its timestamp is the existing sample
  **before** `process.poll()`, not a new retirement-time sample.

The retirement-predicate line emits only when sampled state is not Retiring;
all original retirement calls remain. No counters/logging state are added.
`None` marks unevaluated liveness: deadline selection implies neither alive nor
dead. A negative `alive()` result is not a natural-exit diagnosis.

New `preview ` lines omit identities, credentials, routes, tokens and payloads.
The unchanged Main collector filters retained PID/invocation, bounded at
64 records/65,536 bytes. Owner uses its existing invocation-bound
256-record/1 MiB tail. Journal snapshots do not establish service wait/EOF.

## Qualified normal builds

| Selection | Existing checks | Normal output | Direct Scope wait/EOF |
| --- | --- | --- | --- |
| Owner diagnostic Sol3 | 117 lifecycle library tests, 26 preserved ignored; 34 SourceOwner/cleanup/status/startup/Stop controls; formatting and strict Clippy | x86_64 Release, opt3/debug0, features empty, 3,401,744 bytes | 11 outer + 19 inner |
| Audio diagnostic Sol2 | 55 default package tests plus the explicitly selected Main wire test; formatting and strict Clippy | x86_64 Release, opt3/debug0, features empty, 1,360,296 bytes | 11 outer + 8 inner |

Dependencies remain frozen to qualified Owner Sol2's base `63c855c4` and prior
audio inputs. Only three files differ in Owner's 271-file snapshot; only Main
differs in audio's 52-file snapshot. All shared inputs agree; the later
connectivity publication is excluded. Raw, formatted and compiled bytes agree.
Original cutoffs, wait/EOF, same daemon, exact container absence, unchanged inputs
and cleanup qualify.

Unexecuted Owner preparation Sol1 preserves its pre-container AST failure.
Failed Owner Sol2 preserves 60 passes/57 failures from omitted lifecycle fixture
environment, with no normal Owner export and qualified cleanup. Sol3 uses the
existing successful actor recipe/helper and unchanged 90-second library-test
bound; no test predicate changed.

## Measured cutoff propagation in one passing GUI repetition

One fresh x86 KVM diagnostic repetition passes its consumers. Owner/Main journals
are bound to retained unit startup identities. Fixed fields measure:

| Existing sample | Remaining time to the recorded cutoff |
| --- | --- |
| Owner request sample → original app cutoff | 999.947 ms |
| Owner request sample → captured Source/effective forwarded cutoff | 81.370 ms |
| Main entry sample → that same forwarded startup cutoff | 55.741 ms |
| Main post-start check → that same forwarded startup cutoff | 55.015 ms |

Captured Source, effective forwarded and Main startup cutoffs are equal. Main
records `predicate=admitted` and `admission_ready=true`; work remains six seconds.
No startup-retirement predicate fires. The later Playing→Failed retirement is
part of the successful explicit operator-revoke check.

This measures the current clamp, not the earlier failure's predicate/cause or a
repair/repeatability result. Full tone/revoke and pre-Play Source recovery remain
separate evidence. Operator grant/revoke is fixture administration, not consent
UI; no ARM, phone or host-speaker claim follows.

[Hash-only evidence](evidence/native-preview-startup-diagnostics.json) binds the
three source files, normal providers, build receipts, preserved failures and
the measured runtime records. Raw journals and PCM remain private.
