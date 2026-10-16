# Native compositor owned-startup checkpoint

The public compositor startup-control member now has a qualified **structural
compilation RED**. Its implementation and the actual broker → Rust worker → DRM
startup remain unqualified. This does not add a bootable smartphone or a Source
GREEN. [Hash-only evidence](evidence/native-render-owned-startup-provenance.json)
records the actual selection and closure.

The two frozen [existing-header control](../tools/native-render-owner/main-startup-existing-header-control.cpp)
and [target](../tools/native-render-owner/main-startup-public-member.cpp) include
the real compositor, shared command definition and HyprCtl headers. Both retain
existing `initServer`, callback and command-registration type checks. Only the
target adds the exact public nonconst signature
`std::string CCompositor::ownedBackendStartupReply(std::string_view)`.

Actual native x86 KVM GCC16 C++26 `-fsyntax-only -Wall -Wextra -Werror` compilation
passes the control with zero diagnostic bytes. The target exits 1 with exactly
two errors on its new assertion: the missing member and its dependent invalid
template argument. The [executed classifier](../tools/native-render-owner/main-startup-compile-oracle.py)
rejects unrelated diagnostics. Both real dependency sets select the same 86
private SDK headers among 559 resolved paths. The 530-file/two-link SDK remains
the earlier matching AQ15/main build; these checks instantiate no compositor,
link no Rust archive and execute no method body or native graph.

All 30 direct child waits and 37 PIPE EOFs meet the original finite cutoffs.
Actual KVM/QMP normal guest shutdown, same-socket EOF, QEMU0, foreground CLI0,
exact container absence and unchanged locked inputs qualify separately. An
independent source/receipt review agrees. Outer success acknowledges expected
structural RED; it is not implementation success.

A separate native x86 KVM selection also qualifies structural RED for the two
missing retained-initialization APIs. Its [existing-header control](../tools/native-render-owner/aq-retained-initialization-header-control.cpp)
compiles cleanly; the [frozen target](../tools/native-render-owner/aq-retained-initialization-members.cpp)
exits 1 with exactly four errors: missing `createRenderSessionHolder` and
`initializeRenderSession`, each with its same-line invalid-template cascade.
The [executed classifier](../tools/native-render-owner/aq-retained-initialization-compile-oracle.py)
requires both primary errors. Both real dependency files select the same 8
private SDK headers among 335 paths. This epoch independently qualifies 30 direct
waits/37 PIPE EOFs and the same original closure requirements. It also invokes
no native body and establishes no failure-custody behavior.

Source review identifies a necessary native ownership repair before runtime
admission. Existing `createForRenderSession` acquires DRM objects before returning
the outer backend. A failed DRM initialization can destroy a local partial child;
a mandatory `start()` failure clears implementations. Retaining only the returned
backend cannot preserve custody across those paths. A deny-only display scope
prevents authorization of guarded display writes; it does not prove native graph
retirement or prevent destructors from running.

The required successor creates an empty render-backend holder, retains it on the
Rust-owned worker, then initializes native resources. Each empty DRM child must be
retained before registration/resource setup and remain retained on failure.
Initialization failure and start failure remain distinct from readiness. The
missing-API structural RED is retained. Actual post-acquisition/mandatory-start
failure custody tests have not executed at this checkpoint.
Subordinate factories also need their own acquisition/cleanup evidence.

The [next runtime oracle](../tools/native-render-owner/main-startup-runtime-oracle.py)
is frozen **source only**, strengthened after architectural review. It requires
authentic real-broker capture before marks, both borrowed Rust watches, real
worker-local mirror/backend/output discovery and copied control acceptance. Its
held-startup case requires release and actual callback return before the original
startup deadline, followed by a public stopping/nonready query and retained
custody observation. A clean `Pending` callback must report status 0/retired 0;
error quarantine must never be relabeled as that clean branch. It also requires
the empty backend holder to be retained before actual native initialization.
Early control uses exact raw UTF8 requests delimited by client `SHUT_WR`/server
EOF, with nonblocking listener/accept/read/write, at most 16 clients, 64 request
bytes and 2,048 response bytes, and an absolute 2s BOOTTIME cutoff from actual
acceptance checked before dispatch and every send. Missing records
never default to success. No main-runtime execution qualifies this oracle yet.

