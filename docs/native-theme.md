# Native Omarchy appearance integration

## Current migration qualification

The [new Rust appearance owner](native-appearance-service.md) has separate host
process/kernel and original ARM palette-composition evidence. Its partial typed
Qt prototype remains private; this document's historical full theme path is
still the public application's implementation. The palette control does not
qualify complete typography/settings migration, desktop admission or Source/lock.

A later independent pinned-source review identifies distinctions requiring fresh
actual controls before the full port. Original Color popup alpha replaces
intrinsic alpha, whereas original Border rendering multiplies it; direct popup
text differs from surface-role projection. The current JavaScript must not be
used as the oracle for those cases. These are static contract differences, not
newly executed renderer REDs. User-shell/font updates do not share theme-set's
lock, so their consistency cannot be called an atomic theme transaction. A
resolved font family also does not establish UID11010 access to private font
assets: actual QFontInfo/metrics and separately owned resource access are needed.
Fonts, all eight/six tokens, settings deletion/override, alpha and worker
confinement remain acceptance gates. Never promote the nine-RGB prototype by
silently dropping these features.

## Historical full-theme implementation and evidence

This boundary follows Omarchy v4.0.4, commit `c668141e9c42b13c80c9ca4ea108e11708c5e8a5`, through its original generated current-theme files, live user shell settings and canonical fontconfig alias. The public archive and selected script/template digests are pinned in [the manifest](../native-theme/public-manifest.json). Upstream files stay in ignored storage; published code contains the own observer, renderer probe and runners.

The production Qt application uses `PhoneThemeFiles`, `ThemePalette.js`, `PhoneTheme.qml` and `PhoneView.qml`. Appearance is independent of Rust call/permission/service ownership. It carries no enrollment or action authority. The reader follows `~/.local/state/omarchy/current/theme/{colors,shell}.toml`, `~/.config/omarchy/shell.toml`, and `~/.config/fontconfig/fonts.conf`. Original `omarchy-theme-set` replaces the generated directory, `omarchy-display-text-size` writes the user base size, and `omarchy-font-set` updates fontconfig and restarts the shell. The native window itself should remain open.

The original shell defines a system-wide `monospace` family and per-token pixel sizes, rather than per-token families. A fresh fixed `fc-match` child resolves the concrete family after fontconfig changes, avoiding Qt retaining its previously cached alias. There is one child at a time, a deadline and output limit, latest-configuration checks before publication, and owned cleanup. Failed resolution retains the prior concrete family and reports unavailable resolution; it cannot change service state. The [worker correction](native-theme-worker.md) adds explicit fail-closed teardown and boot-time admission/completion qualification; its new evidence is separate from the historical theme proof.

The native view maps its title, body, call heading and number draft to original display-large/body/heading/subtitle/display tokens. Buttons use title; margins and gaps use original panel/control spacing tokens. User settings overlay the generated theme settings. Existing popup palette/role/alpha projection remains shared. This does not implement every Omarchy shell component, gradient or per-side border setting.

The dedicated suite uses a private copy of the existing Arch ARM64 simulation disk with its own guest entry. The shared image, guest scripts and VM runners remain untouched. It boots full system QEMU/TCG with a virtual GPU and container llvmpipe, starts actual Hyprland, the released Omarchy Quickshell shell and a separate native Qt window, then invokes the original theme/font/text-size commands. An own read-only plugin observes the running original Color/Style singletons. The renderer loads production theme/view components with an explicitly synthetic call, unresolved intent and draft; it performs no phone action. Renderer snapshots, actual font metrics, native window pixel grabs and compositor screenshots distinguish original source execution from rendering.

Fetch and verify the public source with `python3 tools/fetch-native-theme-source.py`. `python3 tools/prepare-native-theme-baseline.py` creates/verifies the read-only previous-reader snapshot from its pinned own Git revision and file hashes. `tools/native-theme-suite.sh prepare` creates the private clone. `tools/native-theme-suite.sh red`, `red-font` and `green` create fresh evidence directories, refuse to overwrite previous evidence, hold a dedicated lock and bound the owned no-network graphics container. `red-font` selects that read-only previous-reader snapshot; `green` selects production source. This changes the actual consumer implementation and leaves the original producer unchanged. The other labels select output locations. The suite expects the prepared graphics image and signed/provisioned Arch disk already used by the repository. The original token red was executed before the token mapping fix. The independent clean font red builds the verified previous reader with the current token mapping. Neither red bypasses original script execution or uses a model as the runtime endpoint. `tools/build-native-theme.sh` runs the separate full isolated Qt/QML regression with actual Rust daemon/CLI fixtures; that offscreen run provides no Omarchy/compositor evidence.

Runtime status and precise remaining gates are recorded in [the provenance report](evidence/native-theme-provenance.json). The ARM64 graphical probe loads production components, rather than an installed app package connected to an enrolled service. Installed package/desktop launcher plus enrolled Rust service in the same Omarchy session, plugin activation for a user installation, production compositor decorations, wallpaper transitions, arbitrary theme compatibility and physical phone rendering remain separate gates. The original supported `OMARCHY_THEME_SKIP_BACKGROUND=1` branch keeps real theme generation and shell IPC while omitting wallpaper transitions. No desktop command is stubbed. The original shell bar also shows missing icon glyphs with the selected stock fonts; complete Omarchy font/icon provisioning is not proved. The original browser post-hook reports a missing packaged browser-policy helper in this guest; browser policy application is not proved. PipeWire, network, UPower and portal service warnings also remain desktop provisioning gates, independent of the observed palette/font boundary. A virtual GPU screenshot does not prove mobile GPU, display, touch, suspend or hardware behavior. No Android or Java component is a production theme dependency.
