# Runtime language decision

2026-10-12 UTC. First-principles review requested by Nicholas, including a preference to keep Java only where Android integration needs it.

## Updated target — 12 October 2026

The later explicit user instruction supersedes the Android-retention rows below:
required phone userspace services are to be implemented in Rust. Use REA for
closed-source behaviour recovery across IMS, camera, power, SMS and other
subsystems. Keep Google's matched kernel, drivers and firmware initially.
The earlier bridge architecture below is historical test infrastructure, not
the final completion target. See `smartphone-goal.md` for authoritative scope.

## Current language boundary

The phone-service target is Rust. The current production direction retains
upstream Hyprland, Aquamarine and Qt, which contain substantial C++. Our own
Qt client adapters and small compositor plugins also contain C++; these are
separate from the Rust phone services and must be counted honestly.

The [Sounds client engine](native-sounds-rust-client.md) now owns direct and
queued requests, packets, original deadlines, reply validation, pagination and
the accepted UI model in Rust. Actual C ABI, unchanged Qt socket tests and full
host application linking pass. Qt retains credentialed nonblocking socket I/O
and QObject/QML presentation. The obsolete public queue ABI is removed.
A fresh prepared native Qt6.12 run now qualifies this whole engine in all four
GUI/audio groups, with eight PCM windows/192,000 measured frames. The earlier
queue-only client retains its own evidence. Cold installed startup and ARM
window/session integration remain open. The [Messages engine](native-messages-rust-client.md) now also owns request,
parser/model, pagination, generation, scheduling and worker-lifetime policy in
Rust. Nine frozen C ABI groups, actual Qt/worker controls, thirteen permanent
CTest cases, default full-host linking and an ARM64 archive build pass. A later
manual generic ARM kernel build links the normal Qt6.12 app and executes nine
C ABI, seven typed worker and five Qt ordering groups. See [manual ARM evidence](evidence/native-messages-arm-client-provenance.json).
A separate native ARM production CMake Release build and guest installation now
pass with the explicit matching archive and nine-group consumer. Normal-main/
window, ARM default Cargo and the full installed ARM cohort remain unqualified.
See [CMake/install evidence](evidence/native-messages-arm-cmake-provenance.json).
A later [prepared x86 Messages session](native-messages-prepared-tracer.md) now
passes denied/granted history, original Omarchy live themes and revocation in
one normal app/window with native captures and owner Stop/closure; that recorded
cohort predates the shared appearance port. The [shared appearance client](native-appearance-rust-client.md)
now puts schema/cache/generation/readiness/deadline decisions in Rust. Qt keeps
credentialed socket I/O, descriptor/notifier retirement, timer delivery and
QColor/QML conversion. Fresh host component/full-app/default-Cargo/install checks
pass. A [new normal Messages Qt6.12 session](native-messages-appearance-rust-tracer.md)
also passes all four visible groups with both Rust engines and owned closure.
Both current normal apps now separately pass [native ARM production CMake
build/install](native-current-arm-apps.md), their frozen nine/six C ABI groups,
matching app/shared-appearance links and guest loader checks. Current Sounds
engine and production service/launcher ARM crossbuilds also close, with service
runtime still unexecuted. Both normal ARM mains/windows, ARM default Cargo,
current Sounds GUI and the complete running ARM shell/service/Source cohort
remain open. Upstream Qt/Hyprland/Aquamarine retain their existing C++ roles.

The custom renderer experiment added substantial authored C++ scene, protocol
and GPU integration. That work is now parked as component research. It is not
a prerequisite for normal app launch or progress porting phone services to Rust.
Its Rust frame ownership, credits and format planner are research results too;
the normal upstream compositor does not acquire them merely by being supervised
by a Rust process.

Rust owns shared phone policy, permissions, service lifecycle, deadlines,
recovery and Stop authorization. Upstream graphics engines retain their native
rendering operations. Keep dependency corrections small and test their actual
boundaries. New phone-service policy, implicit retries or a second readiness
authority must not accumulate in C++. The matched Google kernel/modules/firmware
decision remains unchanged.

## Graphics critical path

17 October2026: use normal upstream Hyprland initialization/rendering as the
next integrated phone-app tracer candidate, supervised by the Rust session owner.
Keep the custom CPU-scene/Rust-executor reconstruction and its exact-buffer,
credit and retirement tests as component research. They are not prerequisites
for launching an app through a normal compositor. This selects one intended
production direction; it adds no fallback or compatibility runtime.

