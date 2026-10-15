# Isolated native session → current Messages authority fixture

This fixture runs the real lifecycle actor, native session Observer, native
Messages storage worker and SQLite projection in an owned Linux container. Its
facts source is a separately launched synthetic UID1000 process. The Messages
client is an actual installed test app launched as UID11010. Positive reads use
source focus/unlock facts and a separate actor policy grant; they never use root
unlock/background controls to establish foreground authority.

A root-created inherited `SOCK_SEQPACKET` channel carries actual kernel
`SCM_CREDENTIALS`; its creation-time credentials are recorded separately. The
installer transfers the source socket and a separately owned source pidfd to the
fixture service. The root test driver gets the launched app's actual
`SO_PEERPIDFD` from its private app-driver socket, retains it, and transfers it to
the source over a private control socket. The source retains that handle across
heartbeats. Every focused facts packet carries only PID/UID/GID metadata and
exactly one actual `SCM_RIGHTS` pidfd; null focus carries no descriptor. Received
control handles have atomic CLOEXEC and are closed on replacement, null focus,
source close and exit. Extra, truncated or mismatched control rights are refused.

The production Observer first checks that the retained source pidfd is living
and matches the expected source PID in the pinned procfs view. It then retains
the received focused handle. The actor compares
it with its already-owned launched app pidfd through the shared identity leaf's
pinned protected procfs view and a final joint liveness poll. Numeric focus PIDs
never cause a pidfd lookup. Device/inode metadata are recorded as observations,
not used to grant authority. This protocol works with host Linux6.8 shared
anon-inode pidfds and newer ARM pidfs handles. Deployment still requires one
qualified PID namespace and protected procfs mount.

The inherited descriptor arguments exist only behind `isolated-fixture` in this
separate crate. They do not add a selector to production `messages-service`.
Worker entry points dispatch to the existing native workers. Current combined
history/enumeration sources are selected by default. This selection does not
qualify a joint actual Binder/session run.

Run with the already provisioned local Docker images and offline Cargo cache:

```sh
python3 tools/native-session-authority/runner.py --phase red --protocol-baseline 84ac3f8 --label owned-focus-red --case unknown_denied --case root_controls_unknown_denied --case source_only_read --case anon_inode_owned_focus_read
python3 tools/native-session-authority/runner.py --phase green --label owned-focus-green
```

The explicit red selects immutable commit
`84ac3f856d9b71aeb7bccbfd33eb5a8a828b079f` for all production dependencies and
runs the actual `run_with_messages_and_session` ingress. Its earlier scalar
protocol rejects the desired owned-focus packets, so positive reads fail while
unknown-source denial and root-control fencing remain passing controls. No
production code or type declaration is overlaid. All selected crate locks are
regenerated offline in a private copy, recorded, and built with `--locked`.
These scalar-protocol baseline assertions were executed after the initial
owned-focus implementation was written; they do not establish test-first
chronology. The later source-owner constructor regression and its red-before-fix
ordering belong to the separate Observer selection. The former f733
missing-ingress fixture is preserved only in historical frozen artifacts.

Use `--case <case_name>` repeatedly to select cases. The default is 14 groups;
on a shared anon-inode host it adds a fifteenth positive group. Groups cover
unknown-source denial, separate policy grants, null/ineligible focus, exact
PID/UID/GID, a wrong living handle with the same metadata, a retained dead app
handle with a fresh generation's exact metadata, and fresh-generation grants.
Held read tests preserve coalesced lock→unlock→refocus invalidation, same-state
heartbeat preservation and the original100ms lease. Actual source SIGSTOP, EOF
and SIGKILL, real reply-queue EAGAIN retries, sticky root lock/background intents,
and responsive stop remain exercised. The dead-handle group also performs24
focus/null replacements and requires no source FD-count growth. The source may
observe dead focus and later recover fresh focus while remaining the same living
source; source loss itself remains terminal.

Labels are immutable and refuse overwrite. The runner holds its own `flock`,
owns each uniquely named container only after successful creation, disables
network access, and removes that owned container in a `finally` block. Direct
children remain owned until actual wait results are recorded. Service-owned
app/worker death requires both owned pidfd readiness and process disappearance.
Successful service exit alone is insufficient reap evidence.

