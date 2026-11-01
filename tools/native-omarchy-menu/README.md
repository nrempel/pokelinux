# Native Omarchy menu integration

`stage.py` consumes the hash-pinned public Omarchy archive and emits a fresh
three-file overlay plus its explicit manifest. It preserves the original wrapper
and default menu, and replaces only AppLibrary's launch function with the pinned
Quickshell desktop-entry API. The installed Rust client still requests an
actor-owned app launch. No services start during staging.

```sh
python3 -B tools/native-omarchy-menu/stage.py \
  --archive /absolute/path/to/omarchy-public.tar.gz \
  --root /absolute/path/to/fresh-overlay \
  --manifest /absolute/path/to/fresh-overlay.json
```

For graphical profiles, use `compose.py` in place of manually merging this
overlay. The base selects the original shell, staged theme and independent
deployment files. Both output paths must be fresh, with existing parent
directories, and separate from every input:

```sh
python3 -B tools/native-omarchy-menu/compose.py \
  --archive /absolute/path/to/omarchy-public.tar.gz \
  --base-root /absolute/path/to/graphical-base \
  --base-manifest /absolute/path/to/graphical-base.json \
  --menu-root /absolute/path/to/fresh-overlay \
  --menu-manifest /absolute/path/to/fresh-overlay.json \
  --replace-original-app-library \
  --root /absolute/path/to/fresh-composed-root \
  --manifest /absolute/path/to/fresh-composed.json
python3 -B native-session-owner/packaging/install_deployment.py \
  --root /absolute/path/to/fresh-composed-root \
  --manifest /absolute/path/to/fresh-composed.json \
  --destdir /absolute/path/to/build-only-destdir
```

The composer derives the exact three required byte/hash/mode rows from real
hash-pinned menu staging and compares them with the actual prepared overlay
bytes, including after a manifest change between reads. It accepts equal collisions and requires explicit
acknowledgment for the exact original-to-native AppLibrary replacement. Every
other differing collision, omitted component or unadapted menu component is
refused before output creation. It reuses the generic installer's file/path and
aggregate bounds. The generic installer remains unchanged and cannot identify
assets omitted from both a selected tree and its manifest. The composer checks
this menu component; other base selections remain the caller's responsibility.
Neither the overlay nor the composer is a system image or a preview installer.

`tests/composition.py` exercises actual theme, original-shell and native-menu
file staging with the pinned archive and a previously built normal provisioner.
The baseline manual merge and generic installer accept three invalid static
graphical selections. A deterministic manifest-substitution control also
reproduces an earlier composer's acceptance of an unadapted AppLibrary after
checking a different manifest. The current composer refuses all four invalid
selections before output creation. The test substitutes only its private copy,
triggers once at the actual preparation boundary, and restores the hook.
Valid composition
installs all 235 selected files, including equal wrapper/menu collisions and the
explicit AppLibrary replacement. These are host staging checks without service,
GUI, ARM or VM execution. [Hash-only results](../../docs/evidence/native-omarchy-graphical-composition.json).

```sh
# Expected exit1: three graphical admission assertions fail with the generic installer.
python3 -B tools/native-omarchy-menu/tests/composition.py \
  --archive /absolute/path/to/omarchy-public.tar.gz \
  --provisioner /absolute/path/to/omarchy-theme-provision \
  --output /absolute/path/to/fresh-baseline-check --baseline
# Expected exit0: all five cases and the explicit-replacement refusal pass.
python3 -B tools/native-omarchy-menu/tests/composition.py \
  --archive /absolute/path/to/omarchy-public.tar.gz \
  --provisioner /absolute/path/to/omarchy-theme-provision \
  --output /absolute/path/to/fresh-guarded-check
```

The two patches and `consumer-pins.json` reconstruct the exact executed x86 menu
consumers from the named published bases. Apply each zero-context patch with `git apply --unidiff-zero` to a private copy
of its base file, then verify the output hash. Both reconstructions were checked.
The guest adds real bar/Apps/Sounds input, authentic desktop selection checks and
the current audio continuation. It still requires the separately pinned runtime,
compiled normal services and unchanged fixture dependencies. These execution
snapshots do not supply a standalone interactive preview command.

[Results and limitations](../../docs/native-omarchy-menu-launch.md) distinguish the
passing menu/app boundary from the failed later audio continuation, and retain
the earlier actual menu RED. The native Sounds screenshot has measured window
and output geometry; menu-only captures do not inherit that window qualification.


## Standard close gesture

`window-bindings.lua` supplies the standard Omarchy Super+W close action through
the matched Hyprland Lua API. Append its bytes once to the selected compositor
configuration when assembling the preview bundle, then regenerate that manifest
row. The three-row menu overlay itself is unchanged.

`close-reopen-consumer-pins.json` and its two patches reconstruct the later
permission-withheld close/reopen consumers directly from the named public cold
bases, using `git apply --unidiff-zero`. They do not stack on the earlier menu
patches. [Actual results](../../docs/native-omarchy-app-lifecycle.md) distinguish
this complete user loop from audio playback and remote-viewer input.

The later [viewer sources](../native-omarchy-viewer/README.md) reconstruct the
separately qualified Unix RFB input continuation.
