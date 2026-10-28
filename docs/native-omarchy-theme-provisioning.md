# Genuine initial theme provisioning

The normal graphical deployment needs actual Omarchy user theme state before
starting the compositor, appearance collector and full shell. Shipping only
`shell.json` leaves a fresh desktop home without the mandatory
`.local/state/omarchy/current/theme/colors.toml`. The normal appearance service
then truthfully publishes `available=false`.

`native-appearance/src/bin/omarchy-theme-provision.rs` supplies that deployment
prerequisite. It validates the root-owned configuration and source commands,
clears supplementary groups, drops to UID/GID1000, and handles user state only
after that drop. A complete selected theme must have a safe, bounded selection
marker and mandatory colors which project successfully with the same appearance
projection used by the collector. Existing generated/user shell overrides are
validated when present. A valid selection is left untouched; a partial or corrupt
selection fails instead of being overwritten. A home with no selection invokes
the original `omarchy-theme-set "Tokyo Night"` in its headless provisioning mode.
The helper execs the original setter; the selected PID1 unit owns that same
process and its descendants through completion or Stop. No palette, authentication
authority, live shell IPC, manual Source warmup or background retry is authored here.

## Normal staging

`tools/native-omarchy-theme/stage.py` is reusable normal deployment tooling. It
verifies the complete pinned public archive and selected source hashes before
staging forty genuine assets: the MIT license, three original theme commands,
all built-in theme templates and the complete Tokyo Night theme, including its
backgrounds. The original `install/user/theme.sh` is pinned as the default-policy
reference; its unrelated Pi/btop setup is not executed. The setter, template
renderer and color command are copied without modifications. The base image must
supply Bash, GNU coreutils, findutils, awk, grep, sed and util-linux `flock`.

The source is Omarchy commit `c668141e9c42b13c80c9ca4ea108e11708c5e8a5`;
the public archive SHA256 is
`8cc6b1d9d903c606600395e3b3d80ad9f15bd3ab78f0f19b0f49041b1df82013`.
`assets.json` records every selected file's size, mode and SHA256. The largest
asset is 4,445,106 bytes; the complete genuine selection is 19,818,545 bytes, within
the unchanged generic installer's 8 MiB per-file and 64 MiB bundle limits.

Build the normal native Rust binary with the retained target toolchain and export
an unlinked copy of the Cargo output before staging. Cargo may hard-link its
`target/release` binary; the generic installer's existing single-link input rule
still applies. For example, from the repository root, with explicit absolute
archive/output paths:

```sh
cargo build --offline --release --manifest-path native-appearance/Cargo.toml --bin omarchy-theme-provision
install -m0755 native-appearance/target/release/omarchy-theme-provision /absolute/output/omarchy-theme-provision
python3 tools/native-omarchy-theme/stage.py --archive /absolute/public/omarchy-public.tar.gz --provisioner /absolute/output/omarchy-theme-provision --root /absolute/normal-bundle --manifest /absolute/theme-manifest.json
```

Merge the emitted manifest rows with the normal graphical profile's selected
rows, then use `native-session-owner/packaging/install_deployment.py` for the
build-only DESTDIR copy. The stager does not initialize a host home, activate a
unit, fetch sources, patch Qt namespaces or change the shell environment.

The forty genuine files plus the native helper, oneshot unit and four normal
profile dependency drop-ins make forty-six rows. The oneshot uses
`/etc/pokelinux/appearance.json` and `/usr/share/pokelinux/omarchy`, with a 30 second
Start limit, 5 second Stop limit, control-group cleanup and `Restart=no`.
Both the normal target and the appearance, session-owner and shell services have
`Requires=` plus `After=` on provisioning. This makes provisioning failure a
startup dependency failure for each selected consumer. These drop-ins belong to
the normal graphical profile; standalone owner packaging gains no theme dependency.

## Actual boundary qualification

The frozen consumer is `tools/native-omarchy-theme/tests/fresh_home.py`, SHA256
`e929f5a940710ff6c76a384829efe0df1ae1dcda2dd05c5754bf2d0997fb0c3f`.
It uses an actual fresh UID/GID1000 home, the normal appearance daemon, real
credential-checked SEQPACKET replies, an actual completed collection and typed
Stop followed by normal process wait/EOF closure. Its expected publication and
genuine Tokyo Night background assertions are unchanged between RED and GREEN.
Only the explicit normal provisioning artifact/asset selection differs.

- `/tmp/pokelinux-default-theme-red-sol-1` is the closed actual missing-state RED:
  collector completed 1, schema3 `available=false`, null appearance, desired
  assertion failure/consumer exit 1, typed Stop and normal daemon exit 0.
- `/tmp/pokelinux-default-theme-green-sol-1` remains a failed aggregate: the first
  implementation named the pinned rustix `umask` API in the wrong module.
- `/tmp/pokelinux-default-theme-green-sol-2` remains a failed aggregate after its
  successful formatting, Clippy, four library tests and normal Release build:
  staging correctly refused the hard-linked Cargo ELF. No runtime GREEN is
  claimed for this epoch.
- `/tmp/pokelinux-default-theme-green-sol-3` is the closed actual GREEN, reusing
  SOL2's exported single-link normal binary: collector completed 1, revision 1,
  schema3 `available=true`, real Tokyo Night background RGBA
  `(6682,6939,9766,65535)`, typed Stop and normal daemon exit 0. The same daemon ELF
  and frozen consumer bytes were used for RED and GREEN.

The normal x86_64 provisioner is 730,144 bytes, machine 62, SHA256
`3ef3ba26bc3a8b69ef4a1401eda9385b70c0e8addab5691ad366a734a49a4de6`.
It has the normal Release/default-feature profile. SOL3's additional actual-helper
controls preserve a complete user selection's bytes, inodes, modes and mtimes,
and refuse a nonempty marker with missing mandatory colors without modifying
remaining files. They also validate the exact staged normal dependency and finite
unit declarations. Fixture DAC override permits the root test driver to inspect
UID1000 state; it is not part of the production oneshot's capability set.

Each epoch retains immutable source/consumer pins, actual direct-parent process
waits, stdout EOF, original deadlines and same-daemon exact-container retirement.
These are host process/collector results. Actual PID1 failed-provision dependency
propagation, the combined normal full-shell cold startup, Qt namespace behavior,
Pixel10 app QWindow/capture qualification and a usable full preview remain
unqualified by this boundary test. No graphical VM or physical device ran in
this theme cohort.

[Hash-only source and execution evidence](evidence/native-omarchy-theme-provisioning.json)
keeps the successful build, failed aggregates and runtime result distinct.

The frozen process consumers import the repository's
`tools/native-installed-services/owned.py` (SHA256
`5a738b9529a1a0e5d5b0bd20e27e2ec4244d4e61d7c8e0dad96fe1ffbef3c423`).
Place that directory on `PYTHONPATH` when running the consumers in the isolated
root-capable test container. They create their own UID1000 home under `/run` and
require the staged assets, exported helper and normal appearance daemon; they
are not commands to run against a user's desktop home.

Subsequent integration: the [combined cold x86 Omarchy/Sounds checkpoint](native-cold-omarchy-sounds.md)
now passes with this helper, real Source, the actual native window, qualified
Pixel10-profile capture and normal shutdown. It is one permission-withheld case;
authorized flows, failed-provision PID1 propagation and phone hardware remain open.
