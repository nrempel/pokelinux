# Messages request and model engine in Rust

The app-local Rust engine now owns Messages request construction, response
validation, accepted models, pagination, stale-result generations, scheduling,
notification ownership and shutdown decisions. Qt retains QObject/QML, one real
QThread, credentialed nonblocking SEQPACKET I/O, received-rights closure, queued
invocation and actual native wait. Permissions and service lifecycle remain in
the existing Rust owner; this port adds no provider authority or SMS sending.

Safe engine/model/protocol/layout modules forbid unsafe code. The narrow C ABI
copies bounded inputs and outputs through one internally serialized handle;
only its pointer boundary and fallible libc BOOTTIME helper use unsafe code.
The twelve-function ABI has no legacy implementation or fallback. Views cannot
send, expire or renew work. A copied plan retains its original identity/deadline.

Requests alternate bounded conversation/message reads, with one original1s
BOOTTIME exchange. Connect retry100ms and completion cooldown250ms retain their
original wake. A timely received event can publish after GUI delay; an action's
generation rejects stale in-flight/queued results. GUI delivery can precede the
worker's notification ACK: the ACK slot survives FIFO removal independently.
Stop cancels notification work, retains every issued-connect close obligation,
and owns one500ms budget. Actual successful Qt wait plus observed worker exit
is required before free; a worker-exited flag or false wait never suffices.
Initial CONNECT remains clock-ungated, while SEND requires a valid sample.
Clock failure produces unavailable then close/exit, with no recovered-clock retry.

Pages retain bounded original array bytes and unknown keys, last duplicate keys,
row order and duplicate rows. Required numeric fields use exact decimal
integrality. Accepted pages must also fit Qt's finite numeric presentation domain.
Authored validation rejects unpaired Unicode surrogates and IDs other than exact
32-byte lowercase hex. The measured old Qt accepted a surrogate and terminal ID
newline; these are explicit tighter policies, not baseline defects. Invalid
pages clear both lists; an exact denied result selects denial. A combined invalid
page/overflow plus denied can conservatively classify differently from old Qt;
that refinement is deferred. Exhaustive parser equivalence is unclaimed.

## Actual red-green evidence

Before implementation, the existing QObject/header and seven native worker
controls pass. Forty current-Qt parser rows retain37 fixed assertions and three
observations, with45 actual root-peer waits across six behavior modes. The frozen
nine-group consumer compiles with assertions, then links unsuccessfully on exactly
the twelve absent Rust exports. This is genuine structural RED, not an old Qt bug.

Two later actual integration failures precede their repairs:

* Holding the real QThread before Rust entry makes its real20ms wait return false.
  The candidate wrongly returns REFUSED and the test aborts. Removing that one
  pre-entry refusal yields WAIT within the original budget, then actual entry,
  exit, true Qt wait and free. The identical frozen fixture passes; no wait
  result is injected and no socket is created in this control.
* The unchanged original Qt accepts a finite unknown number and returns unavailable
  for overflow. The first Rust candidate instead aborts on that overflow at the
  whole-client boundary. Rust page admission now refuses it before mechanical
  projection. The same finite/overflow fixture passes with the final engine.

The early Qt setup failure remains failed: cap-restricted container root could
not traverse a private harness directory. The successor binds only the required
authored files read-only into scratch directories. No original private directory
permissions were changed. Earlier unused preflight/source preparations remain
separate; they supply no functional pass.

## Host qualification

All checks use Harbour and the frozen final source, with Qt6.8.2 on native x86:

| Boundary | Actual result |
| --- | --- |
| Rust | Formatting, strict all-target Clippy, three unit cases and locked/offline optimized x86/ARM64 archives pass |
| Independent C ABI | All nine frozen groups pass; actual socketpair credentials/EOF/EBADF are distinct from logical join reports |
| Current Qt behavior | Six modes pass, including40 parser rows and45 root-peer waits |
| Existing worker | All seven original classifiers pass; held-clock and false-wait-report controls terminate with their expected SIGABRT dispositions |
| Qt scheduling/Stop | All five frozen real-thread/socket ordering groups pass |
| Permanent tests | All13 CTest cases pass: five ordering, pre-entry, overflow and six current-Qt modes; these overlap the groups above |
| Appearance | All29 unchanged shared cases pass in their own isolated fixture |
| Installed app boundary | Actual UID11010 clients and installed QML pass installed/live-appearance/Messages-policy modes without creating a window; four normal-main argument/UID refusals pass |
| Native default build | Production CMake invokes locked/offline Cargo, links the full app and passes its permanent nine-group CTest; forced cross configure rejects a missing matching-target archive |

