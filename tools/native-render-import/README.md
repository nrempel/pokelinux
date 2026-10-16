# Connected Rust SHM import prerequisite

`hyprland.patch` applies to upstream Hyprland
[`efb50993780079460b0cbed1363e2166a2de1d9f`](https://github.com/hyprwm/Hyprland/tree/efb50993780079460b0cbed1363e2166a2de1d9f).
`cohort.json` binds exact before/after bytes for all33 affected files. It extends
the earlier31-file scene prerequisite with the immutable frame codec and explicit
owned snapshot adoption. The implemented-only frame header replaces private
executor declarations; frame layout and behavior remain unchanged. The original render observations, no-attach
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

Build the exact current Rust archive and matching `pokelinux_render.h` and
`pokelinux_render_frame.h` first. Configure the patched compositor with explicit
absolute `POKELINUX_RENDER_STATIC_LIBRARY` and `POKELINUX_RENDER_INCLUDE_DIR`, plus
`POKELINUX_RENDER_NATIVE_LIBRARIES` from that actual target Rust build. CMake does
no Cargo/network work. The matching SDK must carry both exact C headers beside
`SHMSnapshot.hpp` and `FramePlanCodec.hpp`; rebuild every plugin against that SDK.
The current frame CPU selection builds archive `3cf0a142…`, with snapshot header
`4a7a9a60…` and public frame header `56f8ec91…`.

The historical qualified31 ELF selects snapshot-only Rust `69b81281…`; its Source7
replay is a separate retained result. A later historical FULL33 build uses the
private95339 frame/header selection and passes five FFI plus four codec CPU
groups. The new public-header successor has its own Rust CPU QA and now a separate
successful native full-build/507-file SDK plus the same five FFI/four codec CPU
groups. Its own qualified epoch supplies that evidence; prior artifacts remain
historical. See the [frame report](../../docs/native-render-frame.md).

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

`frame-codec-integration.cpp` supplies an authored CPU plan and calls the actual
linked frame capture/visitor/snapshot-clone/adoption functions. Its four groups
check value preservation, refusal, padded lifetime after input/frame replacement,
actual joined-thread adoption and consume-on-entry transfer refusal. It requires
the matched patched source, exact Rust archive/headers, native libraries and an
owned finite driver; this directory supplies no complete VM runner. No live scene
producer, GL primitive or production executor is executed by this actor.

The later [Rust worker CPU component](../../docs/native-render-worker.md) has a new
Rust-QA archive8067436b. This native Frame ELF/SDK selected3cf0a142 without that
worker; neither the C++ groups nor native artifacts qualify the new archive.
Its genuine native binding and a fresh matched runtime selection remain required.


## Separate current-executor/AQ15 compile checkpoint

This directory's patch and cohort remain the exact frozen Frame507 baseline.
They are historical evidence, not a runtime fallback. The separate
[Hyprland incremental patch](../native-render-owner/hyprland-executor-aq15.patch)
applies after this exact full33 patch, and the
[AQ15 source patch](../native-render-owner/aquamarine.patch) supplies its independent
pinned backend dependency. Their metadata bind all source hashes and the actual
matching main/SDK [CPU and loader build](../../docs/native-render-executor-gl.md#separate-aq15current-abi-build-and-startup-failure).
That build passes 16 affected CPPs, 530 regular SDK files/two links and all 23 Rust
exports; it reruns five Snapshot, four codec, lifecycle and five C ABI CPU groups. The subsequent
matched compositor aborts before Source/actor, with no proven exact abort cause.
The compile-only successor is deliberately not selected as this default import
patch. Real broker/render startup and the remaining Source/phone gates stay open.
