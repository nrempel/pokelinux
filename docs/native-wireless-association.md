# Explicit native open wireless association

The new library operations submit one exact open-AP association or one separately
requested Disconnect through Linux `nl80211`. They use the existing bounded
worker model; there is no new daemon or connectivity-service backend. Normal
scan remains a separate operation. No physical phone or host radio is used.

## Actual RED

The frozen consumer requests `associate-open` from the unchanged normal ARM scan
executable, naming the independently authored nonce SSID, exact AP BSSID/frequency,
actual station ifindex and original five-second BOOTTIME cutoff. The real CLI
returns exit 64 with `invalid_request` / `invalid_arguments`. This is the existing
CLI's definite refusal of the requested new operation, not unsupported/status 2 or
a failed scan.

Before teardown, original `iw` queries report `Not connected.`, an empty AP station
dump and unchanged STA/AP wiphy/wdev/ifindex/MAC identities. Actual worker wait and
stdout EOF occur about 48.9 ms and 51.1 ms after arming. The one desired association
assertion fails only after qualified owned cleanup. Fifteen host, three boot,
87 guest and one poweroff waits/EOFs qualify; 33 adopted terminal waits precede
actual ECHILD. QEMU exits 0, the deliberately failing RED container exits 1 and is
removed, and selected inputs remain unchanged.

The unchanged [consumer](../tools/native-wireless-association/consumer.py) hashes
`c7a1ea56fafe318903b37b33707da5bc0812be8d1ed5d45e5baa4f64c9b73ae3`.
Its classification and desired assertion remain separate from outer orchestration
success. A prerequisite, uncertain outcome or cleanup failure cannot qualify RED.

## Implementation and current verification

`associate_open_once` subscribes to MLME before one CONNECT with exact BSSID,
frequency and SSID restrictions and open-system authentication. ACK0 and progress
notifications do not complete it. Completion requires the selected success event
and complete generation-consistent associated-BSS readback for the exact peer.
`disconnect_once` separately requires a local reason 3 Disconnect event and
readback with no associated BSS. Namespace and interface identities are retained
and rechecked. All operations use their original absolute BOOTTIME cutoff.

The normal CLI does not request socket-owner disconnection when its worker exits. Any possibly
submitted failure remains uncertain and admits no resubmission, reconnect,
automatic Disconnect, rollback or refreshed action deadline. The fixture requests
Disconnect only after both normal Rust completion and independent positive STA/AP
kernel observations. Socket closure and process reaping do not prove cancellation
of an external kernel action.

The first build fails before compilation because its private mount directory has
the wrong owner for the capability-restricted builder. Two later epochs fail
strict Clippy on a nested conditional and constant-size iterator. All are retained.
The next build passes 19 host tests, formatting, strict Clippy, unchanged dependency
lock and normal host/ARM Release builds, using no default feature and optimization 3.

Three actual ARM attempts remain failed. Each independently shows the exact
kernel association, while the worker reports an uncertain parse result: first
`Malformed`, then `MissingAttribute(1)` twice. None submits Disconnect or qualifies
normal association completion. Every epoch retains its own original worker/AP,
QEMU and container closure evidence.

Two focused decoder REDs expose unrelated-interface filtering and nonterminal
progress being subjected to the terminal schema. Their regressions pass after
narrow changes: filter by IFINDEX before selected-interface requirements and
ignore nonterminal MLME progress without decoding authentication frames. Selected
CONNECT/DISCONNECT still require their exact terminal identity/status and kernel
readback; ROAM contradicts the restricted peer selection. These component fixes
do not establish which notification caused the native failures.

The final build passes 21 host tests, strict Clippy, formatting and normal
host/ARM Release compilation with unchanged dependencies. Bounded missing
attribute context identifies the fixed operation stage and MLME command number
without raw payload logging. The prepared snapshot before that context change
remains unexecuted.

## Actual native GREEN

The exact same frozen desired consumer passes with the normal completing ARM
Release executable. Rust receives only the exact selected open-AP request and its
original cutoff; original `iw` performs observations, not the association.
The result names the exact BSSID/SSID/frequency and retained station identities.
Independent station link and AP station table output agree after the actual Rust
worker wait and stdout EOF. The kernel association therefore survives that
worker's natural exit in this controlled epoch.

Only then does the fixture submit one separately requested normal Rust
`disconnect` action. Its worker returns exit 0 with completed local disconnection.
Independent station `Not connected.` and empty AP station table readback confirm
it before that action's original cutoff. Neither action is repeated or given a
refreshed deadline. Both action workers and their independent kernel observations
close under their respective original five-second bounds.

The single desired assertion passes after all owned cleanup. Fifteen host, three
boot, 91 guest and one poweroff waits have actual stdout EOF. Thirty-three
successful adopted terminal waits precede actual ECHILD; their origins were not
sampled, and no adopted PIPE EOF or vendor cleanup is inferred. QEMU, container
and attached CLI exit 0; the exact container is removed, the same Docker daemon
remains, and every selected input and source is unchanged.

This one controlled ARM/TCG epoch qualifies the exact open association and explicit
Disconnect path. It does not attribute a particular notification to the earlier
parse failures. The [hash-only provenance](evidence/native-wireless-association-provenance.json)
keeps the genuine CLI refusal RED, proposed and final decoder tests, all failed
native attempts, normal builds and passing native epoch separate.

