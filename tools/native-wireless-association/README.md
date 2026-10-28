# Explicit wireless association consumer

`consumer.py` is the frozen desired-result consumer for one exact normal CLI
request:

```text
pokelinux-wireless-scan associate-open IFINDEX FREQUENCY_MHZ BSSID_HEX SSID_HEX ORIGINAL_UNTIL_NS
```

Its source SHA256 is
`c7a1ea56fafe318903b37b33707da5bc0812be8d1ed5d45e5baa4f64c9b73ae3`.
It performs no process, socket, clock or filesystem operation. The fixture supplies
actual owned worker wait/EOF receipts, exact authored AP/STA identities and three
independent original `iw` command outputs/receipts. Both the worker and those
kernel samples must close before the same original five-second BOOTTIME cutoff.

The baseline is the existing normal ARM scan executable. Its unknown
`associate-open` request returns actual exit 64 with exact
`invalid_request` / `invalid_arguments` JSON. Independent station `link`, AP
`station dump` and role inventory establish the unassociated baseline before
cleanup. This is an explicit new-operation refusal, not unsupported/status 2 and
not a failed scan. Desired association is asserted once after qualified cleanup.

The completing worker must return exact selected BSSID/SSID/frequency and retained
wiphy/wdev/ifindex, while independent kernel output names both selected peers.
`disconnect_consumer.py` separately validates a later explicit normal Rust
Disconnect and actual station/AP disconnection readback. It does not submit an
action. The fixture admits that action only after association completion and
independent positive kernel proof. Uncertain Connect does not admit another
Connect, renewed deadline, Disconnect or rollback.

The ARM fixture reuses the existing discovery fixture's matching hwsim modules,
signed offline hostapd/iw/regdb package inputs, masked competing managers,
foreground AP and direct process custody. It retains the namespace descriptor
through actual AP/radio teardown, closes every owned process with wait/EOF,
requires the AP control endpoint absent and hwsim radios/module removed, and
qualifies adopted terminal waits plus ECHILD. A new overlay has only readonly
prepared-image backings and is never exported. No host radio or phone is used.

Private sealed selections, original runtime receipts, package archives, images
and executables remain outside Git. See the
[association report](../../docs/native-wireless-association.md) for actual epochs
and the [library](../../native-wireless-scan/README.md) for the production API.

`arm/` contains the exact executed RED and final GREEN guest/bootstrap snapshots.
`red-guest-main.py` and `red-boot.py` name the historical refusing epoch;
`guest-main.py` and `boot.py` name the later completion plus explicit Disconnect.
The staging `guest.py` is shared byte-for-byte. The original package helper and
capability reducer come from `tools/native-wireless-scan/arm/`; the matching
`owned.py` comes from `tools/native-installed-services/`. Use the recorded private
selection/direct-input pins rather than treating these snapshots as a standalone
VM installer. Their absolute work/close cutoffs belong to each original epoch.

The normal completing ARM worker and the unchanged association consumer pass one
actual fresh hwsim epoch. The separate explicit Disconnect validator also passes
with independent kernel readback. Earlier uncertain native attempts and both
actual decoder REDs remain recorded in the report; the proposed frame-decoding
regression was superseded, and no management-frame state machine was added.
