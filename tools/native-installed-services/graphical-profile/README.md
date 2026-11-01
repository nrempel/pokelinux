# Fixed generic ARM graphical profile

`../stage_graphical.py` composes the selected normal services and the explicit
VM-only hwsim/display/audio preset into two build-only layers. It consumes a
named artifact index, the pinned public Omarchy archive and a readable prepared
base containing the selected Hyprland. It performs no build, download, service
activation, permission grant, image boot or device operation.

```sh
python3 -B tools/native-installed-services/stage_graphical.py \
  --artifact-index /absolute/build/artifact-index.json \
  --artifact-root /absolute/build/exported \
  --base-root /absolute/build/prepared-base \
  --omarchy-archive /absolute/sources/omarchy-public.tar.gz \
  --output /absolute/fresh-profile
```

The index has `schema: 1`, `parameters: {"ssid": "explicit-open-test-ap"}`,
`base_providers.hyprland`, `abi` and exactly the seventeen `artifacts` roles
listed in `stage_graphical.py`. Each artifact entry contains a canonical relative
`path`, integer `bytes`, `sha256` and `provenance` with its exact `role` and a
nonempty `source_reference`. Source files may be sealed read-only; installed
modes come from the fixed recipe. The two plugin entries also carry the same
`abi` object as the index: `hyprland_sha256` and `sdk_sha256`.
The pair must equal the selected public `plugins/abi.json` build contract;
caller metadata cannot silently select a different SDK pair.

`base_providers.hyprland.path` is exactly `usr/bin/Hyprland`, with its bytes/hash
and declared provenance. The existing file must be ARM64/mode0755 and match the
index and plugin ABI declaration. The stager reads it without copying it.
Hyprland is larger than the generic installer's8MiB file bound; no bound is
increased and no large binary is split or passed through another copier.

Artifact provenance is **declared input**, recorded with the index hash. The
stager verifies delivered bytes, names and ABI declarations; it does not rebuild
ELFs or attest compiler receipts. Operators must obtain qualified build/source
records separately. Historical test artifacts do not establish that current
source reproduces their binary hashes. Output always records
`runtime_qualified: false` and `compiled_source_provenance_revalidated: false`.

Existing build definitions supply the normal artifacts:

| Crate or definition | Named outputs |
| --- | --- |
| `native-session-owner` | `session-owner`, `presentation-environment` |
| `native-lifecycle` | `sounds-launch`, `settings-launch`, `permissions-launch` |
| `native-connectivity-control` | `connectivity-main`, `connectivity-worker` |
| `native-audio` | `audio-owner`, `audio-backend-ready` (`backend-ready` Cargo target) |
| `native-appearance` | `appearance-owner` (`pokelinux-appearance` target), `theme-provisioner` |
| `native-{permissions,settings,audio}-ui/CMakeLists.txt` | `permissions-ui`, `settings-ui`, `sounds-ui` |
| `plugins/CMakeLists.txt` | `session-source`, `focus-observer` |
| `tools/native-phone-display-profile/raw-output-capture.cpp` | `native-capture` |

Use the existing locked offline Cargo targets and matching ARM Qt/Rust-client
archives. The plugin CMake definition is the existing selected two-module build,
using the production Source adapter and the authored test focus observer. Its
source paths now point into the public tree. It requires the selected native
ARM compositor/SDK environment; this patch runs no compiler. Host-exported
Hyprland imports can remain unresolved in standalone plugin inspection and must
be checked against the matching compositor's actual dynamic exports.

The source recipe reuses genuine Omarchy source, the explicit Qt namespace
adaptation, theme/menu stagers, close-binding composer and unchanged generic
installer. `public-files.json` fixes the existing public aliases. `templates.json`
fixes the reviewed authored configuration targets and modes. Normal Permissions
remains inside the existing actor; no daemon or grant is added. The AP SSID is
explicitly rendered into the open-AP test selection. Policy storage is never
initialized or reset by staging.

The keeper, ring preparation, focus observer, virtual audio and station/AP
selection are test infrastructure. Their source is authored project code, not
private state, audit output or vendor code. The helper sources preserve the
selected behavior; packet peers, owned-process helpers, audio units and seat
drop-ins reuse their existing public sources. The compositor config joins the
VM display fragment with maintained public presentation/window rules; the menu
composer adds the standard close fragment exactly once. The maintained Settings
rule is required even when a historical witness lacks it.

The result contains:

- `graphical/` and `graphical.json`:241 source/config rows,21,511,542 bytes in the
  retained-input control.
- `services/` and `services.json`:77 normal-service/test rows,32,208,651 bytes.
- `selected-profile.json`:319 accounted targets, including verified base Hyprland.
- `composition.json`: role/source/index provenance and the seven further guest
  generated/retained rows required by the existing image-preparation contract.

