# Retained SHM surface content

`surface-state.patch` adds `SSurfaceState::hasContent() const noexcept` and makes `sourceSize()` use retained immutable SHM content. A valid snapshot must have nonzero dimensions matching `bufferSize`, and the state must be accepted. Existing viewport, transform and scale math remains. This changes exactly SurfaceState.cpp and SurfaceState.hpp; the attachment/upload path still calls the existing GL renderer.

Actual original-code RED precedes unchanged-consumer five-group GREEN. All96 Rust-owned bytes survive producer overwrite/unmap/close; the repaired surface reports5x3 and preserves identity, geometry and independent clones through the remaining groups. Candidate declaration-only syntax checks pass separately. See the [report](../../../docs/native-render-surface-content.md). No new full main/SDK build, normal Wayland frame, GL upload, worker presentation, Source, ARM or phone result is claimed. This SHM-only content predicate does not qualify other buffer forms.

`source-cohort.json` pins the public parent382d3ff506e97bda1124b83e80274cb965a02b66, exact two-file before/after bytes, tests and existing dependencies. Reconstruct the source from official Hyprland efb50993780079460b0cbed1363e2166a2de1d9f and the pinned udis86 subproject. Apply the existing public patches in this order:

1. `tools/native-render-import/hyprland.patch`
2. `tools/native-render-owner/hyprland-executor-aq15.patch`
3. `tools/native-render-owner/hyprland-startup-custody-test.patch`
4. This `surface-state.patch`

The first patch supplies both before-files; the next two leave them unchanged. The cohort records the checked reconstruction and all patch identities. The separate AQ15 public chain and its source exceptions remain as specified by the existing owner cohorts. No upstream or project source needs private planning files. This package contains authored sources, a patch and hashes; it includes no binaries or raw private run receipts.

The unchanged frozen files serve distinct checks:

- `cpu-surface-content.cpp` calls real SSurfaceState methods and CSHMSnapshot/Rust C APIs. Its five groups cover empty extent; exact96-byte padded5x3 ARGB custody after producer disposal; no-attach identity; viewport/rotation/scale; and detach/replacement/reset with an independent old clone. It creates no Wayland server or texture stand-in and never calls the new accessor directly.
- `existing-surface-state-header-control.cpp` and `retained-content-public-member.cpp` are the original syntax-only dependency/control and exact accessor-signature checks. They instantiate or execute nothing.
- `main_member_epoch.py` is the exact functional oracle selected before the actual original-code run. Both its original-failure and repaired-success branches execute in the separate qualified epochs. It requires real dependency/link proof, exact program output/exit and original owned wait/PIPE EOF records; setup failures cannot qualify the geometry failure.

For the CPU component, use the exact existing535-regular/two-link SDK9a31e800bd4b7bc1e3eccf7089cdbce181cfe3a57d4446eab8a43b240bde9893 and sole Rust archive246a6680b7f9ed8affd89bd9b022319a0903f5ebc58f03d7d76eaa85f8653311, with its matching three C headers. Overlay only the candidate SurfaceState.hpp in a fresh private SDK copy, recording the original manifest and that one new header identity. This overlay is a component compiler input, not a rebuilt compositor SDK. The published Rust source baseline is f3be35a74d48245d41638f87b78847421f6dd0c0; a newly built archive needs its own actual identity and qualification.

Use genuine original production code for the functional link: SurfaceState.cpp, render/SHMSnapshot.cpp and protocols/types/Buffer.cpp, plus debug/log/Logger.cpp, helpers/env/Env.cpp, helpers/Color.cpp and helpers/cm/ColorManagement.cpp. Paths and exact source pins are in the cohort; select the candidate SurfaceState.cpp for the repaired run. Copy the selected seven production TU bodies into a private flat compilation directory so quoted headers resolve through the selected SDK. Compile the consumer too: eight objects total.

Use GNU++26, `-O2 -Wall -Wextra -Werror -pthread -ffunction-sections -fdata-sections`. Only the four unchanged support TUs also use their recorded original production `-Wno-*` options, listed exactly in the cohort. The consumer and three target TUs keep the original strict flags without those options. Put private SDK `usr/include`, `usr/include/hyprland/protocols`, `usr/include/hyprland`, `usr/include/hyprland/src`, and its `protocols/types`, `render`, `debug/log`, `helpers/env`, `helpers`, `helpers/cm` directories on `-isystem` paths. Obtain genuine dependency cflags with `pkg-config --cflags hyprland hyprutils wayland-server pixman-1 libdrm`, selecting the private AQ15 pc directory first and private Hyprland pc directory next. Remap their absolute /usr/include/hyprland and /usr/include/aquamarine paths into that SDK and preserve other system dependencies. Retain actual dependency files and reject system Hyprland/AQ header mixing.

Link the eight objects with `-Wl,--gc-sections`, a retained link map, genuine `pkg-config --libs hyprutils wayland-server pixman-1 libdrm hyprgraphics`, the sole exact Rust archive, then `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`. Check actual `ldd -r` closure. The actual link map must show `updateSynchronousTexture` discarded; the executable must retain the real sourceSize/updateFrom and pr_snapshot_capture definitions. No fake globals, destructor replacements or unresolved-symbol bypasses are part of this recipe.

Syntax-only checks use the same private SDK/header overlay and original strict dependency setup with `-std=c++26 -fsyntax-only -Wall -Wextra -Werror`. Do not treat syntax results as body or buffer execution. A functional run keeps the existing UID1000 five-second program bound and owned process/outer closure requirements; select the expected epoch before execution and retain the exact frozen consumer and oracle.

Normal startup selection, coherent CPU monitor/workspace and protocol managers, real Wayland commit-to-snapshot extraction, immutable frame admission, exclusive native owner execution and genuine completion remain separate work. Existing Source timing/actors and cold/hardware gates remain unchanged.
