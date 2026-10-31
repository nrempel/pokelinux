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

## Remaining join

This is native x86_64 component evidence with authored Source and installation
inputs. The audio check uses a spy, not live playback. The normal session-owner
installation checks and Permissions client/UI are still Audio-only at this
checkpoint. Network consent through those installed paths, live cross-service
isolation, ARM execution and cold-boot persistence remain to be joined. It does
not establish new phone, radio, carrier or hardware support.
