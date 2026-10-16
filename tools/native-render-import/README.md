# Connected Rust SHM import prerequisite

`hyprland.patch` applies to upstream Hyprland
[`efb50993780079460b0cbed1363e2166a2de1d9f`](https://github.com/hyprwm/Hyprland/tree/efb50993780079460b0cbed1363e2166a2de1d9f).
`cohort.json` binds exact before/after bytes for all18 affected files. This combines
the six preserved original endRender observation files with13 SHM import files;
`GLRenderer.cpp` belongs to both. It does not require an unpublished source patch.
The source remains subject to upstream licensing.

The production path is `commitState` → `updateSynchronousTexture` → Rust capture
→ const texture input. Synchronous recapture now requires the incoming buffer-
update bit; retaining a current buffer alone does not trigger another import. The
C++ wrapper clones/moves/releases the Rust handles
when surface states copy or retire, and cursor CPU reads use the same retained
immutable payload. Client memory access ends before the existing GL call.
`native-render` is the original std-only Rust library and C ABI; no proprietary
input, decompiled code, runtime binary or private raw receipt is included.

Build the exact Rust archive/header first. Configure the patched compositor with
explicit absolute `POKELINUX_RENDER_STATIC_LIBRARY` and
`POKELINUX_RENDER_INCLUDE_DIR`, plus `POKELINUX_RENDER_NATIVE_LIBRARIES` from the
actual target Rust build. CMake performs no Cargo or network work. Rebuild the
matching SDK and every plugin: texture input signatures are const-correct and
the SDK must include the exact C header beside `SHMSnapshot.hpp`.

The initial full native build and five C++/Rust lifetime groups pass. A separate
[actual no-attach case](../../docs/native-render-import.md#actual-no-attach-identity-red)
qualifies RED: a retained synchronous buffer is recaptured and its import identity
changes despite unchanged bytes. The exact [boundary actors/oracle](noattach/README.md)
are supplied here. The public patch includes the one-file incoming-buffer guard
repair; its matching full native rebuild/five FFI groups and unchanged actual
no-attach oracle pass. Identity remains7→7 with the same bytes. This is not
complete production import acceptance. Existing `texture->ok()` checks object
validity rather than
proven GL upload success; actual upload failure/completion remains unqualified.
The existing pointer consumer also assumes tight AR24 cursor rows despite the
snapshot retaining actual stride. These limits must remain explicit until their
actual behavior is tested and repaired.

GL still executes on the original compositor owner. This is an immutable CPU
handoff prerequisite, not an executor, complete frame/output ownership split,
Source GREEN, cold composition pass or phone result. The unchanged
[rendering-stall baseline](../../docs/native-render-authority.md) remains RED.
