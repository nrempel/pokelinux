# Actual compositor session facts fixture

This fixture builds a selected adapter against the exact installed ARM64
Hyprland package, launches the compositor directly as an owned UID1000 child,
and authenticates actual inherited-channel packets by kernel SCM credentials.
It uses an independent VM clone, lock, no-network container and no-NIC system-QEMU.
The monitor configuration is explicitly `320x240@60` with virtual graphics and
host software GL. No production permission/actor integration or trusted-locker
containment follows.

Current owned-focus run from the repository root; select fresh result labels:

```sh
python3 tools/native-session-owned-build.py new-owned-focus-build
bash tools/native-session-compositor/run.sh green new-owned-focus-compositor \
  tools/native-session-compositor/adapter.cpp \
  artifacts/native-session-observer/new-owned-focus-build/target/aarch64-unknown-linux-gnu/debug/session-observer-fixture
```

Each result preserves the fixture/adapter source, frozen diagnostic probe,
selected Rust sources, generated native lock-client inputs/ELF, package/header
identities, packets and actual owned-child reap results. Existing results are
never overwritten. The original build probe and shared Messages clone remain
untouched. The frozen probe supplies only private test controls; the adapter
exposes no such commands. Red admission is expected to fail and preserve the
actual packet that first exports null focus while still claiming unlocked.

The initial receiver is an authored same-kernel Python recorder with actual
credential, sequence and bounded-frame checks. Its current-state query uses the
original 100 ms BOOTTIME deadline, but it can record later packets after a gap;
its transition groups qualify facts observation, not the Rust Observer terminal
failure policy. Rust Observer qualification is explicit when its separately
supplied ARM64 fixture is selected; copying binary/source alone is not execution
evidence. A fourth optional runner
argument selects that binary and its matching preserved Rust source/build manifest
for actual Rust Observer epochs; the parent closes its receive endpoint before the
Rust child starts consuming.

Runtime evidence is spooled in guest-local `/tmp` and copied to the result share
only after actual child cleanup. This keeps synchronous 9p evidence I/O out of
the measured producer and consumer paths. Functional epochs retain the kernel
default send buffer; only the explicit stopped-consumer pressure epoch selects
a small buffer. Results include the effective buffer size, source/output identity,
first observed output closure and parent pump gap diagnostics. Rust epochs map an owned warmup native window before the first adapter load,
wait for its initial render work, and clear central focus with the frozen probe.
No facts have been sent before this preparation; the waiting Rust consumer has
never accepted or revived a source. This qualifies a prepared compositor fixture
only. Preserved cold-start runs exhibit actual source gaps beyond 100 ms and
conservative Rust expiry, including runs with entirely local evidence I/O.

No diagnostic
changes the adapter or Observer deadline, retries a failed source or revives facts.

The fixture deliberately permits its synthetic lock client to restore/unlock.
It does not qualify the original Omarchy lock/PAM/delegation contract, secure
presentation, forced PID reuse, actual suspend, cross-process atomic publication,
modem/carrier or physical-device behavior.

The preserved executed baseline `red-admission-final` records the original admission
ordering failure: the first actual null-focus packet exports `locked=false`.
Its guest, runner and frozen lock-client source hashes match the executed green
recipe; the adapter source diff contains the admission latch and deferred timer
reconciliation. The immutable corrected adapter in `green-prepared-1` passes all
17 groups:
actual package build, source credentials, native peer pidfs identity, null/layer
focus, admission latch, locker death, completion/restore, protocol/force unlock,
Rust facts intake and unload/stopped-source/backpressure/death denial. Its 41
recorded owned-child reap entries all succeeded. The selected Rust-live sample
gaps stayed below 100 ms until deliberate unload; stopped-source denial occurred
about 104 ms after the original accepted sample while its pidfd remained live.

`green-admission-2` through `green-admission-7` retain cold graphics availability
failures. Moving Rust logs locally, moving all runtime evidence locally, restoring
the normal send buffer and reducing virtual monitor dimensions did not remove
the initial expiry. `green-admission-6` observed the source output still open when
Rust reported expiry, with producer closure afterward. Original producer sample
timestamps in the Python diagnostic epoch include gaps of 249–475 ms. Mapping a
warmup window before the first adapter load permits the prepared Rust facts tests
to complete; the exact renderer/driver operation responsible for cold delays was
not traced. No successful cold graphics availability qualification is claimed.

Unload, pressure and source death each produced terminal `positive_state=false`.
The observed Rust error in those cases was `TimedOut`; these runs qualify bounded
denial with actual channel closure/source state, not immediate EOF or a specific
pidfd-death error path. The separate process suite covers its own synthetic peers.
All artifact/source/ELF selections are preserved privately in each result; no
raw logs, proprietary components or binaries belong in published source.

The current owned-focus selection `green-owned-source-final` replaces scalar fields
with actual SCM_RIGHTS peer pidfds. It passes17 original-compositor groups and
separately executes14 leaf and26 observer checks on the actual ARM kernel.
The immutable prior adapter fails the focused-descriptor requirement in
`red-owned-focus-baseline`.45 reap entries cover44 distinct direct children;
all reaped, with one bounded SIGKILL fallback. The current observer/actor source
is selected independently; this producer run does not compose the actor or app.
See [owned focus](../../docs/native-session-owned-focus.md). Earlier admission/
scalar selections above remain frozen historical records.
