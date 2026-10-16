# Connected Rust SHM import prerequisite

`hyprland.patch` applies to upstream Hyprland
[`efb50993780079460b0cbed1363e2166a2de1d9f`](https://github.com/hyprwm/Hyprland/tree/efb50993780079460b0cbed1363e2166a2de1d9f).
`cohort.json` binds exact before/after bytes for all31 affected files. It extends
the earlier21-file import/Cairo cohort with factored scene order, CPU frame
records and owner-only GL primitives. The original render observations, no-attach
guard and supplied-stride Cairo repair remain included.
It does not require an unpublished source patch.
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
actual target Rust build. The qualified31 ELF selects the snapshot-only archive
`69b81281…`, with C header `4a7a9a60…`, from the snapshot-only Rust source in
public checkpoint `79c80ba39dd7b12bee87384547291e222241cf73`. Its five source
hashes match the earlier library qualification.
The separately published eight-case permit module/archive `fe5aac70…` is not linked
into that ELF; a rebuild against current Rust is a new artifact selection.
CMake performs no Cargo or network work. Rebuild the
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
The [actual consumer actors and oracle](consumer/README.md) separately qualify
padded ARGB Cairo-consumer RED on a behavior-preserving native baseline: source
stride144 and exact cursor cache remain intact, while input stride128 paints992
wrong pixels out of1,024. That baseline extends the18-file import cohort to21
files with `PointerManager.cpp` and `CursorImage.cpp/.hpp`, and builds a matching
502-file SDK. A one-helper stride repair passes its matching full native rebuild,
five FFI groups and unchanged actual oracle: two live pre-client-exit Cairo paints
check1,024 pixels each with zero mismatches. Post-exit clones verify raw Rust
pixels/padding without invoking the helper. This is a CPU consumer result,
not a complete hardware/software cursor-output, upload or presentation result.

The [qualified31 checkpoint](../../docs/native-render-import.md#qualified-scene31-checkpoint)
passes a full native build, all15 affected C++ units and the matching505-file SDK.
An earlier29-path compiler failure remains a compile failure, not behavioral RED.
The new CPU producer and GL primitives compile; neither is called and no render
owner starts in the selected runtime. A fresh replay of the unchanged seven-case
Source oracle still has four passing controls and three rendering failures. The
older no-attach/Cairo oracles were not rerun on31.

GL still executes on the original compositor owner. This is an immutable CPU
handoff prerequisite, not an executor, complete frame/output ownership split,
Source GREEN, cold composition pass or phone result. The unchanged
[rendering-stall baseline](../../docs/native-render-authority.md) remains RED.
