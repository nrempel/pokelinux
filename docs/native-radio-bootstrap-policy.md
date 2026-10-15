# Native radio bootstrap deadline and Stop controls

Four additional controls pass through the production bootstrap/Supervisor under
an actual generic Arch ARM7.1.6 kernel with the same retained public Rust Binder
peer. Production runtime source is unchanged from9b1948b. These are new coverage,
with no manufactured behavioral RED. The earlier parent-death and reap-error
RED→GREEN remain separate in the [bootstrap report](native-radio-bootstrap.md).
[Bounded execution record](evidence/native-radio-bootstrap-policy-provenance.json).

| Control | Actual boundary and result |
|---|---|
| Live wrong parent | A live independent same-UID/GID peer pidfd is transferred in otherwise valid BOOT. Actual child wait1, PermissionDenied, no READY or observed success; peer registrations0. |
| Late queued GO | The Rust fixture receives actual credentialed READY, retains the child pidfd, sends SIGSTOP and observes kernel state T. It queues valid GO before the original three-second deadline, keeps the child stopped across expiry, then sends CONT after expiry. Actual child wait1 with TimedOut and no success canary; peer registrations0. |
| Parent polling withheld | The real Supervisor remains occupied and denies admission while polling is withheld beyond its own immutable three-second deadline. Its next poll returns TimedOut. Actual poll_reap returns child exit1; same-owner and recreated-owner registration remain refused by the retained mode-0600 journal. |
| Stop during Initializing | The real Supervisor is stopped before its first bootstrap poll. Subsequent poll remains Terminal and admission remains denied. Actual poll_reap returns SIGKILL9; same-owner and recreated-owner retries remain refused. |

Late GO's actual wait is observed29.715879ms after the original deadline, before
the fixed refusal cutoff at deadline+1s. The remaining second of the original
cleanup window stays reserved for error disposal. Parent controls also record
actual wait timestamps and their original disposal cutoffs. Source checks and
the external parser both reject late observations; no cutoff is renewed.
The guest preserves a late direct-wait receipt before failing, and repeated
cleanup calls reuse that receipt.

The parent policy canary's first deliberate successful-body action is exit42.
Neither parent selection observes that success. Their `body=false` witness is
source-backed absence of observed canary success. It does not independently
instrument every instruction in the KILL/return race. The late-GO control uses
actual READY and the typed TimedOut result; it does not substitute this parent
policy canary observation for queued-frame qualification.

The Rust fixture remains the sole worker reaper. The guest waits each control
client with exit0, keeps the same independent peer live through all four
controls, observes no REGISTER transaction, then actually waits peer teardown.
No guest second reaper or process-reap-to-vendor-cleanup inference is added.
The parent policy cases do not claim READY was already queued when expiry/Stop
occurred; that exact observation remains an open control.

All37 native x86-64 radio library tests pass, including a new actual non-main
thread launch refusal with no child or admission journal. This runtime test does
not substitute for the owner's explicit non-Send/non-Sync type contract. Strict
host formatting and all-target/all-feature radio Clippy, fixture and containment
tool Clippy, and separate ARM client/peer compilation pass. Root independently
checks67 source bindings, both unchanged production source regions, all four
actual controls, recorded wait cutoffs, frozen inputs and full outer closure.
Actual Docker CLI wait0, fresh exact-CID absence and same-daemon checks pass.
The weaker intermediate planned envelope remains preserved and unexecuted.

The new public fixture modes are `bootstrap-late-go`,
`bootstrap-withheld-poll` and `bootstrap-stop-initializing`; each requires the
already initialized private guest Binder device and an owned live public peer.
The frozen private guest supplies those prerequisites and outer custody.
These controls are infrastructure; no production transport-worker executable,
service manager integration, vendor discovery or user-facing phone state is added.

SIGSTOP/CONT scheduling is not actual system suspend. Matched Google
kernel/modules/firmware, service-manager/cgroup/descendant ownership, stronger
containment, unkillable kernel work, vendor reset/cleanup, IMS semantics,
modem/carrier and physical hardware remain separate gates. No original Android
or vendor runtime, private REA, authentication/PAM or physical phone is selected.
