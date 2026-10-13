# Incoming Messages Binder fixture

This separate offline test composes actual generated frozen public messaging V3
NDK/Rust callbacks, genuine retained ARM64 Android Binder/Bionic, the current sole
supervised ReceiptOwner/Inbox child and the existing current-authority read path.
The normal installed Messages entrypoint remains history-only. This fixture has
no vendor service lookup, modem ACK/send operation, production selector or phone
access. The 512-byte bound applies to callback copies/retained payloads; generated
Binder decoding may allocate before the callback.

Build with `bash tools/native-messages-incoming-binder/run.sh build`; create the
dedicated cloned root disk with `clone` once. Run `incoming-green [unique-label]`
for the actual ARM kernel suite, or `smoke [unique-label]` for callback-only scope.
`incoming-systemd`, `incoming-stop-green` and `incoming-systemd-stop-green` expose
focused actual unit/completion checks. Result and selection directories refuse
replacement. `incoming-red`, `incoming-stop-red` and `incoming-systemd-stop-red`
expect their specific executed intermediate failure and return exit1; they require
the corresponding preserved pre-fix source/binaries, not current green binaries.
Current docs explain the boundary in [native-messages-incoming.md](../../docs/native-messages-incoming.md).

Each peer has an explicit fresh generation, two slots and one registration
attempt per slot. Callback replacement, reset and held-registration release are
absent. A synthetic barrier confirms fixture attachment; one-way return alone
establishes IPC progress. The child durably records a one-attempt marker before
any Binder initialization. Child or whole-service restart with the same generation
opens history only. Synthetic successors require actually reaped old storage,
peer and manager processes plus explicit private Binderfs context removal and
fresh independent peer/generation. This fixture teardown is separate from vendor
producer cleanup, quiescence or permission to reattach an uncertain production
source.

The VM has no NIC or host Binder/adb/USB exposure. Its disk is a separate clone
with snapshot writes; project inputs are read-only and results use new private
artifact paths. Original pinned schema/compiler/runtime manifests are verified.
Genuine retained Android runtime bytes stay private in place. Selected authored
source, generated interface and ELF checksums are recorded without copying those
runtime bytes into public material. [Clean incoming provenance](../../docs/evidence/native-messages-incoming-provenance.json)
separates actual integration, actual kernel identity tests and injected ownership
shape checks.

The original `scaffold-v2-smoke` qualified three callbacks/two slots/equal payloads,
exact V3 metadata, UID1000, deliberate second-attachment refusal and retained
death, with zero ACK/send/response-ACK/other counts. Its independent peer and
manager were actually reaped. `scaffold-provenance.json` remains historical and
makes no storage/authority claim. Earlier failed setup attempts and all pre-fix
source/ELF selections remain preserved separately.
