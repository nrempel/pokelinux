# Connectivity packaging

## Optional installed-unit layer

`installed-unit.manifest.json` authenticates only
`usr/lib/systemd/system/pokelinux-connectivity.service`, installed mode0644.
This is a packaging boundary, not a complete runnable deployment. The unit
executes the current normal Main directly with four absolute paths:

```text
/run/pokelinux-connectivity/control.sock
/etc/pokelinux/connectivity.json
/run/pokelinux-connectivity/admitted
/usr/libexec/pokelinux/pokelinux-connectivity-service
```

Main keeps its own fixed three-second startup cutoff. `Type=exec` reports exec
success; actual fresh Inspect observations must establish backend readiness.
The manager's five-second startup/Stop settings do not extend Main's cutoff.
`Restart=no` prevents automatic replacement. `KillMode=mixed` delivers normal
SIGTERM to Main before the manager's remaining-cgroup SIGKILL fallback; cgroup
closure alone does not establish native worker waits or radio cleanup.

The root-owned runtime directory is0700. `RuntimeDirectoryPreserve=yes` keeps the
admission marker after Stop for the rest of the boot, including failed startup.
No unit command deletes or resets it. The existing Supervisor refuses a
pre-existing marker even after a successful reap. A normal `/run` tmpfs is fresh
after kernel reboot; same-boot restart is not a recovery contract. Durable
session policy and unclean/pending seals remain in their separate persistent
directory and must never be reset by service installation or startup.

Prepare a selected bundle containing the exact unit under its manifest-relative
path, then use the unchanged build-only installer:

```sh
make -C native-session-owner install-native-services \
  NATIVE_SERVICES_ROOT=/absolute/selected-unit-bundle \
  NATIVE_SERVICES_MANIFEST=/absolute/repository/native-connectivity-control/packaging/installed-unit.manifest.json \
  DESTDIR=/absolute/private-stage
```

This stages no provider ELF, operational profile, policy, enablement link or
hwsim/AP helper, and activates nothing. A complete installed cohort still needs
one coherent authenticated manifest for qualified current Main/worker binaries,
a protected root0600 startup profile, exclusive namespace/interface provisioning,
selected session owner/socket bindings, permission consent and explicit startup
ordering. The historical provider manifest below is not paired with this unit;
its ELF pins are not evidence for the current four-path ABI. hwsim radios,
foreground hostapd and packet peers remain test-only provisioning.

The host packaging tests first fail three missing-unit/deployment-row assertions,
then pass the same three assertions after adding this layer. They check direct
four-path startup and lifecycle settings, authenticate the manifest and actually
run the existing Make/Python installer into a private temporary tree. Site
configuration, an authored unclean admission marker and policy fixture retain
bytes, inode, modes, ownership and modification metadata. The fixtures are inert
file data; no service, worker, namespace or policy authority is executed.
All eight existing installer filesystem tests also pass.
[Recorded evidence](../../docs/evidence/native-connectivity-unit-packaging.json).

```sh
python3 -B native-connectivity-control/tests/packaging.py
python3 -B native-session-owner/tests/install_deployment.py
```

These are host packaging results only. No systemd activation, same-boot restart,
ARM execution, boot, Source, graphical session, packet exchange or phone result
is supplied by this layer. The two-cold-boot installed composition remains open.

## Historical disabled ARM provider bundle

`development-arm64.manifest.json` authenticates the two normal AArch64 providers
from the [historical ARM crossbuild](../../docs/native-settings-connectivity-window.md#arm64-compile-prerequisite):
Main and its networking worker, each installed mode 0755. It pins that exact build;
a different build requires its own authenticated manifest.

Put the matching files under the manifest's relative paths in a selected bundle,
then reuse the existing installer into a private staging directory:

```sh
make -C native-session-owner install-native-services \
  NATIVE_SERVICES_ROOT=/absolute/selected-bundle \
  NATIVE_SERVICES_MANIFEST=/absolute/repository/native-connectivity-control/packaging/development-arm64.manifest.json \
  DESTDIR=/absolute/private-stage
```

The actual host consumer stages exactly two new regular files, 1,515,040 bytes,
with matching hashes and 0755/root ownership. An existing copy of the production
`session.example.json` retains its bytes, mode, ownership, inode and mtime;
connectivity stays Disabled. All eight existing real filesystem installer tests
pass. Three directly owned processes have actual waits/merged stdout EOFs before
the original cutoffs, with ECHILD and all handles closed. Nested Make/unittest
children have no separate custody instrumentation. The initial Git ownership
setup failure remains failed; its successor changes only the invocation-local
Git safe-directory selection.

[Hash-only staging provenance](../../docs/evidence/native-connectivity-disabled-deployment-provenance.json)
binds the manifest, actual consumer, output and closure. This is host file
staging with ARM binaries as data. Runtime activation still requires trusted,
exclusive network-namespace provisioning, explicit owner selection and startup
ordering; no unit or activation is supplied by this bundle. The next installed
runtime case must select these staged paths through the existing private-ingress
consumer and retain its packet/Stop oracle. The normal ARM window failure and
all phone gates remain separate.
