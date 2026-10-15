# Native connectivity boundary

`native-connectivity` implements a read-only Rust NetworkManager adapter. It
requires an explicit Unix D-Bus address and pins the daemon's unique owner on
that connection. It is a tested backend boundary, not an installed connectivity
service or a working Pixel Wi-Fi/Bluetooth stack.

Every property/device query targets the pinned unique name. Before reading and
before publishing, the adapter checks that the well-known NetworkManager name
still belongs to that owner. The observation includes the bus ID as well as the
unique name; a name such as `:1.2` is not globally unique across buses. Daemon
replacement requires explicit discovery of a new endpoint. Bus death returns an
error; cached properties or an automatic reconnect cannot report success.

The adapter preserves the daemon's distinct State, Connectivity, wireless enable
flags, device types and device states. An enabled radio flag alone cannot prove
that a Wi-Fi device exists, association completed, an IP address was assigned or
the internet is reachable. In the actual isolated daemon, both wireless flags
are true while only an unmanaged loopback device exists, overall State is20
(disconnected) and Connectivity is0 (unknown). No link is activated by this code.

A shared native-power clock supplies `CLOCK_BOOTTIME`. Each observation has one
absolute one-second budget checked before and after calls and before publication.
The budget survives repeated calls; time asleep consumes it. Per-method D-Bus
waits use the library's250ms timeout. A successful original-daemon response after
an injected boot-time jump is refused. These checks fence publication; they do
not establish a hard wall-clock return bound during scheduler stalls or suspend.
Connection/authentication setup is not bounded by the method timeout.

Decoded property replies are limited to64KiB and128 entries, device-list replies
to8KiB and16 devices, and version text to128 bytes. These are adapter decoding
limits. zbus's separate upstream wire-message allocation and background executor
remain library behavior; this component does not qualify a hostile-bus memory
budget, cancellation or owned production process supervision. Global and device
properties are separate observations, not one atomic daemon-wide transaction.

## Actual boundary verification

Two executed runtime reds preceded fixes: after restart, the initial adapter
published the replacement daemon's data under the prior owner; it also published
a successful response despite an expired injected clock. Corrected runs pass
all eight stages against original NetworkManager1.52.1 and real dbus-daemon:

- Absence produces no discovery success and does not start a service.
- Actual daemon/device properties report no Wi-Fi or reachability proof.
- A successful reply cannot cross the absolute boot-time deadline.
- SIGSTOP of the actual daemon causes a read timeout (about250ms).
- SIGKILL and actual reaping refuse old-owner reads.
- Restart cannot silently replace the pinned owner.
- Explicit new discovery observes the new unique owner.
- Bus death refuses cached success.

The same eight stages pass with an ARM64 Rust client under QEMU user mode against
the same x86 original daemon and bus. This qualifies the ARM64 client/wire path;
it does not qualify an ARM64 NetworkManager process or Pixel kernel. Formatting,
Clippy with warnings denied, and x86/ARM64 release builds pass. There are no
additional component tests counted as proof. See the
[public provenance](evidence/native-connectivity-provenance.json).

The suite owns a private container, bus, daemon and client and reaps every child.
It uses `--network none`, no host D-Bus mounts, no host NICs or physical phone.
All interfaces in its namespace are configured unmanaged; connectivity probing,
DNS management and PolicyKit are disabled only in the disposable fixture.
Capabilities are dropped except SETUID/SETGID to launch the unprivileged Rust
client and KILL to reclaim that owned child. These fixture privileges are not a
production permission design. No vendor binary executes.

```sh
bash tools/build-native-connectivity.sh prepare
bash tools/build-native-connectivity.sh lock
bash tools/build-native-connectivity.sh build
python3 tools/native-connectivity-suite.py --output artifacts/native-connectivity/NEW_X86_RUN
python3 tools/native-connectivity-suite.py --client-arch aarch64_user --output artifacts/native-connectivity/NEW_ARM_USER_RUN
```

## Remaining integration

A separate [native kernel network observer](native-network-observer.md) now
qualifies complete link/address observations and actual TUN-loss withdrawal on
an ARM kernel. Its retained namespace and continuously observed local lifetime
are independent of the NetworkManager owner. Neither observation activates a
link or establishes APN readiness. A separate
[Data/kernel owner](native-cellular-data-owner.md) now qualifies controlled
provider-report/link association through its own retained observer. Three later
actual ARM runs qualify ordinary public-poll setup and independent link/list/
death withdrawal; list/death withdraw while the controller still confirms the
same positive kernel context. [Evidence](evidence/native-cellular-data-public-poll-provenance.json).
This neither proves provider ownership of that link nor composes NetworkManager
under an installed connectivity authority.

Implement the authenticated Rust connectivity authority, bounded responsive
observer/stop IPC, supervised backend execution, subscriptions, permissions,
revocation, installed service lifecycle and native Settings/shell integration.
Qualify scan completion, association, secrets admission, activation/deactivation,
IP configuration, portal handling, disconnect/reconnect and uncertain mutations
separately. Activation receipts cannot stand in for connection progress.

Original NetworkManager is a public reference/backend candidate in this proof;
production ownership, deployment and dependency selection remain open. BlueZ
adapter discovery, pairing, device/media lifetimes, Bluetooth call routes,
tethering and USB ownership remain unimplemented. Pixel BCM4383 firmware,
calibration/regulatory data, AoC/serial Bluetooth initialization, coexistence,
modem data and later authorized hardware tests remain explicit gates. Shared
policy has no Pixel device nodes or vendor protocol IDs.

Primary references: [NetworkManager manager API](https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.html),
[device API](https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.Device.html),
[connectivity configuration](https://www.networkmanager.dev/docs/api/latest/NetworkManager.conf.html),
and [D-Bus identity/lifetime specification](https://dbus.freedesktop.org/doc/dbus-specification.html).