The next measured slice joins genuine Omarchy launch and Source/focus with the
Rust permission/audio services, actual virtual PCM, visible state, live theme
change and app/service Stop. Earlier screenshots and separately passing
components do not qualify that composition. Keep Source's unchanged100ms lease,
real compositor identity and fail-closed revocation. Its rendering-stall
availability failure remains open; do not synthesize heartbeats or replay work.

Responsive service Stop, compositor process termination and verified native
display cleanup are separate requirements. Supervision establishes neither fresh
focus/lock facts nor display retirement. Inspect actual uncertain native retries
at their operation boundary; replacing the entire upstream renderer is not the
default remedy. Reconsider a focused control/render separation only when the
measured normal composition establishes the necessary blocker.

## Earlier recommended target

| Layer | Language | Reason |
|---|---|---|
| Linux phone/platform services and command clients | Rust | Explicit state types, ownership of resources and concurrent work, native ARM64 deployment, bounded IPC and durable state |
| Native app presentation | QML / Qt Quick Controls, with a small compiled adapter where needed | Matches Omarchy shell technology and permits independent app processes; see `ui-framework-decision.md` |
| Omarchy shell integration | Thin Quickshell/QML plugin | Uses the actual shell plugin and live theme interfaces |
| Android framework edge | Small Java adapter | Public Telecom/InCallService, permissions, roles, lifecycle and AndroidKeyStore remain Android-owned |
| Graphics/camera/vendor ABI edge | Existing C/C++, or narrowly scoped Rust FFI where it improves ownership | EGL, native camera, vendor libraries and Qt interfaces define the boundary; do not rewrite stable ABI glue for language uniformity |
| Build, audit and test tools | Python and shell | Useful orchestration without making the phone runtime depend on the test tools |

This is a target decision, not a claim that a Rust replacement already exists. Current production candidates still use Python on Arch and Java on Android. Rust is suitable, but adopting it requires integration proof rather than a syntax translation.

Nicholas subsequently accepted this decision and instructed completion with no backwards compatibility. The clean Rust protocol/socket/journal components now pass twenty-seven component tests on Harbour native Linux and twenty-six under ARM64 QEMU user-mode; saved source hashes match. Native tests include actual process termination after durable admission. Separate real emulator tests now verify Rust mTLS observations and strict Android request parsing on both architectures. This is not yet a daemon or a replacement release. The existing packaged Python runtime is historical verification material while the target is built, not a promised compatibility path or a second target to maintain.

## Why change the Linux runtime choice

Python provides sockets, TLS, SQLite and process tooling with little build setup. It made the current adapter, emulator environment and failure tests straightforward to exercise. That is a useful development benefit, but it is not sufficient reason to keep it as the default for an expanding smartphone platform. The existing service has independent observation threads, locks, session fences, command admissions and durable recovery. Rust is a stronger fit for expressing those lifetimes and states explicitly in the final service.

Python is memory-safe at the language level too. Rust's advantages here are not a claim that Python inherently corrupts memory or cannot be production-quality. A Rust port will add compiler/build and dependency maintenance costs, and native dependencies/FFI still require care. No measured battery, memory or latency comparison currently proves a performance improvement. Reducing polling and needless wakeups matters regardless of language.

