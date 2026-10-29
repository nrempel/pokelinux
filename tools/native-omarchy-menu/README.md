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

Merge the three manifest rows and their files into a fresh complete selected
bundle. Preserve canonical file modes and independent deployment files; pass the
complete manifest to the unchanged generic deployment installer. This overlay
alone is not a system image or a preview installer.

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