## Split AP and station namespaces

A later native ARM prerequisite passes with the same normal 560,400-byte Rust
executable, SHA256
`4598f2e121cd78e8c8f794efde23c99f759985567aa1df6d7eaa18b4e5813044`.
No production source or build changes are involved. The fixture creates both
hwsim radios in its fresh guest namespace, then moves the still-down AP phy once
into a separately retained namespace before starting hostapd. The station and
Rust workers remain in the original namespace. This separation prepares a later
packet test whose traffic cannot take a local route between two addresses in one
namespace; this prerequisite itself assigns no addresses and exchanges no IP
packets.

The [new frozen consumer](../tools/native-wireless-association/split-namespace/consumer.py)
hashes `2417ecc302f0f63e5a979eeea7dd4bdfd30711b69b6bb4adf4a287d91b0ea3f3`.
It retains exact worker completion and selected-peer checks, and validates four
separate original `iw` receipts: station link/inventory in the original namespace
and AP station table/inventory in the peer namespace. AP commands use explicit
`nsenter --no-fork` targeting the owned keeper. Actual hostapd and keeper
namespace identities match the retained peer descriptor; both namespace
descriptors belong to the same kernel boot and remain distinct. Interface indices
are namespace-local; the oracle checks each role in its own inventory. Four
separate synthetic parser controls accept the intended evidence, reject a shared
namespace and swapped role inventories, and accept equal indices in different
namespaces. Those controls establish no native kernel result.

An initial preparation remains unexecuted after a local socketpair check exposed
Linux's echoed `MSG_CMSG_CLOEXEC` return flag. The next actual container exits
before guest execution: its selected UID1000 cannot write the new UID0/mode0755
results root. All 15 host waits/EOFs close and that container is removed; this is
an infrastructure failure, not a Rust or kernel behavior RED. The corrected
selection matches the prior successful results root, UID/GID1000 and mode0700,
and checks that metadata before execution. A capability-free UID1000 host process
also reads every selected input and actually creates, fsyncs, closes and removes
an output probe. The failed and passing selections retain identical consumer and
guest bytes.

The corrected epoch completes one normal Rust association with exit0. After its
actual wait and stdout EOF, independent station link and AP station table output
confirm the exact selected peers across the two namespaces. A separately
requested normal Rust Disconnect exits0, followed by station `Not connected.` and
an empty AP station table. Both actions, their actual wait/EOF and all four
independent observations use their respective original five-second BOOTTIME
cutoffs. No association, namespace move or Disconnect is retried.

All 123 direct waits have actual stdout EOF: 15 host, three boot, 104 guest and
one poweroff. Thirty-three successful adopted terminal waits precede actual
ECHILD; their origins and PIPE EOF are not inferred. Actual AP SIGTERM/wait0/EOF,
control-endpoint absence, hwsim module/radio removal, peer namespace returning to
loopback only, keeper STOP/wait0/EOF and closure of both namespace descriptors
pass. QEMU, container and attach exit0; the exact container is removed on the
same daemon and all selected inputs remain unchanged. Existing resource limits
and outer work/close cutoffs are preserved.

The exact executed [guest](../tools/native-wireless-association/split-namespace/arm/guest-main.py)
and [boot](../tools/native-wireless-association/split-namespace/arm/boot.py)
snapshots accompany the consumer. Staging reuses `arm/guest.py` and
`disconnect_consumer.py` from the original association fixture,
`tools/native-wireless-scan/arm/{arm_tools,worker_caps}.py`, and
`tools/native-installed-services/owned.py`. The staged `namespace_peer.py` is
byte-identical to `tools/native-connectivity-control/peer.py`; only its keeper
role executes. The
[hash-only evidence](evidence/native-wireless-split-namespace-association-provenance.json)
binds these public sources, all three preparation/execution outcomes and the
private closure records. Original same-namespace frozen sources remain intact.
These are selected fixture snapshots, not a standalone VM installer.

## Fixture and remaining scope

The fixture reuses the [discovery](native-wireless-scan.md) image's matching
mac80211_hwsim modules and signed offline hostapd/iw/regdb package inputs. A new
readonly-backed disposable overlay creates two fresh radios, one foreground open
AP and one station, with competing network managers masked. The trusted fixture
owns this exclusive radio epoch; names and namespace inventories do not establish
that authority themselves. No original `iw` mutation performs the requested
association or Disconnect.

Actual AP SIGTERM/wait0/EOF, exact control-endpoint absence, removal of its hwsim
radios/module and retained namespace-descriptor closure are distinct from worker
completion. Both interfaces stay without addresses. The original 393 package
versions remain unchanged after the three signed additions.

This slice does not configure an address, exchange IP packets, obtain Internet,
implement WPA/credentials, integrate normal Settings/connectivity ownership,
qualify a Pixel backend or access a physical phone. The next product boundary is
independent address and packet evidence after completed association. The existing
ARM persistent-worker `ReceiveCommand` 50 ms failure remains separate; a headless
prerequisite does not repair Settings or establish persistent wireless ownership.
