# Persistent wireless service acceptance

This directory contains a concrete normal Main/actor integration consumer and
the existing generic ARM hwsim owner adapted to call it. No persistent-service
RED, GREEN, production backend implementation or phone qualification is claimed
by writing these files. Freeze these consumer bytes before production changes.

The normal Main keeps five arguments:

```text
ENDPOINT ROOT0600_STARTUP_PROFILE JOURNAL WORKER_ELF ORIGINAL_START_UNTIL_NS
```

The protected root-owned regular `0600` JSON startup profile is selected once:

```json
{
  "backend": "wireless_open",
  "target": {"namespace": {"dev": 4, "ino": 123}, "ifindex": 3, "name": "wlan1"},
  "frequency_mhz": 2412,
  "bssid": [2, 0, 0, 0, 0, 1],
  "ssid": [112, 107, 108]
}
```

These numbers illustrate the schema; the consumer supplies the actual retained
station namespace/interface and nonce AP. The separate virtual backend has
`{"backend":"virtual_pair","peer_namespace":"/absolute/namespace/path"}`.
There is no argv2 namespace compatibility fallback or implicit backend selection.
Open association and documentation addresses `.1/30` and `.2/30` are authored
tracer inputs. Credentials, WPA, DHCP/DNS, Internet and Pixel hardware remain open.

`consumer.py.prepare()` installs and pins six exact ARM64 ELFs, checks protected
executable ancestry, and runs native `ldd -r` for each. Its `run_case()` receives
the existing owner's actual station/AP roles, namespace binding, foreground AP,
retained namespace keeper and `Scope`. It reuses the existing AP provisioner,
cap0 UDP client, packet peer, address/route parsers and radio readbacks. No prior
one-shot association is admitted.

The actual UID11050 Settings SDK app reaches the normal lifecycle actor and its
durable NetworkControl grant. The consumer requires ungranted refusal, one
granted Connect, matching station/AP and IPv4 state, a fresh cap0 nonce/ACK, actual
SDK Stop/join and Apps child wait, persistent radio/address/same-worker evidence,
a new requester with two fresh observations, a second independent cap0 packet,
and explicit Disconnect followed by independent radio/address absence and the
full quiet period. The original worker's retained pidfd and exact Main children
list fence replacement. Root-private Inspect binds the same worker wire ID.
No action is retried and an uncertain failure never admits another effect.

Stage `service_connectivity_wireless_actor_tests.rs` in the copied lifecycle
`src/`, and append only to its copied `src/service_restore_tests.rs`:

```rust
#[path = "service_connectivity_wireless_actor_tests.rs"]
mod connectivity_wireless_actor_tests;
```

The exact ignored test filter is:

```text
service::restore_tests::connectivity_wireless_actor_tests::settings_sdk_wireless_packet_lifecycle
```

Build the existing `native-connectivity-settings-sdk` app fixture against the
selected current client sources, and select the actual lifecycle library-test
ELF plus normal Main, worker, launcher and storage ELFs. `selection.service_binaries`
has exactly `main`, `worker`, `driver`, `settings_app`, `launcher`, and `storage`;
each row supplies `relative_path`, `guest_path`, and `ELF: {bytes, sha256}`.
Guest executable parents must already exist with protected root ownership.

Stage this consumer as `source/service_consumer.py` beside unchanged
`address_consumer.py`, `wifi_peer.py`, `udp_client.py`, `worker_caps.py`, current
`owned.py`, and `actor_peer.py` copied from `native-connectivity-actor/peer.py`.
The generic `arm/guest-main.py` retains the original package/module/AP/namespace
setup and teardown. `run_case()` uses `Scope(R / 'wireless-service-case')` and
returns evidence; the outer owner calls `qualify_after_cleanup()` and makes one
`assert_desired_after_cleanup()` only after actual full hwsim cleanup.

The six-second functional aggregate, three-second startup, one-second actions,
250ms service lease, 200ms SDK Stop/join and 800ms quiet bounds remain fixed.
Main/actor/requester actual waits, output EOF and retained namespace custody are
separate from actual radio and address cleanup. A known baseline rejection needs
exact pinned Main/worker ELFs, real normal Inspect and SDK unavailable replies,
normal Main/worker closure, independently absent association/address, and all
genuine AP/peer/loader/outer-closure prerequisites. Compile, loader, setup or
custody failures cannot become RED. The baseline's exact selected expected exit
contract lives in `selection.baseline_profile_rejection`.

The copied actor fixture reports after Main/packet cleanup and leaves the sole
desired assertion to the outer owner. The external consumer proves final
Disconnect radio/address absence; profile-before-radio submission ordering,
composed mutation uncertainty, target changes and radio-loss retirement also
require their separate component/event or negative evidence. The read-only
observation helpers do not manufacture such provenance.

The first selected baseline is now built from unmodified current production
sources: normal Main/worker for host and ARM64, plus the copied actual actor
test and normal SDK/launcher/storage ARM64 executables. Source pins remain
unchanged after compilation. This is build evidence only: the hwsim runtime RED
has not run. Three missing non-Linux lockfile archives were obtained from the
public crates.io distribution and verified against the existing lock checksums;
lockfiles and shared registry inputs were not modified. The guest adapter sets
the explicit isolated-test environment after exclusive hwsim setup and creates
the protected fixture executable directory before loader checks. The final
consumer review retains summary flags; full case evidence remains separately in
the guest record to avoid duplicating it under the inherited publication bound.
