# Shared appearance client policy in Rust

The shared app-local appearance client now runs its policy in Rust. Formatting,
strict Clippy, the counter regression, eight frozen C ABI groups and all29
unchanged native Qt cases pass. Fresh full normal Sounds and Messages host builds
also pass explicit/default-Cargo links, loader and installed-QML controls. The
AArch64 result is an optimized Rust archive build; new ARM Qt/window/session
execution remains open.

Rust owns the schema3 request and bounded JSON validation, normalized cache,
owner/revision checks, connection generations, readiness admission, checked
request/effect counters, deadlines, retry and change decisions. The existing Qt
event loop executes nonblocking socket operations, validates peer/packet
credentials, closes received descriptors and retires sockets/notifiers. It maps
copied RGBA16/style data into QColor/QVariantMap/QML and delivers timer callbacks.
Public appearance supplies presentation data, never session or app permission.

The11-operation C ABI copies values and retains no caller pointer, native fd,
provider authority, worker or thread. Calls and free must not overlap. Actual
native retirement precedes CLOSED/free; CLOSED proves local retirement only.
Rust checks the immutable generation captured by each native notifier callback,
so old readiness or result tokens cannot select work on a new connection.

Each refresh keeps one original500ms CLOCK_BOOTTIME deadline. WOULD_BLOCK and
INTERRUPTED require new actual readiness and do not renew it. Cached publication
expiry is independent. The constructor needs no successful clock sample, and
`available()` is a pure fresh-clock observation. The250ms periodic timer starts
once; Rust returns bounded request/expiry timer delays. Production time is not
injected. Counter exhaustion invalidates without identity reuse and withdraws
availability once.

The cache keeps nine RGBA16 roles, eight font tokens, six spacing tokens, base
size and font scale. Same-owner rollback and changed normalized content at the
same revision refuse publication. Invalidation retains the last accepted snapshot
for presentation without asserting freshness. The wire contract defines canonical
32-lowercase-hex owners and Unicode scalar JSON; this finite suite is not an
exhaustive Qt/serde parser-equivalence claim.

## Actual evidence

| Boundary | Closed result |
| --- | --- |
| Preimplementation interface | Existing Qt control exits0; target exits1 with exactly three missing new API names. This is structural RED, not an original runtime-policy failure. The separate original C++29-case baseline passes. |
| Counter repair | A compiled private unit first fails41 versus expected42 on request-counter exhaustion. The repaired one-unit run passes both request/effect branches. Assertion values remain unchanged through formatting and the struct-initializer-only fixture lint fix. |
| Rust component | Formatting, strict Clippy and one regression unit pass; optimized host/AArch64 archives expose exactly11 strong APIs. Initial two production Clippy warnings and the later unit-initializer lint remain build failures, separately from feature RED. |
| C ABI and native Qt | Eight frozen groups pass directly and as one permanent CTest, with assertions enabled. All29 original Qt6.8.2 cases pass against the new adapter, including native credential/rights/deadline and RGBA16/QML controls;29 final peer waits exit0. CPU-authored native-result reports in the C ABI suite do not substitute for these socket checks. |
| Joined normal host apps | Explicit and fresh default-Cargo builds link one app engine and one appearance archive each. Normal Messages has23 strong APIs and Sounds24; loader, install manifests, protected QML and desktop bytes pass. Native default commands actually use locked/offline/release Cargo for both manifests. Default archives were observed inside the closed container; byte identity to explicit providers is not claimed. |
| Installed component controls | Four unchanged Messages modes pass: installed, live appearance, Messages policy and arguments. Three run as UID11010 without a window; the argument fixture closes four actual normal-main refusal waits. No successful normal-main/window result is claimed. |

The Qt component epoch closes39 owned direct waits/stdout PIPE EOFs and30 regular
stderr descriptors, with peer waits separate. The joined app epochs close49
inside waits/stdout PIPE EOFs and four regular stderr descriptors; their four
normal argument-child waits are separate. Rust's14 foreground shell commands
write regular logs, not PIPEs. Original cutoffs, exact-CID absence, same daemon,
input/source closure and native child custody pass. The unchanged app9/13/audio30
suites were not repeated inside the joined build phases.

[Hash-only provenance](evidence/native-appearance-rust-client-provenance.json)
binds the exact staged18-file component and69-file joined cohorts, failures,
archives, binaries and closed results. Source hashes define the working port;
a later documentation commit is not relabelled tested source.

## Remaining integration

The earlier [ARM CMake result](native-messages-rust-client.md#native-arm-production-cmake-build-and-installation)
and [prepared Messages captures](native-messages-prepared-tracer.md) use the
pre-port appearance client. They retain their own sources and do not qualify
this new provider on ARM or in a visible session. A new current ARM cohort must
freshly build both normal apps with matching appearance/services/shell/Source
inputs. ARM default Cargo, full installed ARM composition and cold startup remain
open. A Messages-only tracer supplies no new Sounds execution/window proof.

Full font/settings/rendering fidelity, actual window/output/DPR/native capture,
trusted lock/focus/session ingress, native display cleanup and stuck kernel tasks,
physical suspend, incoming/send/modem/carrier behavior and matched Pixel hardware
are not qualified by this port. Upstream Qt/Hyprland/Aquamarine retain C++.
