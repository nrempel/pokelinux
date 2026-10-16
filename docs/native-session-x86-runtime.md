# Native x86 session runtime

The native CPU execution environment on Harbour now passes one actual offline
KVM boot. The production Rust payload also compiles for x86_64 from the current
public source. A later native Qt build and current component checks also pass;
the full composition fails during original compositor startup before Sounds/PCM.
No Source availability or phone hardware qualification follows.
[Hash-only evidence](evidence/native-session-x86-runtime-provenance.json) binds the
inputs, builds, execution and qualifications. Private logs and VM artifacts stay
private.

## Runtime and authenticated guest

The existing graphics runtime is actually Debian13 amd64; its historical image
name does not establish an Arch or Hyprland installation. A derived image adds
matching-family QEMU x86, SeaBIOS, ipxe-qemu and genisoimage. Actual package
inventories and tool reports bind QEMU `1:10.0.13+ds-0+deb13u1`; the original
runtime and its renderer dependencies remain intact. An earlier Dockerfile
setup failure remains failed and is not a production RED.

The official `Arch-Linux-x86_64-cloudimg-20261015.611680.qcow2` is576,680,960 bytes.
Both its image signature and signed checksum were verified with the Arch CI key
published in the [pinned arch-boxes README](https://github.com/archlinux/arch-boxes/blob/7f733af26fe9a0c93fdabba13e7803cbe803374a/README.md).
The original is read-only, has no external backing/data file, and retains its2GiB
virtual disk. Only a fresh disposable child overlay receives writes. This is an
x86 Arch kernel, not Google's matched Pixel kernel.

## Actual boot and closure

A separate isolated API query first returns KVM API12. The actual VM then uses
explicit `-accel kvm -cpu host`, with no TCG fallback, no network and only the
host `/dev/kvm` device admitted to its container. Stock BIOS/GRUB, kernel,
initramfs and systemd discover a local NoCloud seed. No kernel extraction,
custom PID1, login/password or package installation runs.

The private bootstrap runtime-masks the offline time-wait service. This is an
explicit fixture change, not an unchanged boot-policy claim or installed phone
policy. The stock no-NIC network-online wait still consumes about120 seconds.
Actual cloud-init26.2 reaches the root bootstrap, which observes Linux
`7.2.9-arch1-1`, x86_64 and UID0. Its final receipt is flushed before it returns;
normal cloud-init power_state then powers off. The guest does not claim a
separately owned shutdown-child wait.

The same authenticated QMP socket identifies the owned QEMU process, confirms
KVM enabled/present, and later records `SHUTDOWN` with guest true and reason
`guest-shutdown`, followed by socket EOF and actual QEMU wait0. Serial identity
and the guest-result hash match the retained file. Seven host CLI waits and14
PIPE EOFs, four boot children, five guest commands and one early command all
close within their original cutoffs. The two CLI exit1 receipts establish exact
name/CID absence; they are expected. QEMU and Docker CLI return0, the same daemon
reports the exact container absent, and selected input hashes remain unchanged
under the shared lock. The complete host envelope takes132.46 seconds against
its240-second work/270-second closure limits; QEMU retains180/210 seconds.

## Matching native Rust payload

All83 selected production Cargo/source/lock files match public commit
`387fe72bc375def6dacd79fae8df2758f537e58a`; the four Sounds exec-fixture inputs
are separately bound. Strict formatting, selected-bin Clippy and native builds
pass for appearance, audio and sounds-exec. Packaging renames
`pokelinux-appearance` to `native-appearance` without changing bytes. The existing
host session owner and fixed Sounds client are separately hash-verified and
bound to that matching source. All five selected binaries are ELF machine62.
Ten CLI waits0 and20 PIPE EOFs close within their original bounds, with exact
container absence and no shared writable target/cache.

This compilation reruns no behavioral tests or Qt checks. `audio-client-test`
is a Qt/CMake target and was not treated as a Rust binary. The earlier32 Qt
checks are not current native-guest evidence. No production behavior changes,
new behavioral RED/GREEN result or screenshot follows from this checkpoint.

## Offline installed guest

A later fresh native KVM child now qualifies the offline installed software
cohort. Native pacman resolves against frozen repository databases;338 original
and29 additional archives have actual trusted detached-signature checks. The
lossless367-package cache preserves the original bundle. Package download/setup
failures remain preserved; they are not production behavioral REDs.

The first guest install misses `pw-cat`: Arch supplies it and `pw-record` in
[pipewire-audio](https://archlinux.org/packages/extra/x86_64/pipewire-audio/files/).
The corrected bundle adds29 missing archives for it and its dependencies. A second fresh
guest reaches all seven tool and four SPA-file hashes, but its version-only
Hyprland query aborts without XDG_RUNTIME_DIR. A private root0700 directory for
that same-child version exec corrects the fixture environment. Both failed
layers remain failed and are not reused.

The third fresh child grows the Btrfs root to8,273,244,160 bytes. Native pacman
projects the complete actual stock upgrade before mutation and finds no missing
archive; its strict signed offline transaction applies187 archives. The actual
installed inventory contains367 packages. Installed Hyprland and SDK bytes match
the signed archive, and the actual version query plus version.h bind commit
`efb50993780079460b0cbed1363e2166a2de1d9f`. QtGui6.12.0, Wayland1.26.0 and
libdrm2.4.134 are observed. This executes a version query, not a compositor session.

All18 guest commands, one early command, five boot children and seven host CLI
waits meet their original cutoffs, with38 actual PIPE EOFs. The same authenticated
QMP stream confirms KVM and normal guest shutdown, socket EOF and QEMU0; Docker
CLI0, exact container absence and750 external input hashes close. The envelope
takes36.11 seconds. Guest work/closure remains420/480 seconds, QEMU600/630 and
host660/690. These are setup limits, not changes to the100ms Source lease.

Only the fresh child receives disk writes. Its closed8GiB virtual QCOW2 is
hashed after QEMU wait0 and sealed0444; the authenticated original remains
read-only. The guest results export excludes the active disk and host selection.
[Hash-only provisioning evidence](evidence/native-session-x86-provision-provenance.json)
binds the archive/source/layer/SDK/inventory results and preserved failures. No Qt
build, component cases, normal app, final display or PCM runs in this checkpoint.

## Actual native Qt and composition attempt

One fresh native KVM child now builds and installs the current normal Qt app and
two test tools with `-Wall -Wextra -Werror`, using eight build jobs. All three
binaries are x86 ELF machine62. Its actual32 audio checks include setup/cleanup
and30 behaviors; all29 appearance cases also pass. These checks execute in this
epoch against Qt6.12.0, rather than inheriting the earlier ARM receipts. All17
selected Qt/desktop source files remain unchanged before/after. The component
scope includes offscreen appearance checks, without a normal app or display claim.

The matching SDK builds all three compositor plugins and the non-presenting
discovery helper. The Rust owner starts the exact installed Hyprland binary as
UID1000 with no supplementary groups. Its retained compositor pidfd becomes dead
during cold startup, before normal Sounds launch or final profile qualification.
The before-Stop authenticated root snapshot retains sequence1 sampled at
54,297,771,031ns and first provider `InvalidData` observed at54,409,790,650ns.
That112,019,619ns difference is neither a rejected-sample age nor a proven
producer gap. `InvalidData` covers receive EOF/truncation and multiple parsed
frame checks; no particular rejection or scheduling cause is known. The
cooperative startup witness retains13 committed landmarks with zero flags,
without proving callback return, in-flight work or an empty queue.

The functional selection stays failed, with zero boundary groups, PCM windows
or new captures. Its78 guest, five bootstrap, two early, five boot and seven
host direct waits meet their original cutoffs, with104 actual PIPE EOFs. The
finite tone child returns its declared-allowed1; the two host absence checks
return their expected1. Original Source/compositor child custody is not relabeled
as a numerical native-parent wait: the external pidfd is dead and the owner
itself waits0. The authenticated QMP stream confirms KVM, normal guest shutdown,
socket EOF and actual QEMU0; Docker CLI0 and exact CID absence on the same daemon
qualify separately. All78 selected source and27 external input hashes remain
unchanged under the shared lock. The complete envelope takes60.82 seconds.
The raw driver also labels the CID unresolved in its functional-assertion handler;
that field remains preserved alongside the independently qualified exact absence.

[Hash-only attempt evidence](evidence/native-session-x86-composition-provenance.json)
keeps the current Qt passes separate from full session failure. This changes no
production behavior and supplies no new behavioral RED/GREEN result.

## Typed rejection replay

A later fresh native KVM child selects the matching default-feature release
owner/client and retains the new diagnostic in a real authenticated root reply
before Stop. Source accepts sequence1 sampled at55,903,193,306ns, then rejects
sequence2 sampled at56,014,017,571ns for `validation/sample_gap`, at drain ordinal1.
The exact110,824,265ns producer sample gap exceeds the unchanged100ms limit by
10,824,265ns. At rejection observation56,019,594,004ns, the successor is only
5,576,433ns old. The receive contains216 bytes with both truncation flags false
and no syscall errno. The freshness check is not reached because chain validation
rejects first; the recorded time is that failure's observation, not an executed
freshness decision. The actor records its failure3,807ns later.

This identifies a producer sampling gap in this selection, without identifying
which compositor operation occupied it. All13 existing configuration witness
landmarks precede sequence1. The record supplies no operation duration, callback
completion, empty-queue proof or diagnosis of earlier ARM/native failures. The
two Rust roles also change from debug opt0 to release opt3; no timing causal
comparison is claimed.

The actual current Qt32 checks (30 behaviors) and29 appearance cases pass again,
and every plugin/discovery build passes. The full selection remains failed before
normal Sounds, final profile, all four functional groups, capture or PCM. All97
direct waits/104 PIPE EOFs meet their original cutoffs. Authenticated QMP KVM,
normal guest shutdown/socket EOF/QEMU0, CLI0/exact CID absence/same daemon and
78 selected/27 external input hashes qualify separately. The envelope takes
62.77 seconds. The runner's reporting repair leaves no contradictory unresolved
CID field when exact absence is already qualified; the original raw receipt
retains its old field unchanged.

The [same attempt evidence](evidence/native-session-x86-composition-provenance.json)
records this later selection separately from the first ambiguous `InvalidData`.
No physical phone was accessed.

## Render witness replay

One further full native KVM replay changes exactly three private fixture files:
the numeric witness header, its focus-plugin registrations and the guest decoder.
It adds render preChecks/pre and numeric stage0..9 to the existing4096-byte,
128-record committed prefix. Focus observation, Source adapter/sampler, Rust
policy,100ms lease, grants, all four app/graph/PCM groups and Pixel guards remain
unchanged. The authored compiled C++ writer and actual guest parser first produce
five passes/two missing-decoding REDs, then seven GREENs with unchanged tests.
Their13/15 actual writer waits and four separate ten-wait container epochs
qualify component encoding/bounds only. Earlier compiler, classifier and readonly
copy setup failures remain failed. No new production behavior is claimed.

The full original replay compiles the modified plugin against the matched SDK
and retains31 committed records after actual compositor death, with zero flags.
Source sequence1 is sampled at54,820,104,470ns; sequence2 at54,933,133,260ns is
rejected for validation/sample_gap. Their113,028,790ns gap exceeds100ms while
sequence2 is only4,119,874ns old at rejection observation. Freshness is not reached
because chain validation rejects first; the actor records failure3,466ns later.
Two adjacent late-render landmarks fall inside that producer gap:

| Observation | BOOTTIME ns |
|---|---:|
| Record30, LAST_MOMENT stage5 | 54,820,202,826 |
| Record31, POST stage6 | 54,933,047,007 |
| Separation | 112,844,181 |

The first landmark follows sequence1 by98,356ns; sequence2 follows the last by
86,253ns. These are cooperative landmark timestamps, without frame/monitor
identity, guaranteed pairing, callback-body duration, listener-order, blocked-call
or presentation proof. They narrow the next investigation without identifying a
specific operation. The original [renderer tail](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/render/Renderer.cpp#L2015)
contains endRender and later bookkeeping; output commit follows POST. The
[OpenGL endRender implementation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/render/GLRenderer.cpp#L79)
contains draw submission, OpenGL finalization and different synchronization paths.
The observations do not establish which path ran or justify changing it.

Current Qt32 checks (30 audio behaviors),29 appearance cases and all plugin/helper
builds pass again. Full composition still fails at the live-compositor assertion
during policy-service readiness, before normal Sounds, final profile, all four
functional groups, capture or PCM. All97 direct waits/104 PIPE EOFs meet their
original cutoffs. QMP KVM/normal shutdown/socket EOF/QEMU0, CLI0/exact CID absence,
same daemon and78 selected/27 external input hashes qualify separately. This
closed failed selection takes61.30s and does not reclassify earlier failures.
The [attempt evidence](evidence/native-session-x86-composition-provenance.json)
records this selection separately. No phone was accessed.

## Paired endRender build and replay setup

The [focused endRender witness](native-endrender-observation.md) now carries
bounded invocation IDs through draw/GL/sync and cleanup marks, with actual return
recorded only after the original endRender call returns. Four controls/nine compiled
missing-phase REDs precede13 component GREENs. A separate offline native source
build produces a new GNU C++26/O3 default-feature Hyprland ELF and matching499-file
SDK; it executes no compositor or Source. This is a new local build, not signed
Arch package reproduction. Exact build/artifact hashes and limits are in the
[new evidence](evidence/native-endrender-observation-provenance.json).

Source-only staging first fails on copied0444 metadata, before a VM starts.
Repaired staging reaches a fresh guest, whose SDK probe assumes an `includedir`
variable absent from the generated `hyprland.pc`. The actual file declares
`prefix=/usr/include`; immutable earlier prose saying `/usr` is not rewritten.
That guest closes normally but the selection remains a setup failure before
Qt/plugins/compositor/Source, app, final profile or PCM.

A later fresh replay corrects only that probe, qualifies the installed SDK and all
three plugin builds, and passes current Qt32/appearance29. Its57 valid records
with zero flags pair116,618,089ns around createSyncFDManager in invocation2, inside
a116,917,677ns Source sample gap. The explicit-valid branch and actual callsite
return follow; this is a measured call span including observation overhead,
without exclusive inner CPU/driver or earlier signed-Arch/ARM causal attribution.
The original live-compositor assertion still fails at policy readiness before
normal Sounds/final profile/groups/capture/PCM. Owned QMP shutdown/EOF/QEMU0,
CLI0/exact CID absence/same daemon and input closure qualify separately.
See the focused report for exact timestamps and scope.

## Inner fence component and second source build

A later [inner-fence checkpoint](native-endrender-observation.md#inner-fence-component)
adds paired numeric marks around eglCreateSyncKHR, required glFlush and native-FD
duplication, with one scoped current invocation on the existing event loop. The
new frozen-v2 baseline passes13 revised-domain prior controls plus three inner
controls and fails eight semantic assertions; instrumentation alone passes all24.
The earlier invalid raw-phase24 negative moves to30 before RED2. Earlier13
evidence, scaffold/source-lookup/candidate-generation setup notes and failed
runtimes remain separate. Controlled EGL calls and actual FD/mmap/parser checks
are component evidence, without an actual EGL context or driver conclusion.

Source build2 completes in160.383778791s with GNU C++26/O3 Release and unchanged
default features. Actual build lines include EndRenderWitness, GLRenderer,
Renderer and OpenGL;449 configured command entries are not an executed-unit count.
The new machine62 ELF and complete matching499-file SDK retain separate hashes.
Thirty named waits/merged PIPE EOFs and separate early-mask custody, normal QMP
shutdown/EOF/QEMU0, CLI0 and exact-container/input closure qualify the build only.
Generated pkg-config declares prefix=/usr/include and no includedir. No compositor,
Source, Qt or PCM executes in that build. The later fresh replay installs the exact
new ELF/499-file SDK and rebuilds all plugins, physical adapter and discovery.
Current Qt32/appearance29 pass. Its69 valid zero-flag records pair113,338,467ns
around eglCreateSyncKHR in invocation2, within a113,354,246ns outer fence bracket
and113,643,605ns Source gap. Required glFlush and FD duplication take4,439ns and
9,227ns; the2,113ns residual includes unmarked work and witness overhead.
The same live-compositor assertion still fails during policy readiness, before
Sounds/final profile/four groups/capture/PCM. Pre-Stop pidfd death and later actual
process disappearance remain distinct; native-parent numeric wait is not exported.
These observations identify no driver/thread/ioctl, exclusive CPU, frame or prior
Arch/ARM cause. Exact timestamps and separate custody remain in the focused report.

## First four-request compositor attempt

Root8 compiles all four installed-header requests and completes BTF setup using
the same local build2 ELF/SDK cohort. All three plugins and current Qt32/appearance29
pass. Policy readiness is reached, then actual discovery dies with SIGABRT6 against
its original allowed exit0. The original full selection fails before final Pixel
profile, normal Sounds, four groups, capture or PCM. Source retains sequence1 at
54,396,733,250ns and rejects cached_sample/sample_expired at102,579,357ns age.
Its57 zero-flag witness records end at invocation2 phase24 without25, supplying
no paired duration, in-flight or cause claim.

All106 physical direct waits/113 PIPE EOFs meet their original numeric cutoffs,
and watched descendants disappear, but the new diagnostic gate also requires functional
expected exits and skips finish after discovery6. No capture is collected: its0
pairs establish no timing or absence. Owned trace/probe cleanup and QMP shutdown/
EOF/QEMU0, CLI0, exact-container/input closure qualify separately in60.860535017s.
A source-only correction separates physical wait/EOF/proc/pidfd/ECHILD/cutoff
closure from the unchanged functional-exit contract. Root9 below separately
qualifies the corrected capture while preserving full functional failure.
See the [focused report](native-endrender-observation.md#first-four-request-compositor-attempt).

## Corrected four-request capture

Root9 fails at the live-actor/compositor assertion during policy readiness before
Sounds/PCM, but corrected physical closure and the original exit contract both
qualify bounded capture. Eleven selected pairs are7 EXECBUFFER/4 WAIT on main
TID966, FD34, device226:128, result0; no transfers occur in this selected window.
The19 fork records/15-TID lineage do not establish common TGID membership.
A successful EXECBUFFER duration is bounded by116,925,001–116,926,999ns wholly
inside invocation2's116,959,135ns EGL bracket. This is syscall-boundary localization,
without an avoidable-wait, exclusive-CPU, GPU-completion or earlier-failure cause.
Source rejects the117,268,140ns seq1→2 sample gap before freshness.
The four diagnostic commands, zero probe misses/loss and owned-resource removal
qualify separately. All105 actual direct waits/112 PIPE EOFs meet the original
numeric cutoffs in61.064927729s, with separate guest subreaper exit1 retained.
QMP same-peer guest shutdown/EOF/QEMU0, CLI0, exact same-daemon container absence
and unchanged locked inputs close separately. [Root and independent receipt hashes](evidence/native-endrender-observation-provenance.json)
retain these scopes separately from the failed full functional result.

## Next integration

The bounded Root9 capture separately qualifies selected syscall intervals after
the preserved Root8 collection gap. The capability probe and unchanged14-case
native EXECBUFFER/VERSION helper GREEN retain their narrower scopes. A bounded
source review demonstrates no concrete avoidable wait or incorrect operation and
selects no justified driver patch. That discrimination investigation is closed;
no more driver profiling is selected. Full original functional composition remains
failed, and the interval establishes no exclusive-CPU, completion or earlier-run
cause.

The next scope is the control/render ownership seam and an actual-compositor RED
specification, without an implementation or runtime qualification yet. Preserve genuine focus, lock and
client-lifetime facts plus ordering, fences, buffer lifetime, render-context,
command/completion and shutdown custody. Required integration must stall rendering
while real focus, client death and lock intent advance within the unchanged100ms
Source contract; authority stalls remain terminal. Cached heartbeats are refused.
Keep the Source adapter, predicate order, ErrorKinds, real grants and every
functional/Pixel/PCM guard unchanged. A future focused RED requires real Source
live, ready clients, an entered render stall beyond100ms and real operations
issued within it under a frozen identical selection. Cold failure before those
prerequisites is setup failure, not RED; the original cold full selection remains
separate acceptance.

The complete four-step target remains denied quiet launch, current grant and real
focused touch with PCM, null-focus background continuation with disabled touch,
and terminal Source loss without app/audio revival. Preserve per-app Wayland,
independent grants,1080×2424/422PPI/420 logical density, capture guards and every
graph/PCM assertion.

Native x86 success would qualify that environment only. The earlier ARM Source
expiry remains failed and does not establish a TCG cause. ARM availability,
matched Google kernel integration, complete Omarchy shell/startup and every
physical Pixel/carrier gate remain open. No physical phone has been accessed.
