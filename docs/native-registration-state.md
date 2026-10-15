# Native registration observation

A data-only native-phone-state crate and the production radio Gate now provide a
conditional read-only registration view. Eight component tests and one separately
selected actual generic ARM Binder test qualify it after a genuine missing-positive
RED. Ordinary owners have contextNone, so the new getter cannot return an
affirmative registration view. Normal radio-service does not use/export this
view: its existing status remains transport/wire diagnostics and can still be
ObservedActive. No shared-view app delivery exists. Readiness injection is private
cfg(test)-only, with no production context writer or assume-ready configuration.

## Current value, history and origin

[Shared types](../native-phone-state/src/lib.rs) carry no transport or commands.
RegistrationView has an independent view revision, availability Unavailable or
RegistrationObserved, and optional last_observed history. Each positive current
or history value contains source_generation/source_revision: the owner's begin
revision and actual committed wire revision. Raw/wire history remains separate.

**Those pairs are unique only inside one RadioOwner.** Separate owners or restarted
workers can reuse them. They are not durable/global identity or authority. Before
combining/exporting values, bind actual provider/worker incarnation and subscription
and revalidate current state and permissions at send time. Cloned views remain
historical data; the getter and fields confer no lease or permission.

The [Gate mapper](../native-radio/src/registration_state.rs) applies only after
complete current-session publication checks and successful typed decoding. A
positive requires matching session/slot, live entity and prior-request membership,
state2, zero feature/mask/status, empty URI/reason and absent extensions. The APN
priors are private test inputs, not remote acceptance or acquired connectivity.

| Input or lifecycle event | Native read policy |
| --- | --- |
| Complete qualified state2 with admitted context | Publish RegistrationObserved and replace last_observed with the actual local source pair |
| Disconnected/unknown/unqualified or malformed matching indication | Withdraw current availability and retain positive history |
| Wrong slot or raw response | Preserve this view; wire history is independent |
| Stop, service death, BOOTTIME expiry or owner replacement | Withdraw and retire context, retaining the old history origin |

Malformed matching input can preserve wire revision while independently advancing
view withdrawal revision. Counters reserve terminal withdrawal capacity and refuse
reopening on exhaustion. This conservative policy does not assert original
asynchronous deregistration, APN removal, remote cleanup or retry safety. It creates
no commands, operation receipts or OutcomeUnknown result.

## Measured component and ARM evidence

The fixed [tests](../native-radio/src/registration_state/tests.rs) cover eight
component groups, including complete admission, source/history replacement,
wrong-slot/raw preservation, malformed withdrawal, held candidates, controlled-clock
expiry and revision exhaustion. They do not qualify actual suspend. Baseline
library tests report60passed/7missing-positive failures/1ignored. Repair reports
67passed/0failed/1ignored; the complete host suite totals78passed/0failed/7ignored,
including six older Android-driver tests still ignored. Strict checks pass.

The separate ignored ARM test runs only the public Rust context manager, named
peer and library-test ELF as UID1000. One actual lookup/registration/client42 and
initial raw historyR3 precede four exact slot1 route3012/state2 callbacks: three
missing-prerequisite combinations, then admitted context. Both phases commit the
same typed sourceR7. Baseline retains the default unavailable/no-history view,
returns its dedicated missing-positive error after cleanup and actually waits101.
Only the production registration_state.rs positive branch changes for repair;
all88 source paths otherwise match, including the test and peer, identical manager/
peer ELFs and all five frozen oracle files.

Repair observes sourceR7, then actual disconnected, unknown255, feature1, truncated
matching and wrong-slot callbacks. Control125 proves malformed wire revision is
preserved while view withdrawal advances. Public peer107 exits0; actual Binder
death-watch withdrawal retains the last positive source. No request, retry or clear
is issued. Rust registration-worker join/watch drop precede actual testwait0/EOF.
Baseline keeps the peer live through testwait101/EOF then guest pidfd-kills/waits9;
repair guest waits peer0. Both kill/wait manager9. All four guest direct children,
original BOOTTIME cutoffs, outer CLI0, fresh exact CID absence, same daemon and
unchanged inputs qualify. No installed unit, production worker, vendor runtime,
NDK, original APK, phone, auth/PAM or UI executes in these ARM runs.

The [bounded provenance](evidence/native-registration-state-provenance.json) pins
source/build/runtime receipts. Earlier [original conditional admission](native-ims-registration-admission.md)
and [installed wire history](native-radio-registration-installed.md) retain their
separate evidence scopes. Next are real APN ownership/acquisition, provider and
subscription binding, restricted installed service/app delivery, and later modem/
carrier/hardware qualification. Full original asynchronous loss/retry remains open
for operations depending on it.