[Rust supports the ARM64 glibc target](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-linux-gnu.html). Maintained interfaces exist for [Unix sockets](https://docs.rs/tokio/latest/tokio/net/struct.UnixStream.html), [TLS](https://docs.rs/rustls/latest/rustls/) and [SQLite](https://docs.rs/rusqlite/latest/rusqlite/). These establish feasibility, not a final dependency selection or compatibility claim. Pin and audit the selected versions and features in the actual build.

QML does not require Python or Java. [CXX-Qt](https://kdab.github.io/cxx-qt/book/) provides a Rust/Qt interop path through generated C++/CXX bindings. A service can also remain fully outside Qt behind IPC, using a small Qt-native client adapter. Choose the simpler verified boundary; do not put Rust inside the shell just to share a language.

Qt's separate [Qt Bridge for Rust 0.3](https://www.qt.io/blog/qt-bridge-for-rust-0.3-cxx-qt-compatibility-and-soundness) is explicitly beta as of its September 2026 announcement. Its existence is not a reason to make the system service depend on that UI binding. Keep the service independently buildable and test the chosen Qt/Rust binding against the pinned ARM64 runtime before adopting it.

## Keep the Android boundary small and honest

The current Java adapter uses public [InCallService](https://developer.android.com/reference/android/telecom/InCallService) and Telecom behavior. Android policy checks that must be atomic with a framework operation must remain on that side of the boundary. Do not relocate all permission or freshness checks to Arch merely to reduce Java lines.

[AOSP supports native Rust components](https://source.android.com/docs/setup/build/rust/building-rust-modules/overview) and a [Rust AIDL backend](https://source.android.com/docs/core/architecture/aidl/aidl-backends). That does not turn Java SDK classes into Rust APIs or make platform-only Binder interfaces available to an ordinary APK. JNI wrappers can call Java, but they retain the dependency and add an interface to maintain. Keep Java where it directly serves the Android framework contract; consider Rust for substantial reusable policy only when the extra JNI/FFI boundary has a demonstrated benefit.

This does not remove Java from Google's retained telephony/IMS stack. The project controls its own adapter language, not the implementation language of those dependencies.

A C ABI does not require a C implementation: Rust can implement or call it through a reviewed FFI boundary. Retaining the current narrow C glue is a migration-cost decision, not a platform requirement. C++ throughout would simplify direct Qt integration, but would add manual memory/lifetime obligations to services that do not otherwise need C++. It would not remove the Android framework dependency.

## Clean replacement requirements

Nicholas explicitly requires **no backwards compatibility**. Build one target runtime and deploy its matching components together. Do not retain legacy adapters, parallel production runtimes, old-format readers, schema migrations from earlier test iterations or downgrade support. Earlier implementations and reports are evidence, not a compatibility contract. Use fresh state for the new offline test deployment; do not erase historical evidence or unrelated data.

1. Define the target wire schema, enrollment format, trust checks, operation identities and durable state model from current requirements. Reuse behavioral scenarios from the existing tests, not their obsolete serialization choices.
2. Implement protocol parsing and typed state transitions with failing tests first. Enforce bounds, integer/boolean distinctions, unknown-field rejection and deterministic request identity within the target implementation. Reject unsupported versions explicitly.
3. Port durable admission and recovery. Exercise real SQLite commits, failed commits, process death, exclusive ownership, concurrent admissions, unresolved operations and capacity behavior. A transaction API call is not proof of equivalent commit semantics.
4. Port the transport while preserving TLS 1.3, mutual authentication, CA/hostname validation, certificate pin checks, key-file permissions, strict response identity and total deadlines. Do not replace standard certificate verification with pin-only acceptance.
5. Preserve the no-automatic-retry rule. Test cancellation after durable admission: dropping an async future must not silently lose responsibility for an admitted command. Keep known receipt, unknown outcome and remote invocation distinct.
6. Run the new binary through the installed ARM64 systemd/socket path and real Android emulator with matching target components. Verify incoming/outgoing, duplicate IDs, changed payloads, permission revocation, shell death, lost replies and restart recovery of the target's own state. Do not drive two command owners against the same backend or ledger.
7. Ship one Rust runtime after required behavior is verified. Retain Python only for independent test/build tools where useful. Measure idle wakeups and resource use before claiming an efficiency benefit.

The UI language review and a Rust migration are independent decisions. The secure protocol, test fixtures, Android integration and graphics work remain reusable.

## Independent critique

The Pro language review (private transcript) supports this split for new development and highlights rewrite risk. Retain its requirement to make commit-to-worker handoff cancellation-safe even if database work outlives its awaiting task. Its legacy digest/schema and rollback recommendations are superseded by Nicholas's subsequent explicit no-backwards-compatibility instruction. Compare behavioral fault scenarios in isolated runs, never by dispatching each real action through two implementations.

BRIDGE VERIFIED: Chat and Pro 5 of 5 selected; two marked user turns and their corresponding terminal responses; final response Copy/Rate/Share controls present, Stop absent; marker `PRO-BRIDGE-20261012-qt-91b3` matched. One language follow-up, no missing-context rounds. Owned tab closed after extraction. This is a critique of the proposed architecture, not verification of Rust code.
