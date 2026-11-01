# Normal theme startup

The normal theme provisioner uses the existing fixed desktop UID/GID 1000. The
selected configuration names its home and `/run/user/1000`; the provisioner drops
to 1000 and requires both directories to exist with that ownership and safe modes
before preserving or creating a theme selection. The normal session example and
appearance collector use the same desktop identity.

`pokelinux-omarchy-theme.service` requires and starts after
`user-runtime-dir@1000.service`. PID1 therefore creates the desktop runtime
directory before the provisioner checks it. The theme unit still runs before the
session owner, normal appearance service and shell. Arguments, capabilities,
theme preservation, original startup/stop limits and `Restart=no` are unchanged.
No directory-reset script, sleep or retry supplies readiness.

The earlier installed ARM cold boot failed theme startup before PID1 started the
user runtime-directory unit. Its console establishes the missing ordering
prerequisite; it does not identify the provisioner's exact failing operation.
That failed run is preserved. A subsequent generic ARM/PID1 cold boot completes
theme provisioning with this dependency and starts the selected appearance
service. Full session acceptance still fails in simulated-wireless startup; no
rendered UI or normal guest shutdown is qualified. See the
[installed integration evidence](../../docs/native-installed-wireless-coldboot.md).
