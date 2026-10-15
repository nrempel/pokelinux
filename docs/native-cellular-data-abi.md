# Native cellular DataV3 ABI prerequisite

Frozen public DataV3 Rust bindings and an independently generated NDK C++ peer
now interoperate through actual Binder under a generic AArch64 Linux7.1.6 VM
kernel. A dedicated Rust fixture manager transfers one named fixture binder; a
Rust client obtains its Data endpoint, copies callbacks and observes peer death.
This is an ABI/transport prerequisite. No native Data owner, kernel association,
APN acquisition, slot mapping, carrier operation or physical phone was exercised.

The [public provenance](evidence/native-cellular-data-abi-provenance.json) records
source identities and summarized execution evidence. See the
[bindings README](../native-cellular-boundary/README.md) and
[fixture tools README](../tools/native-cellular-data/README.md).

## Frozen generated boundary

The original41 public AIDL files comprise31 DataV3 files and10 imported RadioV3
files, retained with upstream headers, NOTICE and
[SOURCE.json](../native-cellular-boundary/SOURCE.json). The pinned upstream commit
is `488942f82bd1bc9ad1cb65a02c71421dc3a6a3d6`. The actual endpoint reports version3
and hash `cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df`.

`native-cellular-boundary` generates synchronous Rust types with pinned
rsbinder/rsbinder-aidl0.12.0. Its default feature set contains only the public ABI.
The opt-in `test-fixture` feature adds two authored control AIDL files to that
same generated module; it preserves the public schemas. The authored control
interface is version1/hash `b74bae7e2fba9b8fbb574174a24e7431dbb6281b`.
It is test infrastructure, separate from the public Data interface.

The separate NDK compile prerequisite generated43 C++ sources and129 headers,
compiled52 translation units with at most16 compiler children, and recorded52
actual compiler waits plus1066 distinct dependency paths before three actual
links. Complete selected SDK AIDL and its bundled host libraries, NDK
compiler/resources/sysroot, public headers and source inputs were pinned before
and after. Generated files, objects and emitted AArch64 ELFs were also pinned.
This build executed no generated ELF; NDK link stubs were compile inputs only.
Rust client and dedicated manager formatting, Clippy with warnings denied, and
host/ARM64 release builds separately pass. No host behavioral test is claimed.

## Actual ARM Binder execution

The runtime used a fresh private binderfs device, the dedicated fixture manager,
the generated NDK peer, genuine retained Bionic libraries and explicit Android
loader, and the Rust client. Actual `/proc` observations show manager, peer and
client at UID/GID1000, no supplementary groups and zero effective capabilities.
The peer's15 mapped ELF files comprise13 catalogue libraries, loader and peer;
14 catalogue libraries were installed, with `ld-android.so` not separately mapped.
Three property-area files are tracked separately from ELFs, as is the Binder
device. Manager/client GNU loader/libc/libgcc mappings were also checked. Actual
mapped identities and bytes remain unchanged after the owned waits. No NDK link
stub is admitted as a runtime library. Missing `/linkerconfig/ld.config.txt`
warnings remain in the Bionic helper/peer logs; Android linker namespace/config
and SELinux qualification remain open.

The manager accepts one fixture name/descriptor and transfers that binder; it is
not a general or installed service manager. Its independent transfer/lookup
witnesses agree with the peer and client. The client validates remote fixture
and Data descriptors, interface versions/hashes and one explicit fixture
generation before attaching real generated callback binders.

One execution then verifies:

- All10 `setupDataCall` arguments arrive unchanged, including all21
  `DataProfileInfo` fields, nested slice/traffic descriptor, addresses and DNS.
- After the one-way setup submission, an actual100ms boot-time interval has no
  callback. Ordinary IPC return is not treated as a completion receipt.
- The peer submits a SOLICITED success with deliberately wrong serial714, then
  serial713. Rust owns two complete callback copies, each checked for its actual
  serial/type/error and calling UID. This is no native-owner serial-fencing proof.
- The chosen result preserves all18 `SetupDataCallResult` fields and its nested
  contents. A one-element unsolicited list equals that copied result; a later
  empty list is copied distinctly.
- Actual peer exit delivers a Binder death notification. The client drops its
  watch and callback references before publishing its bounded final receipt.

The fixture's accepted subset uses `Qos::noinit`, empty QoS sessions and bounded
strings/vectors; other QoS variants are unqualified. These limits apply after
the generated decoder and do not qualify hostile wire-allocation bounds. The
captured one-way calling PID is0 and UID1000; PID0 is not a production identity
or permission grant. Addresses and `interop.invalid` profile text are authored
payloads, not kernel state, APN policy or an acquired connection.

## Custody and deadlines

Peer and client share the original60-second absolute `CLOCK_BOOTTIME` cutoff,
with no second setup or reset. Setup has its original20-second allowance,
cleanup ends at the original62-second offset, and the guest operation has a
120-second whole budget. Observations and waits are checked against their
original cutoffs; this does not establish hard return bounds during scheduler
stalls or suspend.

All eight direct guest children have actual waits and output EOF before their
cutoffs: seven exit0, while the owned manager is deliberately killed and waited
with exit−9 during cleanup. Peer exit0 and its death notification are separate
from manager disposal. Binder/property fixture resources are removed after the
waits, binderfs unmount and poweroff pass, and the owned outer Docker CLI exits0.
A fresh exact-CID absence check on the same daemon closes the container;
original and selected inputs remain unchanged. Root independently rehashed all
selected inputs and all originals, including the backing image, kernel and
initramfs; the final qualifier and independent ABI review both pass. No Binder pool join or native
transport-worker join is claimed from process exit.

## Preserved prerequisites and remaining work

The authored fixture import/generator-input corrections are source-generation
prerequisites; public frozen AIDL was unchanged. The first NDK attempt stopped
at the CID publication race before generation; the next failed to start SDK AIDL
without its bundled `libc++.so`. Both attempts and their closure records remain
preserved. The later fully pinned compile passes. None of these prerequisite
failures is a native behavioral RED, and this ABI execution is no owner RED/GREEN.

A native Data owner still needs operation/source/cancellation fences, durable
uncertainty and recovery, transport-worker supervision, ACK_EXP policy and
actual backend identity. Kernel observations in the separate
[network observer](native-network-observer.md) have not been associated with
this endpoint. Production admission also needs the actual modem endpoint and
version/hash, slot/subscription/registration mapping, carrier/SIM/APN policy,
permissions and installed lifecycle/service integration, responsive Stop,
persistence, matched Google/Pixel kernel/SELinux/VINTF and authorized hardware
qualification. No vendor endpoint, carrier traffic, normal installed app,
authentication or physical phone was used here.