The early main control route must reject flags, batches and unsafe commands
before normal dispatch and postprocessing. Socket reads/writes also need bounded
nonblocking ownership; the inherited blocking transport cannot qualify responsive
startup. Every unsupported post-prefix session event, including DEVICE_ADD and
SHUTDOWN, must revoke/Stop without acknowledging retirement or discarding custody.

The CPU monitor/renderer continuation, actual immutable frame/output route,
authentic native StopRetire and aggregate drain, current GL/Stop/finalize,
unchanged seven-case Source replay, normal cold app/audio/PCM, ARM, matched Google
kernel and all physical-phone/carrier gates remain open. Zero phones qualify.

## Compiled deferred outer holder

The [incremental source patch](../tools/native-render-owner/aquamarine-retained-initialization.patch)
now replaces `createForRenderSession` with the empty CPU holder factory
`createRenderSessionHolder(options, mirror)` and an explicit one-attempt
`initializeRenderSession()`. The worker caller retains the returned holder
before initialization. The holder validates a render-local mirror and selects
one mandatory DRM route; there is no old factory alias. Start also refuses a
second attempt on this owned route.

After the actual missing-API RED, a separate native x86 KVM build compiles the
source and links the matching AQ15 library/SDK. Both unchanged frozen AQ TUs
compile against that generated SDK with strict zero-diagnostic syntax success.
All ten unchanged CPU groups also pass, covering their existing gate/mirror
boundaries. Those groups never invoke the new native factory/initializer; this
is build and public-interface GREEN only. It does not establish native acquisition
or failure-custody behavior. No Rust executor, actual broker, compositor or native
failure hook executes in this build. All six required affected CPP object
records are retained. Each frozen TU selects the same eight generated private
headers among 335 resolved paths. Independent qualification verifies all 53 direct
waits and 53 merged PIPE EOFs within the original bounds, normal QMP shutdown,
QEMU/CLI0, exact container absence and unchanged locked inputs. The published
[cohort](../tools/native-render-owner/aquamarine-retained-initialization-cohort.json)
reconstructs every compiled/header byte from the public patch chain. No library,
SDK binary, raw log or private planning note is published.

The outer-holder change leaves partial DRM child, mandatory-start cleanup and
subordinate factory behavior as the next actual behavioral RED. The forced
post-acquisition and mandatory-start cases must observe genuine weak identities
and destructor entry, without fixture strong references that hide early teardown.
Retain the original failure paths until those failures are observed, then repair
child custody. Normal rendering and every hardware gate above remain open.

## Compiled test-hook prerequisite

A separate [sealed test-hook patch](../tools/native-render-owner/aquamarine-startup-custody-test-hooks.patch)
applies after the deferred-holder patch. The [matching cohort](../tools/native-render-owner/aquamarine-startup-custody-test-hooks-cohort.json)
pins the full public patch order and requires
`-DPOKELINUX_STARTUP_CUSTODY_TEST` for the dedicated build. This does not replace
the default production library or publish an unqualified main implementation.

Actual native x86 KVM compilation records all seven required C++ objects and the
exact test definition on all 36 compiler entries. Its matching SDK contains 23
regular files and two links, including the new test header. The library defines
nine required interfaces as 11 global function symbols, including constructor
and destructor variants. The [address probe](../tools/native-render-owner/aq-startup-custody-hook-link-probe.cpp)
links with strict warnings and `-z defs`; actual relocation checks resolve the
matching AQ15 library without missing symbols or AQ14. No probe function or hook
callback is called. The ten unchanged CPU groups and both unchanged frozen API
syntax contracts also pass.

Independent qualification verifies 57 direct waits/57 merged PIPE EOFs within
the original bounds, normal QMP shutdown, QEMU/CLI0, exact container absence and
unchanged locked inputs. In-memory public reconstruction checks all 33 authored
compiled C++ inputs and 21 SDK headers. Three generated protocol C++ entries and
the two inherited non-build documentation exceptions remain distinct.

This is a compiled test prerequisite. No actual post-acquisition or mandatory-start
failure injection, native custody RED/GREEN, main or Rust native worker executes
with this cohort. The child/start/subordinate-factory repair and every remaining
runtime, Source and phone gate above stay open.
