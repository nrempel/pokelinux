# Bounded endRender observation

A fresh native x86 replay now observes116.618ms between marks around
`createSyncFDManager` in one paired endRender invocation, inside a116.918ms Source
sample gap. The compositor dies before policy-service readiness; normal Sounds/PCM
remains unreached. The preceding13-case component qualification and separate
native source build remain distinct from this runtime failure.
[Hash-only evidence](evidence/native-endrender-observation-provenance.json) binds
each scope; earlier ARM and native composition failures remain failed.

The preceding [render-witness replay](native-session-x86-runtime.md#render-witness-replay)
retains a112.844ms separation between adjacent late-render landmarks inside a
113.029ms Source producer gap. It establishes no particular call or cause. The
new observation is limited to the pinned public
[endRender body](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/render/GLRenderer.cpp)
and normal [caller](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/render/Renderer.cpp).

## Component contract

Private numeric marks bracket draw submission, OpenGL finalization, the selected
sync branch, synchronous callbacks and existing cleanup. Original calls, branch
predicates, ordering and buffer handling remain intact; the cleanup guard stays
after draw submission and the deferred callback is unchanged. Each invocation
carries a local ID1..128 through nested calls and cleanup. Pre-return intent and
cleanup marks remain distinct from actual return, published only on the explicit
subsequent line in the selected caller after its token was consumed at GL entry.

The existing single event-loop writer retains the4096-byte,128-record committed
prefix, with phase0..23 and no wrap. Allocation saturation sets overflow without
committing a zero ID; notification before listener registration may not be
retained. IDs are not monitor/frame identities. Missing marks do not establish
blocked calls, completion, listener order or presentation; bracket time includes
observation cost. Source adapter/sampler,100ms lease, policy and gates are unchanged.

The frozen original-body baseline compiles: four controls pass and nine semantic
missing-phase assertions fail. Instrumentation alone makes all13 pass, including
nested/sequential IDs, throw restoration, early return, authored slow draw/GL/sync,
callback exceptions, saturation and malformed records. Controlled dependencies
and real tmpfs/mmap plus the actual selected parser qualify the component, not a
compositor, EventBus connection or GL driver. GREEN closes39 fixture and10 outer
waits/EOFs with unchanged inputs and exact container absence. Earlier component
RED and setup failures remain preserved.

## Native source build

A fresh offline x86 KVM guest builds the pinned source with GCC16.2.1, GNU C++26
and CMake Release/O3, retaining upstream default feature selection. Actual logs
include all three changed/new core CPP units. The449 compile command entries are
configuration metadata, not an executed-unit count. The new19,536,280-byte ELF is
machine62; its2,590,720-byte matching SDK has a verified499-file manifest. This is
a separate local build, not signed Arch package/recipe reproduction. No compositor,
Qt, Source or PCM executes in the build. The167.07s envelope closes30 named waits
and merged EOFs, normal QMP shutdown/EOF/QEMU0, CLI0, exact container absence on
the same daemon and unchanged selected inputs.

The generated500-byte `hyprland.pc` declares `prefix=/usr/include`, supplies
compiler include flags and defines no `includedir`. Immutable earlier prose
saying `/usr`, and the field named `actual_compilation_units`, remain preserved;
the raw metadata and build logs support the interpretations above. No artifact
was rewritten or rebuilt.

## Replay boundary

Source-only staging first fails on copied0444 metadata before any VM launch.
Repaired staging reaches a guest whose SDK probe incorrectly requires an
`includedir` value. Boot/bootstrap and owned shutdown/container/input closure
qualify separately, but setup fails before Qt/plugins/compositor/Source, app,
profile, capture or PCM. Neither setup failure is a behavioral RED. A probe-only
preparation checks actual prefix/Cflags/compiler dependencies. Its later replay
verifies the installed499-file SDK, compiler dependency path and all three plugin
builds. Current Qt32 checks (30 audio behaviors) and29 appearance cases pass.

The replay retains57 valid committed records with zero flags after actual
compositor death, before Stop. Source accepts sequence1 at54,251,746,635ns and
rejects sequence2 at54,368,664,312ns for `validation/sample_gap`:116,917,677ns
against the unchanged100ms lease. At rejection observation54,370,092,297ns the
successor is only1,427,985ns old; chain rejection precedes freshness evaluation.
The actor records failure3,396ns later. Paired invocation2 supplies:

| Marks in the same invocation | Elapsed ns |
|---|---:|
| Render-pass submission | 64,802 |
| OpenGL finalization | 8,756 |
| Before/after createSyncFDManager | 116,618,089 |

The fence marks are at54,251,928,740ns and54,368,546,829ns, both inside the
producer gap. The explicit-valid branch, cleanup and actual callsite-return marks
follow; this invocation takes that branch rather than implicit glFinish. Invocation1's
fence bracket is30,287ns. These are cooperative single-event-loop observations
including observation overhead, without frame identity, exclusive inner CPU time,
driver attribution or an explanation of the earlier signed Arch/ARM failures.
Independent decoding agrees with the complete retained production-parser prefix.

The full selection still fails at the live-compositor assertion during policy
service readiness, before normal Sounds, final Pixel profile, all four groups,
capture or PCM. QMP normal shutdown/EOF/QEMU0, CLI0, exact CID absence on the same
daemon and unchanged input closure qualify separately. All101 direct waits and108
PIPE EOFs meet their original cutoffs; the60.57s functional selection still fails.
The separate guest subreaper exit1 remains recorded. Further diagnosis must stay
within the measured fence-creation span and preserve rendering behavior.
The functional gate retains matching rebuilt plugins, the four fixed Sounds groups,
real focus and independent grants, Pixel10 native1080×2424/422PPI/420 logical
density, and all capture/graph/PCM guards. Current-owner Messages GUI/session,
ARM availability, matched Google-kernel integration, full Omarchy startup and
physical/carrier gates remain open. No physical phone has been accessed.
