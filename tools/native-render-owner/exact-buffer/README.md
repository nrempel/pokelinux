# Exact native buffer operations

Apply `aquamarine-exact-buffer.patch` after the retained-child custody patch.
It follows Aquamarine's upstream license. The eight-file delta adds explicit
allocation/import operations and copied native results. It does not select a
format or grant display authority. Those decisions belong in Rust; the current
adapter migration remains unfinished.

The exact paths use the existing swapchain, GBM buffer and framebuffer holders.
An implicit request uses one `gbm_bo_create` and one `drmModeAddFB2` with flags0.
An explicit request uses the modifier entrypoints and requires actual capability1.
Neither path guesses from the returned modifier, substitutes a format, retries
or falls back. Failed acquisitions stay retained. Fresh exact buffers cannot
enter the legacy framebuffer constructor; cached reuse requires the same backend.

The five fixture files are unchanged frozen inputs. Their original source-only
wording records preparation time; [the report](../../../docs/native-render-exact-buffer.md)
records subsequent executions and their limits. The actual missing-interface
RED precedes the matching normal library/SDK build and two clean exact API checks.
The full kernel consumer is separate and has not yet executed.

Compile the kernel consumer only against a matching generated AQ15 SDK and its
library, the genuine current Rust headers and one matching Rust static archive.
Use GNU16/C++23, O2, Wall/Wextra/Werror/pedantic/pthread, actual dependency files
and link map, and the real libseat C-linkage header from the earlier initial-batch
fixture. Require loader relocation checks before running. A proposed header,
stand-in object, second Rust provider or old AQ SDK cannot qualify the boundary.

The consumer uses a genuine Rust-owned test bootstrap with a real seatd broker
and virtual DRM driver. It deliberately publishes no Ready and performs no
atomic commit. Its five operation cases do not prove production prepare/execute,
normal app frames, complete native retirement, Source, ARM or a physical phone.
The process parent must independently record wait and output EOF; `_Exit` alone
is no cleanup evidence. Preserve the original finite ownership envelopes.
