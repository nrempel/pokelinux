# Persistent wireless service acceptance

This directory contains a concrete normal Main/actor integration consumer and
the existing generic ARM hwsim owner adapted to call it. The second actual ARM
cohort qualifies the baseline refusal RED through this consumer. The third
candidate ARM cohort now passes the same frozen Main/actor service consumer in
a generic ARM64 QEMU guest with simulated hwsim radios. Normal Wi-Fi GUI, WPA,
DHCP/DNS, Internet and physical-phone qualification remain open. These consumer
bytes were frozen before production changes.

The recorded Main cohort used five arguments:

```text
ENDPOINT ROOT0600_STARTUP_PROFILE JOURNAL WORKER_ELF ORIGINAL_START_UNTIL_NS
```

The [pre-migration consumer](https://github.com/nrempel/pokelinux/blob/256c553c1eb7c8434711772fa815d2d5ab21cc00/tools/native-wireless-service/consumer.py)
retains that historical ABI and its recorded evidence. The current consumer
passes only `ENDPOINT ROOT0600_STARTUP_PROFILE JOURNAL WORKER_ELF`; Main derives
its own fixed three-second cutoff. The fixture keeps its independent startup
observation limit and action deadlines. This current launch migration is
unexecuted; the [host CLI pair](../../docs/native-connectivity-installed-startup.md)
does not validate the wireless/ARM/GUI consumer with the new CLI.

The current protected root-owned regular `0600` JSON startup profile uses only
the exact provisioned station interface name:

```json
{
  "backend": "wireless_open",
  "interface": "wlan1",
  "frequency_mhz": 2412,
  "bssid": [2, 0, 0, 0, 0, 1],
  "ssid": [112, 107, 108]
}
```

The consumer supplies the existing fixture station name and nonce AP. Main's
resolver selects it once from a complete bounded kernel snapshot
under the same original three-second cutoff and retains the namespace across
Service startup. The worker still validates disconnected/unconfigured state.
This requires exclusive administration/provisioning and does not prove continuous
ownership of a name or index. There is no resolution retry or rebind. Old numeric
installed fields are refused; the trusted internal resolved worker profile keeps
its namespace and ifindex. The recorded historical successful cohort and its
`CONTRACT.json` retain the old numeric schema at their immutable source commit.
The separate virtual backend has
`{"backend":"virtual_pair","peer_namespace":"/absolute/namespace/path"}`.
There is no argv2 namespace compatibility fallback or implicit backend selection.
The current public profile-writer migration is unexecuted. A separate fresh
headless adapter retains the original successful functional path and now
qualifies the pre-implementation Main's unknown-interface schema refusal;
no actor or worker was admitted by that RED. The exact same frozen consumer now
passes candidate Connect, persistence, two cap0 packets and Disconnect/quiet
with full closure. Its [small reconstruction patch and contract](installed-selector/README.md)
retain the immutable original source cohort. The maintained follow-up variant
still has not executed after migration. This is separate from the historical
worker-refusal contract below.
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
original quiet-window checks. The original worker's retained pidfd and exact Main children
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
unchanged after compilation. Compilation alone is build evidence; the separate second ARM hwsim run now
qualifies runtime RED. Three missing non-Linux lockfile archives were obtained from the
public crates.io distribution and verified against the existing lock checksums;
lockfiles and shared registry inputs were not modified. The guest adapter sets
the explicit isolated-test environment after exclusive hwsim setup and creates
the protected fixture executable directory before loader checks. The final
consumer review retains summary flags; full case evidence remains separately in
the guest record to avoid duplicating it under the inherited publication bound.

Baseline classification also requires exactly the selected unscoped normal-worker
`Error: Kind(InvalidData)` line in Main's inherited stderr, alongside its recorded
worker exit1. Wrapped capability/bootstrap failures or other error text remain
unqualified. This identifies the retained implementation's refusal class, not
the exact underlying kernel error discarded by that implementation. This
baseline contract was observed by the second actual ARM cohort.

The first actual ARM attempt fails the cap0 path preflight before Main/actor
launch or any normal-service network request: `/results` belongs to UID1000 with mode0700,
so capability-free guest UID0 cannot traverse it. The failed case stays separate
from service RED. QEMU wait0, owned radio removal, exact container absence and
unchanged inputs are recorded; normal case closure qualification remains false.
Operational gates, startup profile/socket and journal now live beneath the
root-owned guest input/runtime directory; only output logs remain under `/results`.
The unchanged helper reproduces the bad-ancestor refusal and passes the corrected
layout in actual native host cap0 processes with waits/EOF and original one-second
bounds. The fresh second ARM run passes setup and observes Main Inspect and the
actual UID11050 SDK app both unavailable, with the exact worker InvalidData line,
worker exit1 and Main exit0. No Connect, Disconnect or packet client is admitted.
Independent readback stays unassociated and unconfigured. The actor/app join,
command waits/EOF, AP/radio/namespace cleanup, QEMU wait0 and exact container
absence qualify the intended RED; the desired service assertion remains false.
See the [curated receipt](../../docs/evidence/native-wireless-service-baseline.json).
This RED preceded production implementation against the unchanged consumer;
it qualified baseline refusal only.

The implementation retains actual current namespace and constructing-thread
identity, command/MLME sockets and the IPv4 observer. It brackets profile reads
with exact radio observations and publishes the oldest timestamp. Action-wide
uncertainty survives later component failures; retirement never retries or
performs implicit Disconnect. At that component checkpoint the host library
suite passed38 tests (one explicit ignore), and the radio suite passed28. The
subsequent ARM service result below is separate from those component/build checks.

The first candidate ARM attempt reaches available/disconnected, then loses the
worker during the case. All three counters remain zero, but they advance only
after successful readback gates; they are not reliable dispatch counts on a
failure path. Zero therefore cannot prove no Connect or kernel effect. Failure cleanup removes the owned radios,
closes handles and QEMU/container custody; normal case qualification is false.
The generic worker InvalidData does not identify the originating component error.
This attempt remains separate from service GREEN and the
old baseline refusal RED. [Candidate evidence](../../docs/evidence/native-wireless-service-candidate.json).

The diagnostic second attempt identifies ApplyBackend / Profile::Add /
Observation(InvalidData), with891ms left on the action deadline. Association was
confirmed before that profile call; the composed result correctly stays Uncertain.
Sparse Linux wireless link notifications without MTU were the unsupported parser
shape investigated at that checkpoint. Full dumps retain their required fields;
the original action deadline, no-retry policy and frozen consumer remain unchanged.

The third candidate ARM run, `wireless-service-arm-green-root-3`, passes the full
frozen consumer (`92bfa458…`). The actual UID11050 Settings SDK fixture receives
the durable grant and submits one accepted Connect through the normal actor and
Main. Independent station/AP and fixed IPv4 readback agree with a capability-free
nonce/ACK exchange. The first requester stops and joins its SDK and is actually
waited; a distinct requester reopens, observes the same retained worker and
connection twice, and completes a second independent cap0 packet exchange.
One explicit Disconnect then has independent station/AP disconnection and
address/subnet-route absence, followed by the original quiet-window checks.
The800ms budget includes gate delivery; the peer records zero datagrams and an
actual final EAGAIN after the original cutoff.

Main, actor driver and packet peer exit0. Main normally retires its worker with
SIGKILL9 and retains actual worker wait/output EOF and namespace custody; that
signal is process retirement, not vendor or radio cleanup. Independent radio and
address checks precede owned AP/radio/namespace teardown. Guest, QEMU wait0 and
outer container exit0/absence all qualify, with no unresolved owned child or
changed selected input. The earlier setup failure, baseline RED and two failed
candidate cohorts remain preserved. This result covers an authored open AP and
fixed documentation IPv4 in generic simulated radios, using the SDK app fixture
and authored actor authority. It adds no normal Wi-Fi window, credentials, WPA,
DHCP/DNS, Internet, matched Pixel driver or physical-phone evidence. See
[candidate evidence](../../docs/evidence/native-wireless-service-candidate.json).

## Current scenario follow-up

Current selections require `wireless_service_scenario` equal to `persistent` or
`ap_loss`. The latter deauthenticates the station once from the actual AP after
two packet probes, requires retained IPv4 with a retired original worker and
unavailable Main/SDK, and preserves normal joins, quiet and owned teardown.
The guest records and preserves its actual STA loopback baseline; only the new
AP namespace is required to have empty loopback addresses.

These extended consumer bytes are a new cohort. Their first two ARM executions
both fail at the app boundary: persistence sees app-control UnexpectedEof and
app exit1, while AP loss reaches unavailable Main but does not satisfy the SDK
getter predicate before its original cutoff. Neither qualifies GREEN. Failure
cleanup completed; both unexplained failures remain preserved as `current-arm-root-1`.
The original frozen-consumer success above remains separate.

The fresh `current-arm-root-2` pair now passes both scenarios. Main/worker ELFs,
the Python consumer/guest, desired predicates and original deadlines remain
unchanged from root1. The actor and SDK app fixture add bounded error-only
diagnostics; these are a new test-source cohort, not a causal repair or an
explanation of the earlier failures. Persistence completes two cap0 nonce/ACK
exchanges across actual first-requester wait and fresh reopen on the same worker,
then explicit Disconnect, independent radio/address removal and quiet.

AP loss admits one actual AP deauthentication after two packets. Independent
radio loss is followed by unavailable Main and actual SDK getters with null
configuration and zero observation times. The original worker is independently
terminal before Main Stop, with no replacement and Main still alive. Its actual
exit1 records `RefreshBackend` / `refresh_wireless` radio contention. IFF_UP,
fixed IPv4 and the subnet route remain present after worker retirement and Main
Stop; only the later explicit hwsim teardown removes the interface. No service
Disconnect, reconnect or post-loss UDP probe is admitted. Peer quiet passes
under the original800ms budget, which includes gate delivery.

Both scenarios observe two Apps waits0, both SDK joins0, Main/driver/peer exit0,
complete wait/output EOF and namespace closure, QEMU0 and actual container
wait0/removal/absence on the same daemon. Persistence's worker SIGKILL9 is normal
owned retirement, separate from radio cleanup. Selected inputs and source remain
unchanged during each run. These are generic ARM simulated-radio SDK-fixture
results with authored authority; no normal graphical Settings, installed session,
WPA, DHCP/DNS, Internet or phone result follows. See
[follow-up evidence](../../docs/evidence/native-wireless-service-followup.json).
That record separately maps the executed actor/SDK fixture hashes to the later
rustfmt-only publication hashes. The final formatting check passes; the formatted
publication bytes were not executed in either ARM run.
