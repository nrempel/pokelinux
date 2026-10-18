# Disabled connectivity development bundle

`development-arm64.manifest.json` authenticates the two normal AArch64 providers
from the [current ARM crossbuild](../../docs/native-settings-connectivity-window.md#arm64-compile-prerequisite):
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
