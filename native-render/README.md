# Immutable SHM, frame values and worker components

This original Rust library owns bounded immutable CPU pixel snapshots for the
[connected Hyprland import](../tools/native-render-import/README.md). It copies
exactly `stride * height` bytes before returning success, retains format/shape and
import identities, and charges distinct live pixel allocations once across
explicitly cloned handles. Releasing the control store does not invalidate
snapshots. Each C handle has one owner; a caller must clone before transferring
another independently owned handle. Pixel views borrow that owned handle and
must end before its release.

The store admits at most512MiB; one snapshot admits at most64MiB and each extent
at most16384. The compositor wrapper selects a128MiB store. Invalid identity,
reserved fields, layout or input length publish no snapshot. Capacity refusal
leaves the input with its caller and consumes no additional budget. Rust owns no
Wayland, client-buffer, C++ scene, EGL or output object.

Bulk pixel allocation uses `try_reserve_exact`. Handle and `Arc` metadata use
infallible allocation: allocator exhaustion can terminate the process rather
than return `PR_ALLOCATION`. The capacity bound is a live pixel-byte budget,
not a bound on allocator overhead or a complete out-of-memory recovery policy.

The snapshot/Frame APIs do not move GL/context, GPU buffers, fences, output
commits or resource deletion to another owner. A separate
[Rust worker CPU core](../docs/native-render-worker.md) is now implemented and
tested; its genuine AQ/GL/native binding remains required.
A successful snapshot does not establish upload, GPU completion, presentation,
secure lock display, Source freshness or phone support. The prepared rendering
[authority baseline remains RED](../docs/native-render-authority.md).

## Build and actual boundary test

The crate is std-only and has no downloaded dependencies. For a native Linux
x86 build, run from this directory:

```sh
cargo fmt --check
cargo clippy --offline --locked --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
cargo rustc --offline --locked --release --target x86_64-unknown-linux-gnu -- --print=native-static-libs
```

Use the archive path Cargo actually reports, including any explicit
`CARGO_TARGET_DIR`. Compile `tests/ffi.cpp` against that exact archive and
`include/pokelinux_render.h`; link the native libraries reported by that Rust
build. The current x86 compiler reports `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`.
The C++ flags are `-std=c++20 -O2 -Wall -Wextra -Werror -pthread`.

The test uses an actual shared memfd and forked producer: capture, mutation,
unmap/close and actual child wait precede the retained-byte check. A separately
owned clone crosses a joined C++ thread after the original handle and store are
released. Remaining groups check ABI/store admission, quota lifetime and invalid
admissions without partial output. Success emits five named groups plus actual
producer and thread closure receipts under original BOOTTIME cutoffs. These are
CPU custody checks; they cannot qualify compositor import or rendering behavior.

Formatting, Clippy and the x86 Release Rust archive compilation have executed
successfully. A separate actual x86 KVM guest compiles and runs the C++ test: all
five groups pass, with producer wait/EOF and actual thread join retained. The
connected full Hyprland build and matching501-file SDK also pass. These results
remain separate from actual compositor runtime and rendering acceptance; see the
[build report](../docs/native-render-import.md).

