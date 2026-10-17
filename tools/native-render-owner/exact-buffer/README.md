# Exact native buffer operations

Apply `aquamarine-exact-buffer.patch` after the retained-child custody patch.
It follows Aquamarine's upstream license. The eight-file delta adds explicit
allocation/import operations and copied native results. It does not select a
format or grant display authority. Portable policy belongs in Rust; the current
native adapter migration remains unfinished.

The exact paths use the existing swapchain, GBM buffer and framebuffer holders.
An implicit request uses one `gbm_bo_create` and one `drmModeAddFB2` with flags0.
An explicit request uses the modifier entrypoints and requires actual capability1.
Neither path guesses from the returned modifier, substitutes a format, retries
or falls back. Failed acquisitions stay retained. Fresh exact buffers cannot
enter the legacy framebuffer constructor; cached reuse requires the same backend.

The original frozen contract, classifier and two structural TUs remain unchanged.
The kernel consumer has two narrow fixture corrections: a typed metadata copier
with compile-time checks, and failure-only observations at the unchanged control
predicate. Every operation assertion remains unchanged. Historical source-only
wording records preparation time; [the report](../../../docs/native-render-exact-buffer.md)
records actual execution and preserves both failed selections.

The matching normal library/SDK build, ten old CPU groups and four API checks
pass after missing-interface RED. Two real-driver runs observe all five resource
groups passing, but both aggregates fail the control-session check. The diagnostic
identifies revocation with a queued CHANGE at sequence4, no transport failure.
The control record precedes the resource-operation rows. These observations do
not qualify an active session, nativeReady, presentation or complete retirement.

Compile only against the matching generated AQ15 SDK/library, genuine current
Rust headers and one matching Rust archive. Use GNU16/C++23, strict warnings,
actual dependency/link maps and the real libseat C-linkage header from the earlier
initial-batch fixture. Require loader relocation checks. No proposed header,
stand-in object, second Rust provider or old AQ SDK can qualify the boundary.

The consumer runs in a genuine Rust-owned TEST BOOTSTRAP with real seatd and a
virtual DRM driver. It deliberately publishes no Ready and performs no atomic
commit. The parent independently waits for process exit and output EOF; process
disposal does not prove native cleanup. Keep original finite ownership envelopes.
Connect the production Rust CHANGE/selection/prepare path next; do not ignore the
real event or turn the partial operation observations into a passing full test.
