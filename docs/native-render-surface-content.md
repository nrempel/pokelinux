# Retained CPU surface content

Surface geometry now uses the immutable Rust SHM snapshot. A new
`SSurfaceState::hasContent() const noexcept` checks a non-rejected state and a
valid nonzero snapshot extent matching `bufferSize`; `sourceSize()` uses that
predicate before the existing viewport, transform and scale calculation.
The [two-file patch and exact tests](../tools/native-render-owner/content/README.md)
are public. This is a CPU component repair; the core attachment/upload path still
calls the existing GL renderer on control.

## Actual original-code RED and unchanged GREEN

The native x86 KVM consumer compiles the real `SurfaceState.cpp`, `Buffer.cpp`
and `SHMSnapshot.cpp`, four genuine logging/colour support files and one frozen
consumer, then links the sole Rust archive. Private headers come from the
matching535-regular/two-link SDK. The repaired run overlays only SurfaceState.hpp
and selects the candidate SurfaceState.cpp; six other production TUs, the
consumer, classifier, Rust archive, link libraries and original cutoffs remain
unchanged. Four support files retain their authentic production warning options;
the consumer and three target files keep strict Wall/Wextra/Werror flags.
Actual link maps discard the unused legacy GL upload; no texture stand-in,
renderer, native context or Wayland server is created.

In RED, a real5x3 ARGB memfd with32-byte stride is captured into Rust. All96 bytes,
including36 padding bytes, remain exact after the producer overwrites its mapping,
unmaps it and closes the actual descriptor. The real surface-state update retains
that snapshot, but sourceSize returns0x0 instead of5x3. The consumer exits1 after
these prerequisites; this is the qualified geometry failure.

After the two-file repair, the identical consumer exits0 with all five groups:

- Empty content has zero extent.
- Retained content has5x3 extent after producer disposal.
- A no-attach update and pending reset preserve exact identity and bytes.
- Rotation, scale and viewport-source precedence retain their actual geometry.
- Detach, fresh import and current reset preserve only the separately owned old
  snapshot clone; the fresh import has distinct monotonic identity and bytes.

Each functional epoch closes45 actual direct waits and52 PIPE EOFs within the
original bounds. The consumer has one5s absolute deadline. Same-peer QMP KVM,
normal guest shutdown, QEMU0, foreground CLI0, exact container absence on the same
daemon and unchanged locked inputs qualify separately. The producer mapping is
in the consumer process; no separate producer-child exit is claimed.
[Hash-only evidence](evidence/native-render-surface-content-provenance.json)
binds the separate RED/GREEN epochs and original closure.

Four preceding attempts remain setup failures: ambiguous test constructors,
missing genuine support definitions, then mismatched logger and colour warning
settings. The constructor fix selects explicit doubles at exactly five sites;
all five semantic groups are frozen before the qualified RED. No failed compile
or link is relabeled a behavioral failure. Production support source is unchanged.

## Separate declaration checks and remaining gates

The earlier strict syntax control passes; the frozen target exposes the missing
hasContent member and its dependent type error. After one candidate-header
overlay, both unchanged TUs pass with zero diagnostics. Each syntax epoch closes
30 waits/37 PIPE EOFs and selects the same96 private headers. Those checks do not
execute a method body and are separate from the functional CPU proof.

No fresh full main/SDK build, real Wayland configure/commit, CPU scene extraction,
worker output submission or presentation executes here. Normal CPU managers,
monitor/workspace state, immutable assets/cursor routing and core capture remain
on the path to the first genuine client frame. Native readiness, authenticated
StopRetire/drain, unchanged Source7, normal Sounds/PCM and Messages, current ARM,
matched Google kernel and all phone/carrier gates remain open. Zero phones qualify.
