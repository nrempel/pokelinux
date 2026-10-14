# Native flat Color alpha checkpoint

The native Rust appearance owner now publishes schema3 with exact16-bit RGBA
channels, preserving the qualified Original flat popup Color alpha behavior.
An actual Original/Rust/Qt comparison fails before the repair and passes afterward.
This is component integration in an instrumented read-only Qt host, separate from
the normal installed app. See [bounded provenance](evidence/native-appearance-alpha-provenance.json).

Each of the nine palette roles is exactly `{r,g,b,a}`, with unsigned integer
channels0–65535. Opaque8-bit colors expand by257. There is one current wire;
schema1/2 production compatibility is not retained. The eight font tokens, six
spacing tokens, base size, owner/revision/BOOTTIME lease and publication fences
remain part of the complete typed snapshot.

Original popup surface and flat border Color consumers replace intrinsic alpha
with their companion alpha through `Qt.rgba`; they do not multiply the two
alphas. Half alpha is32768, matching Qt's normal RGB16 storage rather than an
8-bit approximation. Direct popup text uses QColor `#AARRGGBB`, preserving an
8-bit alpha expanded by257, and ignores its text-alpha companion. The qualified
`#804488cc` text therefore has alpha32896. This does not claim parity with the
separate Original Border renderer.

## Chronological integration

The qualified RED uses the preceding schema2 Rust/Qt selection and actual
Original Omarchy Color/Style singletons under the retained Arch ARM7.1.6 kernel.
Two opaque prerequisites, every opaque restore and final recovery pass. Three
mandatory cases then fail: companion half-alpha, intrinsic plus companion alpha,
and direct text alpha. The Rust publication becomes unavailable and the measured
plane differs by184338,240786 and13230 pixels respectively. The intrinsic-only
control and valid numeric-prefix/fallback controls pass. A malformed-quote case
is diagnostic only; no malformed upstream parser quirk is declared mandatory.
Root rehashes192 bindings and independently decodes all34 full captures before
the production repair.

A separate chronological real Qt peer/QObject/QML RED demonstrates that the old
reader rejects a valid current schema3 RGBA16 snapshot, after an accepted old-wire
baseline. It is qualified before changing the reader. The private reader is then
changed to exact four-channel validation and `QColor::fromRgba64`; all existing
credential, deadline, revision, transport and typed-style controls remain.

The repaired Original comparison uses the unchanged17 phases, scalar assertions,
comparison geometry and deadlines. All34 actual1080×2424 captures decode fully;
every measured592704-pixel plane matches byte-for-byte. All mandatory popup RGBA,
eight font tokens, six spacing tokens, scale and availability observations match.
The two stable observations per phase use the real Rust socket and current Qt
reader. Private UID1000 HOME remains0700 and inaccessible to UID11010.

The experiment actually waits294 direct guest children and observes665 native
Rust collector identities through pidfds; each observed collector is dead and
absent afterward. It does not claim direct waits for grandchildren. The component
host and broker exit0, the Docker CLI is waited, and exact-container absence and
input closure pass. Root independently rehashes223 bindings, decodes all34 whole
PNGs and recomputes every full-plane comparison. Captures and raw audits stay
private; this is not a normal-app UI preview or a Pixel hardware result.

## Separate regression boundaries

The same production Rust source passes23 actual host groups and22 separate Arch
ARM kernel groups:13 process groups,4 same-process real SEQPACKET controls,
4 captured-value scalar regressions and architecture-specific confinement.
The extra host group executes the x86 wrong-syscall-architecture refusal. The
ignored app helper is invoked by its owning process test and is not an extra pass.
Formatting, warnings-denied Clippy, selected builds and actual owned cleanup pass.

The first ARM aggregate remains21pass/1fail because a supplied test still expected
schema2. A fresh successor changes only that fixture assertion to3; the production
ELF is unchanged. Host prerequisite failures for missing Python and an ARM-only
ELF selector remain zero-test failures. Fresh host execution uses the actual
static-PIE x86 binaries with no interpreter or DT_NEEDED. Postrun audit assumption
corrections remain preserved separately from actual runtime results.

A fresh current-wire held-input regression also passes. The real collector sends
a null candidate after atomic file replacement; actual SCM credentials, pidfd,
ACK and parent wait4 are observed before the original collection deadline.
Publication withdraws, then fresh collection recovers at revision3. All46 frozen
inputs and exact owned-container closure pass. Only the tracer's current-wire
expectations change; its actual boundary predicates and deadlines are unchanged.

The private host Qt6.8.2 component suite passes505 actual cases: all154 preceding
controls plus288 channel refusals,54 role-shape refusals, one schema2 refusal
and8 precision/revision/liveness cases. All nine QColor/QML roles retain exact
16-bit values. Each malformed test begins with accepted current-wire state;
all505 test children and final peers are waited. This synthetic peer suite is
separate from the Original Qt6.12 ARM comparison, and the partial app consumer
remains private pending full feature preservation.

## Remaining migration work

Concrete font-family selection, private font resources, actual glyph resolution,
normal-window font/DPR observations and large-font layout remain open. Full
JavaScript Number/color language, arbitrary named colors/gradients, Border
rendering and Qt overflow behavior are not qualified by these selected cases.
The existing public app's full font/settings/theme path remains in place until
its native migration preserves those features.

External service supervision, genuine installed desktop launch, current Source/
lock/per-app display composition and the complete smartphone remain unfinished.
No appearance revision grants data permission. The Google-kernel RAM boot proof
does not qualify this appearance service on that kernel or a physical phone.

References: [Original Omarchy source](https://github.com/omacom/omarchy/tree/c668141e9c42b13c80c9ca4ea108e11708c5e8a5),
[Qt QColor](https://doc.qt.io/qt-6.12/qcolor.html),
[Qt.rgba](https://doc.qt.io/qt-6.12/qml-qtqml-qt.html#rgba-method).