`artifacts/native-session-authority/<label>/` contains selected source origins and
hashes, selected entry/features and image identity, offline build log, exact ELF
hashes, per-case identities/timestamps/reap observations, result JSON and proof
hashes. Raw local-history content and execution logs are kept private; authored
evidence publishes counts and hashes. [owned-focus-evidence.json](owned-focus-evidence.json)
indexes the current protocol selection and its limits.

The separate ARM64 system guest uses its own clone, lock, container and evidence
directory. It does not touch the compositor fixture VM. Selected source/ELF
hashes are checked both before boot and inside the guest, then child reap evidence
is saved and the guest powers off:

```sh
python3 tools/native-session-authority/runner.py --phase green --label owned-focus-arm-build --target aarch64-unknown-linux-gnu --build-only
python3 tools/native-session-authority/vm.py --build owned-focus-arm-build --label owned-focus-arm-green
```

This uses the actual guest Linux7.1.6-1-aarch64-ARCH kernel; user-mode QEMU cannot
qualify this boundary. PIDFS metadata are descriptive and not an authority gate.
Neither this guest nor the host substitutes for the retained Pixel kernel set.

## Selected owned-focus evidence

`owned-focus-host-red-2` records two passing unknown-source controls and two
failed desired positive reads against immutable84ac3f8. Both failures return
`denied`; all four setups and cleanups qualify. The preceding `red-1` also fails
both desired positives, but its second source-state command encounters a broken
pipe after scalar-protocol rejection. It remains exploratory evidence.

`owned-focus-host-green-4` passes all15 groups on actual x86_64 Linux6.8.
`owned-focus-arm-green-2`, built from `owned-focus-arm-build-2`, passes all14 groups
on actual ARM64 Linux7.1.6-1-aarch64-ARCH. Their53 production Rust/manifest hashes
match and include the corrected source-owner constructor. The host records109
reap entries for94 distinct recorded owned children; the ARM guest records102
entries for88. All recorded service-owned pidfd deaths and process disappearances
are observed. Excluding each intentional source-death test, bounded source
SIGKILL cleanup is required once on host and six times in ARM. These are qualified
waits, not claims that every source exited gracefully.

The separate Observer selection `red-source-owner-pair` records25 passes and one
failure: the earlier constructor accepted a wrong live pidfd while expecting
another source PID. That red precedes the constructor fix.
`green-source-owner-pair` passes all26 Observer cases with the corrected binding.
Those labels live under `artifacts/native-session-observer/`; their evidence is
maintained separately. These15/14 authority groups install the expected source
handle and do not exercise deliberate wrong-source constructor input.

The earlier host `green-1` preserves14 passes plus the fixture grant-sequence
failure while old dead-focus heartbeats remained. `green-2` passes all15 groups
before the private Rust packet representation was adjusted for strict linting.
`green-3` and `owned-focus-arm-green-1` preserve passing15/14 selections before
the source-owner correction. Their artifacts remain unchanged. Unprovable dead
focus streams do not establish stable grant availability; the dead-handle fixture
supplies fresh living focus before requesting the fresh generation's grant.

## Preserved scalar-protocol history

The previous13-group ARM scalar selection remains frozen at
`arm-green-authority-final`; its exact actual pidfs identity requirement passed
on Linux7.1.6-1-aarch64-ARCH. The old three-group host denial selection remains
`host-green-unsupported`, with `pidfs_identity_qualified: false`. The original
missing-ingress red is retained at `arm-red-proven`. Earlier setup/reap failures
remain in their original labels. [observed-evidence.json](observed-evidence.json)
indexes those historical source/ELF/result/proof hashes. They do not describe
the current owned-focus implementation.

These selections qualify synthetic local process/socket authority ingress. They
do not qualify original compositor/Quickshell protocols, trusted production PAM
unlocking, compositor configuration/containment, zero publication after an
unobserved external transition, a joint installed production session launch,
actual Binder/session composition, phone hardware, modem behavior or physical
suspend. The normal installed Messages entrypoint still has no production source
selector. No physical phone, proprietary executable or vendor input is used.

The fixture package also supplies a dev-only `History` library facade for
[native lifecycle retained-history tests](../../docs/native-session-restoration-history.md).
It owns the existing Projector and delegates the complete Provider contract through
inherent methods plus neutral token/packet events, avoiding lifecycle unit-test
crate identity mismatches. It adds no successful-auth constructor, feature or IPC.
The normal lifecycle public API remains unchanged. Actual retained native pages
and clean same-ID reopen pass in the separate headless restoration selection;
normal Qt/original compositor and genuine authentication remain separate gates.
