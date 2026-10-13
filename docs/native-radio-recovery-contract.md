# Factory radioExternal recovery contract

Status: bounded static recovery on Harbour, 12 October 2026. The actual factory
Binder service adapter and generated native AIDL bindings are now identified,
extracted from the already-retained vendor partition, and matched to the earlier
factory inventory. No vendor target was executed. No native retry, reset,
registration successor, or fragmented semantic publication is enabled by this
work. The durable uncertainty marker remains required.

This is the Samsung backend for the native Rust phone goal. Initial backend qualification requires matched Google kernel, modules and
firmware; Android/Java are reference and test infrastructure. These facts do not define a portable phone API.
[Machine-readable provenance](evidence/native-radio-recovery-contract-provenance.json)
contains artifact digests, exact REA observations and remaining gates. Raw
provider results, extracted binaries and snapshots remain ignored under
`artifacts/radio-recovery-rea/`.

## Exact implementation and version

Only Pixel 10 **frankel CP3A.261005.005**, Android API 37, was investigated. The
retained `vendor.img` came from Google's
[CP3A factory archive](https://dl.google.com/dl/android/aosp/frankel-cp3a.261005.005-factory-3721e6ac.zip).
Its digest is
`0f6fa174fd21791357af40d7c21ab10d6538ef0ba1910655ac1d06ff7f25ee7e`.
Only two small, previously missing libraries were extracted in this increment;
both match the prior native inventory. No factory archive was downloaded.

| Boundary | Factory artifact | Recovered role |
|---|---|---|
| Service host | `/vendor/bin/hw/rild_exynos` and its init definition | Stock radio daemon; inspected metadata only |
| Client manager and complete producers | `/vendor/lib64/libril_sitril.so` | `RilExternalManager`, `RilExternalClient`, `RadioExternalCallbackImpl`, whole-message AIDL response/indication splitters and separate pending request list |
| Actual Binder service adapter | `/vendor/lib64/libril-aidl.so` | Defines the imported `RadioExternal::make`; wraps the supplied callback interface, retains native callback proxies and delegates calls |
| Generated Binder protocol | `/vendor/lib64/vendor.samsung_slsi.telephony.hardware.radioExternal-V1-ndk.so` | Native proxy/stub dispatch, parcel fields, transaction flags, interface version/hash |
| Modem request implementation | `/vendor/lib64/libsitril.so` | Separate upstream/modem side; not executed or claimed replaced |

The factory VINTF entry declares
`vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal/default`.
The generated native service returns interface **version 1**, hash
**`62c86cf5f8f15456f451d2241b4f21b10627e514`**. These are static observations of
this artifact, not results of a live service lookup. Record the library digests
and interface identity together when qualifying another build.

The public-source search found the older Samsung HIDL
[1.0 interface](https://github.com/LineageOS/android_hardware_samsung_slsi-linaro_interfaces/blob/a23ed82b3f3859aa9e0207036b0d0e98119b959b/telephony/radioExternal/1.0/IOemSlsiRadioExternal.hal)
and
[1.1 extension](https://github.com/LineageOS/android_hardware_samsung_slsi-linaro_interfaces/blob/a23ed82b3f3859aa9e0207036b0d0e98119b959b/telephony/radioExternal/1.1/IOemSlsiRadioExternal.hal).
In that source, clear is synchronous; 1.1 adds a client name and indication
filter. The current factory AIDL clear is one-way. HIDL's old client-ID enum and
synchronous clear must not be imported as the current AIDL allocation or cleanup
contract. The bounded public repository/tree search did not locate the matching
AIDL service/manager source. This is a search boundary, not a claim that no such
source exists.

## Cleanup and remote uncertainty

The generated native clear proxy sends transaction **1** with one-way flag;
registration sends transaction **5** synchronously and reads status plus an int.
The current native callback segment proxies send transaction **2** one-way.
The low flag bit is the [NDK one-way flag](https://developer.android.com/ndk/reference/group/ndk-binder).
These now come from the actual factory native sender, separately from the older
APK proxy evidence.

The recovered service clear adapter delegates to its callback and creates an OK
status locally. The generated one-way stub discards that status and writes no
cleanup reply. The delegated callback reaches manager clear: under the manager
mutex, it finds the numeric client ID, clears callback references and segment
storage, and removes that client. A successful Binder submission therefore does
not certify that this path ran, that prior callbacks were consumed, or that a
pending modem request was cancelled.

The earlier [cleanup investigation](native-radio-cleanup.md) establishes a
separate pending-request lifecycle. Client clear/reset and pending completion
are different paths. This increment supplies no modem cancellation or drain
acknowledgement. The recovered radioExternal Binder method inventory contains no
reset/drain method. The inspected init trigger merely resets
`vendor.sys.rild_reset` from 1 to 0; that definition does not establish a modem
or request reset. Other reset mechanisms remain outside this bounded trace.

A synchronous version/hash lookup after one-way clear is not a recovered cleanup
barrier: its generated handler returns metadata without taking the client
manager lock or consulting pending work. Android's
[Binder threading contract](https://source.android.com/docs/core/architecture/ipc/binder-threading)
separates asynchronous submission from execution and serializes asynchronous
transactions per Binder node. It does not turn a later metadata query into a
service-specific acknowledgement.

## Registration overlap, death and numeric identity

The recovered `RadioExternal` constructor and registration method in
`libril-aidl.so` retain/delegate the injected callback object. Their recovered
bodies contain no mutex around the delegated registration call. The generated
registration dispatch also contains no application lock. This does not prove
that all upstream callers overlap or establish the stock thread-pool schedule.

In `RilExternalManager::onNewClientConnect`, allocation occurs before the manager
mutex. The ID allocator increments a process-local integer with overflow
checking; the recovered instructions do not use an atomic read-modify-write.
Callback installation and list insertion occur under the manager mutex. The
success path then unlocks and calls `onNewCommandConnect`, which submits client
metadata and initial per-slot radio state directly through the client callback.
Thus the list lock alone is not evidence that the complete registration path is
serialized. A reached duplicate-ID race has not been executed here.

The factory-used `RadioExternalCallbackImpl` constructor retains the supplied
upstream proxy and manager. The radioExternal response/indication wrapper
setters and destructors in `libril-aidl.so` retain/release shared callback references. No death recipient is
armed in the recovered constructor/registration/setter closure. That same
library does contain `BinderDeathRecipient`; its recovered direct callers are
other radio and Google RilExt wrappers. The presence of that helper is not proof
of radioExternal death cleanup. This is scoped body/callgraph evidence, not a
global absence claim. Callback submission failure can reach the previously
recovered client clear/removal path; immediate cleanup merely because a client
process died remains unqualified.

The inspected allocation paths do not demonstrate recycled IDs within the
service process, nor bind IDs to a service incarnation, callback endpoint or
caller identity. Restart reuse and concurrent allocation need execution. Never
send delayed cleanup against a successor service using only the saved numeric
ID. Reaping the native Rust process proves local reclamation; it establishes no
remote client deletion, callback drain or modem reset.

## Complete producer boundary

The recovered current AIDL indication and response splitters accept one complete
payload, create successive segments of at most 2,048 bytes, and send zero-based
indices with the original total length. Exact multiples retain a full final
segment. Both loops stop when a segment callback submission reports failure.
This is static native producer evidence; the APK request splitter's independently
executed exact-multiple defect is a different, AP-to-vendor path.

The trace covers both manager broadcast overloads, both manager indication
overloads, raw/explicit segmented client response and indication entry points,
both complete AIDL split loops, the service adapter's raw/segmented callback
wrappers, and the generated native segment proxies.

| Recovered caller boundary | Observed serialization |
|---|---|
| Manager raw broadcast and numeric-ID indication | The manager mutex covers client selection and the downstream client call, including the delegated complete segment loop |
| Manager response delivery | The manager mutex covers lookup and the downstream complete response call |
| Manager pointer indication and supplied-list broadcast overloads | No internal manager lock; caller-held serialization is not established by those bodies |
| Registration initial-output path | Called after list insertion unlock; emits directly through the same retained client indication object |
| Client and service-adapter callback wrappers | No message identity field or whole-message lock is added by the recovered bodies |
| Generated indication segment proxy | Message ID, slot, bytes, segment index and total length; no producer message-instance field |

ARM64 instructions confirm that raw broadcast, numeric-ID indication and clear
use the same manager mutex field; they also confirm unlock before registration
initial output. This supports a serialization claim for the specific locked
paths. It does not prove a complete producer contract across every entry point,
callback alias, concurrent registration, partial-send failure or restart.
Binder's per-node one-way serialization orders submitted transactions; it cannot
create a missing message-instance identity. Keep fragmented semantic publication
disabled until the qualified producer/reset contract covers the whole path.

## Smallest useful execution gates

The next fixture must exercise the identified factory boundary. A replacement
manager or synthetic Binder peer alone cannot establish remote vendor cleanup.
No callable, source-backed current manager fixture was qualified in this pass.

1. **Binder adapter only:** `RadioExternal::make` with controlled implementations
   of its supplied `RadioExternalCallback` and the current V1 protocol can test
   delegation and cross-process flags. This bypasses `RilExternalManager` and
   cannot certify client deletion, producer serialization or pending cleanup.
2. **Actual manager plus producers:** preserve the original factory
   `RadioExternalCallbackImpl`/`RilExternalManager` and AIDL splitters, with verified
   ABI and controlled upstream collaborators in an isolated ARM64 guest. Hold
   registration and complete-message callback submission at barriers; race
   clear, concurrent registration and each producer entry. Inspect actual client
   membership and original callback submissions. Cover failure at every segment,
   initial registration output, aliased callback handles and allocation/reuse.
   Guessed C++ object layouts or rewritten manager bodies are not an oracle.
3. **Pending-work/reset boundary:** include the original separate request list,
   upstream request admission and completion with controlled fake modem input.
   Clear while a request is outstanding, then complete it. A qualified reset
   must establish the exact service/request incarnation being reset and the
   point after which old work can no longer affect a successor. The recovered
   public radioExternal methods currently supply no such receipt.

A fixture may qualify only its executed scope. An isolated callback-manager
experiment can establish client cleanup and producer behavior without proving
modem cancellation. Actual modem acceptance, matched kernel/policy integration,
carrier behavior and a physical-device reset remain later gates. No instruction
here authorizes running the vendor daemon on Harbour or operating a phone.

## Evidence classification

This increment executed REA/Ghidra providers and read-only ELF/partition tools,
not original vendor code. It added no Rust behavior or reference/synthetic
fixture, so no new red-green implementation result is claimed. Existing
[original APK proxy capture](evidence/binder-provenance.json),
[synthetic cross-process indication tests](evidence/radio-indication-provenance.json),
[owned request tests](evidence/radio-request-provenance.json) and
[supervised local reclamation](evidence/radio-supervisor-provenance.json) remain
separate evidence classes and were not rerun for this documentation change.
None executes the factory service adapter identified above.