Actual symbols retain twelve unique strong API functions. Six explicit native
binary link commands each select one pinned archive; full app/consumer loader
resolution passes. The default build is separately executed and has its own ELF
hash, rather than inheriting the explicit provider's result.

The seven final owned phases close91 host waits/91 stdout PIPE EOFs. Their six
Qt phases separately close113 inside waits/113 stdout PIPE EOFs and45 regular
stderr handles; regular-file closure is not PIPE EOF. Exact container absence,
same daemon, unchanged selected inputs and original cutoffs pass. An adopted
peer's SIGKILL wait in the earlier overflow failure is failed-run process closure,
not a normal Qt join or vendor cleanup.

[Hash-only provenance](evidence/native-messages-rust-client-provenance.json)
records the frozen source/oracles, actual failures, final binaries and results.
The tests are checked in under `native-messages-ui/client-core/tests` and
`native-messages-ui/worker-tests`.

## Manual ARM build and component execution

A later manual native build under a generic Arch ARM7.1.6 kernel in QEMU/TCG
links the full current normal app against Qt6.12.0 and the matching ARM Rust
archive. Actual g++/moc/rcc identities and50 Qt libraries/plugins match the retained
provider. All nine frozen C ABI groups execute successfully. The optional seven
worker controls also pass their typed classifiers, including two expected SIGABRT
refusals; all five Qt ordering cases pass. The normal application main is not run.

The retained evidence closes49 distinct guest process waits and18 PIPE EOFs;
compiler/tool regular logs and the outer regular console are counted separately.
Original420s work/540s QEMU/600s outer bounds, exact container absence, same daemon
and input closure pass. This is manual ARM build/component evidence, not production
ARM CMake, a GUI or physical-device qualification. See the separate
[hash-only ARM provenance](evidence/native-messages-arm-client-provenance.json).

## Native ARM production CMake build and installation

A separate native build in the generic Arch ARM7.1.6 QEMU guest now uses the
current production CMake path. Eight signed offline build-tool packages extend
the actual353-package base to361; all original versions remain unchanged.
CMake4.4.4/Ninja1.13.2 build the full Release app against Qt6.12.0, GNU16.1.1 and
the explicit matching ARM Rust archive. The50 actual Qt libraries/plugins match
before and after, and the normal ELF and archive each expose the exact12 APIs.

The frozen nine-group consumer passes with assertions enabled. Actual CMake
installation places the normal app, desktop entry and protected
`PhoneAppearance.qml` under `/usr`, with root ownership, required modes and
matching hashes. The current six-argument contract is bound through the unchanged
compiled main and installation; normal ARM main/UID11010/window execution remains
open. This explicit-archive build does not qualify ARM default Cargo.

All24 guest command waits and24 merged PIPE EOFs close within their original
cutoffs. One separate native C ABI child wait adds no PIPE EOF. The actual outer
Docker CLI wait, exact container absence, same daemon and input closure pass;
regular console output is counted separately. The original420s work/540s QEMU/
600s outer bounds remain. CMake/Ninja compiler descendants are not reported as
separately observed direct child waits. The earlier seven worker and five Qt
ordering controls retain their separate manual ARM result. See the new
[hash-only CMake/install provenance](evidence/native-messages-arm-cmake-provenance.json).

## Prepared normal x86 window

A later fresh Qt6.12 production CMake build now passes the normal UID11010
window through denial, granted authored history, original Omarchy live dark/light
themes and revoked-list clearing. One app/window is retained; native Stop and
owned closure pass. Root reviewed all four unmodified Pixel10-sized captures.
See the [prepared GUI report](native-messages-prepared-tracer.md); its external
window/output measurements do not claim direct normal QWindow/DPR values.

## Remaining integration gates

The ARM normal main remains unexecuted. An ARM successful-main window, ARM
default Cargo, the complete installed ARM service/app/shell cohort, cold installed
Omarchy/session composition, physical suspend, stuck kernel tasks and native
display cleanup remain open. Existing older Messages GUI/ARM results select their recorded older
client and are not inherited. The new x86 screenshots qualify only their prepared
session; no phone, vendor modem,
continuous incoming/SMS send, notifications/wake or carrier result is claimed.
The separate shared appearance client still contains C++ policy requiring Rust
migration. Upstream Qt/Hyprland/Aquamarine remain substantial C++ dependencies.
