# Fixed network permission authority

The Rust lifecycle authority now supports an installed Permissions controller
for exactly `org.omarchy.Settings` / `network_control`, alongside the existing
Sounds / `media_route` configuration. Each installation still selects one fixed
pair. Requests cannot supply either field. The ordinary target must be installed;
protected profile/executable checks, live credentials, current focus, awake/unlocked
session, owner/revision checks and original BOOTTIME deadlines remain required.

The validated permission stamp now carries that fixed scope through status,
durable grant planning and post-write publication. Only an accepted Sounds audio
revoke withdraws the audio provider. A NetworkControl revoke preserves the
unrelated durable audio grant. No storage schema, wire selector, new daemon or
implicit retry is introduced. The lifecycle desktop route recognizes either
supported installed pair; its full graphical NetworkControl path is not tested here.

## Executed boundaries

The same three tests first fail at the expected missing-scope assertions, then
pass against the implementation. They exercise the protected profile loader,
a real UID11045 caller with SCM_CREDENTIALS and retained pidfd, synced storage,
and service dispatch through the actual storage worker. Candidate assertions also
reject caller-injected scope fields, preserve audio grant history and check zero
audio-provider revokes/effects through the existing spy. These latter assertions
were present but unreachable at the baseline's earlier scope failures.

Two existing audio controls pass in both cohorts. All 24 tests in the affected
profile, credentialed ingress and permission-service modules then pass: 6, 8 and
10 respectively. They retain focus/deadline refusal, busy handling, unsent
cancellation, submitted-write ownership, commit history, EAGAIN publication fencing
and bounded Stop checks. Normal host helper/libtest compilation, selected formatting
and strict Clippy pass. Every executed runtime closes its processes and exact
container; the two setup failures remain separate from assertion RED.

[Curated receipts and exact source/provider hashes](evidence/native-network-permission-authority.json)
record the executed cohorts. A helper-mount loader failure ran no tests. A later
candidate copy lost executable mode and also ran no tests; the fresh corrected
replay uses the same compiled bytes and checks executable access first.

## Supplemental session and audio boundaries

A later native x86_64 run passes all 28 affected tests: the same 24 above plus
four supplemental permission-service cases (6 profile, 8 ingress, 14 service).
It builds published `5482cac` authority sources with test-only additions;
the production authority sources and normal helper match the earlier GREEN.
These are supplemental regressions, with no independent RED or production repair
claim. The original three test bodies and earlier RED, GREEN and 24-test receipts
remain unchanged.

The three connectivity Session cases use an authored recording backend and remote
state model. Actual credentialed Network permission requests and storage writes
show that an unsent action stays unsent through revoke/regrant, transferred work
remains modeled remotely pending while late completion cannot publish, and an
established modeled configuration persists until a new explicit action. Regrant
submits no action. Switching the authored Source focus to Permissions also fences
pending stamps; these cases do not isolate policy-only causality. Existing global
policy revision invalidation remains in force.

The fourth case retains the real audio Owner on an actual UnixStream with normal
SO_PEERCRED validation against the fixture UID0 peer. Authored peer responses
pass through the normal decoder to establish an applied route, settled policy
and playing generation. Normal media
Session publication approves its retained lease, and later fences validate that
same lease. Network grant/revoke/regrant preserves the Owner incarnation, decoded
state, open quiet transport and durable audio grant. The peer requires EAGAIN,
rejecting both EOF and hidden writes. This is transport/client/lease evidence;
it does not establish a running audio Main, PCM or physical rendering continuity.

Pinned formatting/checks, normal release helper and library test compilation, and
strict all-target Clippy pass. Five loader checks precede the tests. All process
waits/PIPE EOF, handle closure/ECHILD and exact container removal/absence qualify
under the original bounds. Exact supplemental source, provider and receipt hashes
are appended under `supplemental_boundary_regressions` in the curated evidence.

## Remaining join

This is native x86_64 component evidence with authored Source, installation and
network backend facts. The earlier audio spy check and supplemental real Owner
transport/lease check retain their separate scopes. The normal session-owner
installation checks and Permissions client/UI are still Audio-only at this
checkpoint. Network consent through those installed paths, actual backend
cross-service isolation, ARM permission execution and cold-boot persistence remain to be
joined. No new phone, radio, carrier or hardware support is established.