A later separate [actual compositor no-attach case](../docs/native-render-import.md#actual-no-attach-identity-red)
qualifies RED: a retained unassigned surface is unnecessarily reimported despite
unchanged pixel bytes. A one-file incoming-buffer guard repair is authored;
its matching full rebuild/five FFI groups and unchanged actual no-attach oracle
pass. The repaired import identity remains stable7→7 with the same bytes. This
does not repair the seven-case Source/rendering RED or qualify upload,
server-resource retirement or complete rendering/output ownership.

A separate [actual padded-cursor consumer case](../docs/native-render-import.md#actual-padded-cursor-consumer-red)
now qualifies RED on a behavior-preserving native baseline with a502-file SDK.
The real retained cursor cache preserves stride144; the linked production Cairo
helper consumes stride128 and paints992 wrong pixels out of1,024. Its one-helper
stride repair passes its matching full native rebuild/five FFI groups and the
unchanged actual oracle: both live pre-client-exit paints check1,024 pixels with
zero mismatches. Later retained clones check original raw Rust pixels/padding
after client wait0, without invoking Cairo. This CPU consumer boundary is separate from a complete hardware-
or software-cursor output path, GPU upload and presentation.

## Compiled scene prerequisites

The public [31-file C++ cohort](../docs/native-render-import.md#qualified-scene31-checkpoint)
now builds all15 affected units and its matching505-file SDK. It adds factored
scene order, CPU frame records and owner-only GL primitives. The new producer
and primitives are not called and no render owner starts in the selected runtime;
a fresh unchanged Source7 replay still gives four controls and three rendering
failures. That ELF selects the earlier snapshot-only Rust archive69b81281, without
this separate commit-permit module. No connected frame/output result follows.

## Commit-permit component

The original Rust `commit` module is an executor prerequisite. A unique control
owner issues a move-only ticket retaining the full nonzero u64 security/output/
scene/session policy. A checked serial and one-shot CAS order sealing against
new accepted intent and sticky Stop; dropping the control owner now fences unused
tickets too. `invalidate_current` revokes unsealed work within the same security
epoch, without granting a replacement operation. Real Linux CLOCK_BOOTTIME is
read before and after the seal CAS. A sealed old-attempt record survives later
intent/Stop, but proves no native call,
submission or completion. Authentic policy values still require the real control
owner; integer shape alone does not authenticate output/session/scene/device
bindings. A worker must reserve job/outcome custody before issuing; the separate CPU core
below does so.
The deadline bounds the grant check only: preemption may place a later native
entry or completion past it.

Six initial native Linux x86 component tests pass. An earlier missing-implementation
compile failure executes no tests and is not semantic RED. Adding a seventh,
control-owner-drop test exposes a real failure: six pass and one fails with
Cargo/CLI101. A `Drop` Stop fence repairs it; all seven then pass with the same
drop-test expectation. An equivalent boolean assertion in an earlier test changes
syntax, so the complete test source is not claimed byte-identical. An eighth
case adds same-epoch invalidation; its earlier missing-API compile failure is also
not semantic RED. Final formatted-source QA passes all eight tests, rustfmt,
Clippy with warnings denied and the x86 release build. The new case exercises
the API with authored policy, without a real device-pause event. The existing
snapshot C ABI/header remains unchanged; its C++ FFI was not re-executed against
this new release archive.

Test-only OS threads are actually joined. The module implements no production
worker/JoinHandle ownership, FD queue, C ABI export, AQ/GL/native output call or
Source integration. These eight component tests are distinct from the seven
rendering-stall cases; connected frame commit/presentation and hardware remain open.

## Immutable frame values

The [`frame` module](src/frame.rs) now copies bounded CPU target/draw/material/
matrix/rectangle values and Arc-clones exact immutable snapshots. The implemented
[`pokelinux_render_frame.h`](include/pokelinux_render_frame.h) exports capture,
release and a synchronous visitor only. It uses `PR_FRAME_ABI_V1`, with no private
executor declarations or compatibility aliases. The visitor's pointers expire
before return; only explicitly cloned snapshot handles can outlive it.

Run the14 actual boundary cases separately:

```sh
cargo test --offline --locked --test frame_capture --target x86_64-unknown-linux-gnu
```

These tests were written after the implementation draft. An initial wrong layout
expectation is corrected without a production behavior change; final14-case QA,
format-check, strict Clippy and release build pass. A structural missing-public-
header expectation is separate and supplies no behavioral RED. Historical permit8
results are not added to this count.

A historical native FULL33/private-header selection builds16 affected units and
507 SDK files and passes five snapshot FFI plus four actual codec CPU groups.
The public-header C++ successor changes names/includes only; its separate fresh
native build now passes all16 affected units,507 matching SDK files, the same
five snapshot FFI and four codec CPU groups. The actor calls real capture/visitor/
clone/adoption using authored plan values, without a live scene producer, native
GL or production worker. See the [frame report](../docs/native-render-frame.md)
for matching evidence and unchanged Source/full-runtime limits.

## Retained worker CPU component

The original [`executor` module](src/executor.rs) owns its worker `JoinHandle`,
three admitted-frame credits and a separate128-record completion pool, reserving
five records before each admission. Normal Busy join/refusal retains ownership;
fallback Rust `Drop` stops and actually joins but can block indefinitely. Pending
native retirement keeps frames, completion delivery does not free their custody,
and native progress uses actual kernel eventfds/`poll` with finite ingress work.

Current [owned bootstrap/worker QA](../docs/native-render-bootstrap.md) passes36
library cases (8 permit/28 worker),14 Frame and12 bootstrap descriptor cases:
62 active total, with two ignored owned-child modes invoked by their parents.
Format-check, strict Clippy and release build pass. Earlier worker-only results
and the five new strict RED/GREEN pairs retain separate receipts.

`OwnedBootstrap::capture` copies the bounded initial session prefix and duplicates
its separate primary/render descriptors with CLOEXEC. The worker retains a
deferred holder before it calls `NativeOwner::bootstrap`; the adapter must take
the input into owner-held state before fallible native work. No live C++ graph,
callback or borrowed input pointer crosses this boundary. Discovery values must
match the transported session/device tuples before control can accept readiness.
Frames must match discovered output identity/generation/dimensions; final native
grants must match the discovered device lease/generation as well.

`Executor::new` requires one original absolute BOOTTIME startup cutoff. Register
both borrowed `completion_fd()` and `startup_deadline_fd()` before waiting.
Inspect `try_discovery` as well as frame completions on completion readiness.
Call `snapshot`, `try_discovery`, `try_submit` or `try_join` on a deadline event to
enforce expiry; never merely drain the timer counter. Ready is accepted by control
at its post-disarm BOOTTIME check, then its timer is disarmed. Stop/expiry is
terminal; native resources and the JoinHandle remain owned until actual drain and
join. Unregister both borrowed watches before `try_release`. Fallback `Drop`
still joins and may block. The ordinary BOOTTIME timer includes suspend time but
does not itself wake a suspended phone; actual suspend remains unqualified.

The bootstrap checkpoint archive1dcce61e was Rust CPU-QA only. The native Frame
ELF/507-file SDK and historical five FFI/four codec groups selected earlier
3cf0a142, without this worker. The current
[worker C ABI](../docs/native-render-executor-ffi.md) is now implemented in
[`pokelinux_render_executor.h`](include/pokelinux_render_executor.h) and
[`executor_ffi`](src/executor_ffi.rs), with current release archive246a6680.
Actual C++ lifecycle and five authored-owner groups pass, alongside all62 Rust
cases, format-check and strict Clippy. The C function table/scalar options move
by value through the single typed factory; native holders are worker-created.
The immutable thread guard precedes inner/slot/output access, and the shared
admission path consumes/nulls a Frame only after successful admission. Stop,
real BOOTTIME expiry and pending native cleanup retain ownership through actual
worker destruction/join. Unregister both borrowed watches before release, and
retain supplier code until join. No compatibility factory or second archive is
used. The later [matching AQ15/main/SDK build](../docs/native-render-executor-gl.md#separate-aq15current-abi-build-and-startup-failure)
passes CPU/loader checks and reruns these C++ groups. It does not rerun the62 Rust
cases. A subsequent matched runtime aborts during backend startup before Source/
actor, so production AQ/GL worker execution remains unqualified.

For a selected current x86 Rust static archive, build/run each actual C++ test:

```sh
c++ -std=c++20 -O2 -Wall -Wextra -Werror -pedantic -Iinclude \
  tests/executor_lifecycle_ffi.cpp /absolute/selected/libpokelinux_render.a \
  -pthread -ldl -lm -lrt -lutil -o /absolute/output/executor-lifecycle-ffi
/absolute/output/executor-lifecycle-ffi
c++ -std=c++20 -O2 -Wall -Wextra -Werror -pedantic -Iinclude \
  tests/executor_ffi.cpp /absolute/selected/libpokelinux_render.a \
  -pthread -ldl -lm -lrt -lutil -o /absolute/output/executor-ffi
/absolute/output/executor-ffi
```

The larger fixture keeps its original5s work/8s close cutoffs; observe its actual
process wait/EOF externally. Its authored callbacks and `/dev/null` descriptor
fixture establish neither genuine native routing nor vendor cleanup. Preserve
failed setup/fixture selections separately, as the report does. Genuine AQ/GL,
later session ingress/OOB revocation, complete actual native output/routing
authority and distinct Stop-retirement remain required.

A separate [actual native GL baseline](../docs/native-render-gl-baseline.md) now
executes real CLEAR/RECT leaves and retained Frame bytes on a worker EGL context,
with authenticated test-channel EOF/unload/join. It selects earlier Frame3cf0a142,
not this latest worker archive. The later [owned resource checkpoint](../docs/native-render-owner-resources.md)
qualifies actual SHM/fence and sampler RED/GREEN against that same older Rust
Frame archive, with explicit native plugin sources and exact readbacks. Binding
the production Rust executor to real native GL/AQ remains required; neither
component supplies Source7 GREEN or output/presentation qualification.


## Control-owned first-Stop observations

[`first_stop.rs`](src/first_stop.rs) owns a separate CPU-only first record and
saturating recording-call count, with creating Rust-thread identity. It is
created before broker construction; native authority Stop precedes recording.
The fixed record owns no worker, descriptor or callback. Polling may add terminal
reobservations to the count, so it is not a count of distinct user Stop actions.
Clock/broker fields describe post-authority observations. Observer release is
separate from native retirement and executor join.

After actual missing-API and normal wire REDs, all64 active Rust tests, formatting,
strict Clippy and Release pass; the old62 names and two ignored child modes remain.
All four compiled C++ consumers pass against this same new archive, including the
five first-Stop groups and two joined foreign threads. The native adapter's
matching normal main/SDK build now passes with all27 Rust exports, six strict API
checks and all CPU consumers. The unchanged actual native wire GREEN identifies
a direct AQ device-change revocation and preserves its first record across a
later public Stop and two snapshots. This does not establish normal readiness,
recoverable device-event handling, frames or native cleanup. See the
[first-Stop evidence and remaining gates](../docs/native-render-first-stop.md).

To exercise the new C ABI against the selected actual archive:

```sh
c++ -std=c++20 -O2 -Wall -Wextra -Werror -pedantic -Iinclude \
  tests/first_stop_ffi.cpp /absolute/selected/libpokelinux_render.a \
  -pthread -ldl -lm -lrt -lutil -o /absolute/output/first-stop-ffi
/absolute/output/first-stop-ffi
```

Use that archive's three actual headers and compiler-reported native libraries.
Do not substitute the older snapshot/executor archive for the new four functions.
