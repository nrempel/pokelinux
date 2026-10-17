# Native battery observations

Rust service for public capacity, charging status and availability. The selected
Linux `power_supply` backend reads one bounded `uevent` packet in a separately
owned process. The supply name is deployment input; phone policy and view types
contain no Pixel or test-driver names. See [contract and evidence](../docs/native-battery.md).

Build with `cargo build --manifest-path native-battery/Cargo.toml --locked --offline --release`.
The tests use temporary authored files, real Unix sockets and real processes/FIFOs;
run them as root in a disposable Linux container. They never inspect host batteries.
The service's normal entry is:

```
battery-service --serve /run/pokelinux-battery/snapshot.sock /sys/class/power_supply SUPPLY
```

Install the [unit](packaging/pokelinux-battery.service) and an explicitly selected
`/etc/pokelinux/battery.conf` through the finite deployment manifest. The
[virtual profile](packaging/vm) selects a guest-only kernel test battery.

Connecting to the socket returns one JSON line and EOF. No request is sent and
there is no mutation verb. Available replies contain capacity0..100, a charging
status and an absolute decimal CLOCK_BOOTTIME `until_ns`. Unavailable replies
have null capacity/status and `until_ns:"0"`. Coarse battery state is public and
supplies no permission, lease or charging/suspend command.

Snapshot/Stop remain responsive while a reader is blocked after launch. A local
trusted executable and default child-wait signal disposition are deployment
requirements; permanently blocked exec setup is not qualified. Installed startup and a fresh automatic cold boot now pass against the real
`test_power` kernel driver in disposable x86 KVM guests, including changing
capacity/status, absence and normal PID1 Stop. The readonly Rust client and a
normal Qt app build also pass. A prepared normal Messages window now visibly
follows all three virtual-driver states through the real service/client path,
with original Pixel10-profile captures. ARM kernel integration and physical
Pixel battery behavior remain separate gates.
