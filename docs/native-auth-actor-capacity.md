# Actor authentication capacity release

The lifecycle actor now has a private capacity-only completion consumer. It
removes one exactly matched attempt slot so a fresh explicit attempt can be
admitted through the existing gates. It supplies no authentication result,
receipt or restoration authority. All40 lifecycle tests pass in the required
root-owned storage profile; the22 actor tests separately pass as UID1000.
Strict formatting/default/all-target/all-feature Clippy and separate ARM64
library-test compilation pass. [Bounded provenance](evidence/native-auth-actor-capacity-provenance.json)
keeps those profiles and the failed prerequisites separate.

ClosedAuthenticationAttempt has private actor-incarnation, non-reused attempt-ID
and original-deadline fields, with no Clone, Debug, serialization or production
constructor. Only a cfg-test builder authors closure facts. The consuming method
compares that tuple and, on a match, removes only the slot. Invalid, absent,
wrong-owner/deadline and successor/replayed stamps do nothing. It does not read a
clock, refresh Source, advance an epoch, touch storage, dispatch work or start a
new attempt. The checked-add attempt sequence is retained, so a discarded handle
cannot identify a successor even when both deadlines are equal.

Capacity reclamation is deliberately separate from result eligibility: closure
can arrive after retirement, original deadline or terminal Source loss. Release
cannot revive the old handle or Source. A new begin still needs the original
live-locker/Source/generation/lock-epoch gates and its own valid deadline. Durable
pending intent, grants, retained history and locked policy stay unchanged.

Seven new controls cover fresh explicit succession; wrong actor/deadline;
absent/successor replay; terminal Provider loss; actual Source expiry without
renewal; actual attempt expiry; and newer lock admission. They call actual
Authority methods through typed Provider facts and retain real locker pidfds.
The closure stamp remains synthetic. Locked-policy assertions are not an
installed Messages read or a cleanup proof. No actual Source IPC, broker,
ScopeChild/Attempt, PAM or authentication cgroup runs in these tests.

The initial RED is a missing-private-API compile prerequisite (E0432/E0599,
Cargo101), with zero executed bodies and no demonstrated runtime defect. A later
incorrect UID1000 full-suite selection passes33/fails7 existing root-storage
requirements; all seven new tests pass, but the aggregate remains failed.
A root-profile output-directory permission failure executes zero tests. Neither
label is relabeled as GREEN. Fresh complete selections supply the same-build
storage daemon and isolated root-owned /tmp and /opt:40/40 pass as UID0/GID0,
with a supplementary results-directory group1000. The separately selected
`service::auth_attempt_tests::`22/22 pass as UID1000;18 other tests are explicitly
filtered there, not silently skipped. Both containers launch with cap-drop ALL,
no-new-privileges, read-only source/rootfs and no network. Actual CLI waits0,
immutable source and fresh exact-CID absence on the same daemon close.
The full and focused runs produce byte-identical host and AArch64 test ELFs.

A future production stamp producer must bind actual whole-resource closure to
the exact actor connection/incarnation, attempt ID and immutable deadline,
including cleanup after result authority expires. An expiring ClosedOutcome is
not the only possible capacity-closure proof. The actor must not accept a raw
snapshot, PID death, caller enum or public success request as that proof.
This patch deliberately adds no producer or ingress. A future positive path must
validate and retain its armed context before dropping the attempt slot; this
capacity-only consumer cannot provide that context afterward.

Contained original PAM/scope with effective retained faillock, broker ownership,
actor result arming, ordered locker release, exact-object unlock, serialized
synced durable clear and normal Messages reopening remain gates. Source stays
100ms without renewal/revival; neither this host selection nor ARM compilation
qualifies actual suspend, the matched Google kernel or hardware. No phone was
accessed. The earlier [attempt admission proof](evidence/native-auth-actor-attempt-provenance.json)
retains its earlier no-release source.