Install each layer with `native-session-owner/packaging/install_deployment.py`
and its own manifest into a build-only DESTDIR. Each layer preserves8MiB/file
and64MiB aggregate bounds. Do not pass the combined319-row manifest to that
installer: it includes the reused base compositor. The other seven rows,
package/tool readback, enablement, actual authority and two-boot acceptance
remain separate image/runtime operations; this is not a new runner.

The actual public-stage RED and retained-input composition GREEN are described
in [hash-only evidence](../../../docs/evidence/native-installed-graphical-profile.json).
The GREEN uses historical ELF inputs only:318 rows match image17 exactly, and
the standalone phone-window fragment intentionally receives the maintained
Settings fix. The generated normal config also contains that rule. Missing or
swapped roles, mismatched plugin or declared ABI and changed base identity refuse before
output creation.

```sh
python3 -B tools/native-installed-services/tests/graphical_profile.py \
  --reference-image /absolute/retained-image17-preparation \
  --archive /absolute/sources/omarchy-public.tar.gz \
  --sdk-sha256 ACTUAL_MATCHING_SDK_SHA256 \
  --output /absolute/fresh-host-check
```

## Complete the seven image rows

`../complete_graphical.py` adds a separate seven-row layer to an authenticated
319-row result. It checks six fixed ARM64 provider files from an explicit prepared
base: QuickShell, seatd and the four retained display helpers. It renders the
hwsim configuration from an explicitly pinned template, replacing only its 14
tool slots with actual canonical file hashes. The eight hwsim source/config rows
and both protected profiles must match the existing composition. No file search,
compiler, package operation, policy initialization or service activation occurs.

```sh
python3 -B tools/native-installed-services/complete_graphical.py \
  --profile /absolute/staged-profile \
  --base-root /absolute/prepared-base \
  --inputs /absolute/completion-inputs \
  --index /absolute/completion-inputs/completion-index.json \
  --index-sha256 REVIEWED_COMPLETION_INDEX_SHA256 \
  --output /absolute/fresh-completion
```

The schema1 index has exactly `profile`, `providers`, `hwsim_template` and `tools`
in addition to `schema`. `profile` pins the four existing JSON files:
`selected-profile.json`, `composition.json`, `graphical.json` and `services.json`.
Each uses integer `bytes` and lowercase `sha256`. `providers` maps exactly the six
paths above to byte/hash/mode0755 plus declared `provenance.source_reference`.
`hwsim_template` has a canonical relative `path` under `--inputs` and its byte/hash.
The template retains the installed owner's schema, 660/700-second bounds and
kernel/module/regdb/closure metadata; those values are authenticated caller
inputs, not observations or live-image attestations by this tool.

`tools` has the exact 14 paths selected by the fixture. Each entry contains
`bytes`, `sha256`, `mode:493`, an absolute guest `resolved_path`, and `links`.
For example, a selected `usr/bin/python3` alias can declare
`"resolved_path":"/usr/bin/python3.14","links":{"usr/bin/python3":"python3.14"}`
plus the canonical file's pin. Regular tools use `links:{}`. Every declared link
must match actual prepared-base metadata; undeclared, cyclic or host-escaping
chains refuse. Absolute symlinks are interpreted inside the prepared base, never
as host file paths. Busctl and the six providers require ARM64 headers; no ELF is
executed and no loader or compiler equivalence is inferred.

The output has `completion/`, `completion.json`, `completed-profile.json` and
`completion-proof.json`. The original 319 rows are accounted unchanged; all 326
rows have a single source. Install only `completion/` with `completion.json`
through the unchanged generic installer. Its 8MiB/file and 64MiB/layer bounds stay
in effect. Do not pass the combined 326 rows to that installer. As with staging,
I/O failure after creation may leave a partial output; a new attempt needs a fresh
path. The inputs are unchanged, and output contains no policy or grant files.

The real filesystem control uses authored unexecuted ELF headers and kernel
metadata. It first observes seven missing rows in the existing bounded layers,
then requires exact completion/readback, unchanged319 rows and strict refusal
controls. This qualifies composition and copying only. It does not qualify real
provider compilation, packages, hwsim, services, UI or boot.

```sh
python3 -B tools/native-installed-services/tests/graphical_completion.py \
  --output /absolute/fresh-local-control
```

A separate file-copy control also passes with seven exact retained image21 ELF
files: the six completion providers and the base compositor. Their hashes and
target modes match the closed image manifest, and input files remain unchanged.
The remaining profile, tools and kernel metadata are authored test inputs. This
adds retained-file evidence, not compilation, loader, service or boot evidence.
