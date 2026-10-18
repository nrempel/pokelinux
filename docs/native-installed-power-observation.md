# Installed ARM battery and brightness observation

The current native Rust battery and brightness services now pass a distinct fresh
ARM cold boot. PID1 starts both enabled units without manual startup. Battery
completes a truthful unavailable observation; brightness reads the real Linux
backlight class through a virtual GPIO driver, observes whole-device removal,
and recovers after rebind in the same service Main and Invocation. Both services
Stop normally. This is a headless generic ARM service pair, separate from the
prepared x86 Settings screenshots and from physical Pixel support.

## Deployment RED and separate cold boot

The same frozen [consumer](../tools/native-installed-services/power-observation-probe.py)
first runs on the unchanged353-package image. It reports two actual assertion
failures: each required unit has LoadState=not-found. All three inner command
waits/PIPE EOFs close, followed by the consumer's expected exit1. A setup failure,
timeout or compiler error would not satisfy this RED.

The existing production `install-native-services` Make target stages exactly
seven explicit files: the two normal ARM executables, two unchanged production
units, two site configurations and one VM-only modules-load file. Preparation
installs them, verifies native loader resolution and units, and enables both while
stopped. A separate manual start/Stop checks executable and root socket identity.
It supplies no cold-autostart proof. The22,544,384-byte prepared layer is exported
only after normal guest poweroff, actual QEMU wait0 and handle closure.

A different QEMU invocation boots that readonly layer through a new overlay.
Fresh BootID and both InvocationIDs differ from preparation. Before the consumer,
there is no install, daemon-reload, enable, start or modprobe operation. Existing
PID1 modules-load starts the retained GPIO driver; both enabled services are
already active. Package inventory remains exactly353, including the prior Qt50.

## Actual observations and Stop

| Boundary | Actual fresh-boot result |
| --- | --- |
| Battery | Completed revision4, available:false, null percent/status, until0 |
| Initial brightness | Real kernel max1/requested1/actual1; credentialed typed Client revision1 |
| Whole-device unbind | Backlight class disappears; same service publishes no observation, revision2 |
| Rebind | Real kernel1 and matching observation return at revision3, same Main/Invocation |
| Whole-pair Stop | Both PID1 Main exit0/Result=success; pidfds dead, processes/sockets absent, cgroups empty |

The three Rust queries execute serially, each within its own original caller
budget of at most one second, with actual wait/EOF before the cutoff. They never
send Set. Optional actual_brightness is this driver's requested-value core
fallback; neither it nor requested brightness measures physical luminance.
No test_battery is present: the selected7.1.6 ARM kernel has no test_power driver.
The earlier positive x86 battery-driver result stays separate.

Preparation closes34 actual direct waits/merged PIPE EOFs; cold closes45. The
host production Make stage's three and separate query compile's ten are distinct.
All timestamps satisfy original cutoffs; both QEMUs exit0, containers disappear
and selected inputs remain unchanged. PID1 service results are separate from
fixture-parent waits. No descendant was observed at Stop, so this run does not
claim an independently retained worker drain test.

## Providers and limits

The normal brightness provider is the current824,216-byte ARM build e194a8c2;
battery is722,472 bytes/6cacd19b. The new730,272-byte readonly query19dc65af uses
the current typed Rust Client and passes strict target Clippy, formatting and
normal Release build. Its actual compiled [formatted source](../tools/native-installed-services/brightness-readonly-probe.rs)
is published unchanged; the original source and formatting-only delta stay bound
in provenance. No daemon was rebuilt just for this fixture.

The older32111d16 backlight daemon and6f8d0cc9 query kernel/Set milestone retain
their own evidence; their runtime result is not inherited by these new providers.
This pair qualifies current readonly ARM execution and cold deployment. It does
not qualify Set, the graphical shell, app permissions/Source, positive ARM battery
values, suspend/charging/thermal behavior, Pixel hardware or a bootable phone.
See [hash-only provenance](evidence/native-installed-power-observation-provenance.json).
