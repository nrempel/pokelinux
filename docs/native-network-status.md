# Native local network status

Settings now presents coarse local network counts through a read-only Rust
client. A healthy empty namespace shows zero interfaces and zero address records;
service loss or expired data clears both values to dashes. The card uses the
shared Omarchy theme and explains that counts do not indicate internet access.

| Boundary | Qualified result |
| --- | --- |
| Native service | Existing Rust kernel observer and one owned finite collector; no names, addresses, routes, SSIDs or mutation protocol. |
| Installed ARM service | Actual missing-unit RED → production two-file Make stage → install/enable while stopped → separate fresh PID1 autostart, healthy0/0 read and normal Stop. |
| Rust client | Compiled nonconnecting-stub assertion RED → same unprivileged native C ABI consumer passes with the normal host Release daemon. Seven actual-socket/ABI groups, fmt and strict all-target Clippy pass. |
| Settings | Compiled normal-QML missing-card assertion RED → current production CMake build and unchanged card assertions pass. One real adapter/QML instance follows0/0 → service loss → actual getter expiry without Qt timer delivery → dashes → fresh service/connection →0/0. |

The [service contract](../native-network-status/README.md) retains root-owned
namespace and collector lifetime. The [Rust reader](../native-app/network-status-client-core/README.md)
owns the fixed socket, root-peer check, strict six-field parser, original500ms
request deadline and getter expiry. Qt adds a small scalar adapter and normal
QML presentation; it performs no network transport or recovery policy.

The installed ARM tests use an independent root Python socket consumer, not the
new Rust client. The separate host reader/QML run uses real UID11050 and the
normal Rust daemon against independently witnessed only-loopback kernel state.
Its native Qt6.8.2 build links the three explicit matching Rust archives once.
The normal Settings binary is compiled/linked, and the components create the
normal QML resource without showing a window. This supplies no new screenshot,
normal-main execution, measured DPR, default-Cargo CMake run or ARM frontend
execution. The earlier ARM59fb0ef1 and brightness captures retain their own
pre-network-card source cohorts.

The counts exclude kernel loopback interfaces; address records may be tentative,
down or link-local. Healthy0/0 qualifies the observation path, without claiming
usable networking. Interface creation/removal, Wi-Fi, Bluetooth, Internet/mobile
data, modem behavior, the matched Pixel kernel/policy and physical-device tests
remain separate gates. No physical phone was accessed.

[Installed ARM provenance](evidence/native-installed-network-status-provenance.json)
and [Rust/client/QML provenance](evidence/native-settings-network-status-provenance.json)
keep authored host execution, generic ARM, crossbuild and unqualified hardware
results separate. The private Messages, Source/authentication and three-app
navigation branches remain parked.
