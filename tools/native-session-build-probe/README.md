# Native session build probe

This authored fixture checks the exact installed ArchARM Hyprland package's
public C++ focus/lock APIs in an isolated ARM64 system-QEMU guest. It is a
FactsObserver build/API probe; its private focus-clear and force-unlock commands,
periodic file logger and synthetic locker are test infrastructure.

The selected result is
`artifacts/native-session-build-probe/run-4/result.json`: eleven groups passed.
See [the boundary report](../../docs/native-session-build-boundary.md) and
[provenance](../../docs/evidence/native-session-build-boundary-provenance.json)
for exact package, compiler, kernel, plugin and source hashes, historical setup
failures and unqualified production boundaries.

From the repository root, with the prepared `sim/rootfs.img`, `sim/boot/Image`,
`sim/boot/initramfs-linux.img` and local image
`pixel-graphics-sim:20261011-b472` available:

```sh
bash tools/native-session-build-probe/run.sh run-5
```

Choose a new `run-N` result name. The runner refuses to overwrite an existing
result and locks only `artifacts/native-session-build-probe/owned.lock`. Its first
execution clones the prepared base image into its own `vm/root.img` and replaces
that clone's boot entry. Later runs reuse this own clone with QEMU snapshot writes;
they never boot the shared Messages clone. Each run preserves five source recipes
in its `source/` directory and records their SHA256 hashes. Current recipes match
the selected run4 copies exactly; keep the recipes fixed for the duration of a
run because the clone's bootstrap invokes the workspace guest entry.

The guest records installed package/compiler identity, ELF notes/dynamic symbols
and all installed header hashes; it compiles the plugin using installed
`pkg-config --cflags hyprland`. It generates a native ext-session-lock client from
the installed Wayland protocol XML, launches original Hyprland and authored
foot/Quickshell/lock fixtures, executes the eleven assertions, unloads the plugin,
attempts bounded owned-process cleanup and powers off. Docker/QEMU limits bound
the whole run. Docker has no network and QEMU has no NIC; the guest checks that
only loopback exists. Results and raw original-source caches are ignored artifacts.

Successful artifact hashes qualify the recorded package and exercised APIs.
A different package/compiler/header or kernel selection requires new evidence.
In particular, the exact ARM PKGBUILD remains unresolved; upstream header matches
do not prove that distribution implementation files are unpatched. The plugin
has no numeric-PID fallback for failed `SO_PEERPIDFD` identity. No actor permission
or publication behavior, trusted-locker admission, secure output presentation,
forced PID reuse or physical-device behavior is qualified by this fixture.
