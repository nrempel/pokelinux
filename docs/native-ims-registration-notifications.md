# Original registration message notifications

A fresh owned SDK37 x86 run executes original `RilIndRegistration.update` and
its direct `onMessage` to a capture-only connectivity listener. All14 records
complete:12 full packets across slots0/1,10 original data instances and20 captured
callback entries. Outcomes were recorded without predicted callback count,
order, enum mapping or field-mutation assertions. No behavioral RED was intended.

This adds original-code reference evidence. It changes no Rust component,
installed IPC or packaging and adds no Rust test count, ARM or hardware result.
The separate [raw route](native-ims-registration-routing.md), [decoder](native-ims-registration.md),
[native owner](native-radio-registration-owner.md) and [installed slot1 history](native-radio-registration-installed.md)
retain their earlier scopes and counts. Current Rust still exposes historical
wire observations, without native IMS or phone-state authority.

## Selected boundary

The [authored oracle](../framework/shannon-registration-oracle/RegistrationSemanticOracle.java)
is pinned at SHA256 `24809dfbd7deb00c360c8421c0a5f49aaa2cb624b214d36641dd7f6a15544086`.
Original data/base constructors execute normally. Original `NetworkAdaptor` and
`RegistrationAdaptor` objects use explicit constructor bypass, with inert
network/listener injection and original identity/`getSlotId` checks. Their
constructors do not run; original class initializers may execute. Reflection,
hidden-API or bypass failure cannot qualify the boundary merely because the SDK
booted. The actual run passes separate boot, original-seam and complete-boundary
qualification checks.

Each call supplies the instance's actual `getMessageId()` enum, whose code is3012.
This is not a new message-ID filter test. The original `AdaptorChannel.onMessage`,
unsolicited-table dispatch, helper/OemRIL, concrete connectivity listener,
APN/StackControl manager, native-load and real service/modem paths are excluded.
No ordinary adaptor construction, full channel integration or remote state
reduction follows from this execution.

## Observed finite cases

Each slot0/1 has one reused message updated2→1, plus fresh messages updated1,2,
255 and3. Every selected array is exactly12 little-endian bytes: state followed
by feature0, mask0, status0, URI/reason lengths0, with no extensions. Before-update,
after-update and after-onMessage fields, exact overload descriptors, ordered typed
arguments and exceptions are retained privately. Both escaped exceptions and
sticky Proxy failures are null; outside-onMessage callback count is0.

| Input state in these packets | Actual ordered Proxy callbacks, at both configured slots |
| --- | --- |
|2, reused first update and fresh | `registrationAssociatedUriChanged` with one URI string `UNKNOWN`; two-argument `registrationConnected(APN_IMS, IMS_SERVICE_REGISTER_SUCCESS)` with CP reason0; capability callback with RegState code2 |
|1, reused second update and fresh | `registrationDisconnected(APN_IMS, IMS_SERVICE_REGISTER_SUCCESS, 0, "UNKNOWN")` with CP reason0; capability callback with RegState code1 |
|3 and255, fresh | No callback entries |

The capability callback's feature/mask/extension integers and long value are0
for these packets. Its observed enum names are
`REG_STATE_VOLTE_SINGLE_REGISTERED`/`REG_STATE_VOLTE_SINGLE_NOT_REGISTERED` with
actual codes2/1. `APN_IMS` has no exposed `getCode`; no ordinal becomes a wire or
APN identifier. The same success-named reason/CP0 appears in Disconnected, so it
cannot establish operation success or manager acceptance.

`UNKNOWN` is the original empty-text sentinel observed through Uri/string
conversion, without subscriber identity meaning. Native wire storage still
retains empty bytes. Callback traces match across these two configured channel
slots; the callback signatures themselves contain no slot argument. Injected
slot provenance is not a new upstream filtering proof. These observations do not
cover nonzero features, nonempty URI/reason text, extensions, other reasons or
broad state-domain behavior. Zero callbacks for3/255 does not define withdrawal
or default-state policy.

## Closure and evidence

The source, original APK and447 SDK file pins are read-only. A fresh private
HOME/AVD holds writes, with networknone and only `/dev/kvm`. Metadata reports
SDK37/x86_64, QEMU, SELinux Enforcing,1080×2424/density420. This headless run is not
visual or physical-density qualification. All generated Java classes enter D8;
the exact DEX jar and APK are rehashed after transfer, then one fixed
`app_process` runs without runtime arguments.

Its shell parent actually waits0 and records subsequent process absence; that
source-backed receipt is not syscall tracing. The owned adb invocation waits0
with bounded EOF. All109 direct container children close within their original
cutoffs; the foreground emulator waits0 and retained adb server waits SIGTERM15.
The owned Docker CLI separately waits0/EOF, then fresh exact container-ID absence,
same daemon and unchanged source/SDK/original-input hashes close the outer run.
No phone, APK installation, authentication/PAM, modem or native Rust execution is
selected. The [bounded provenance](evidence/native-ims-registration-notifications-provenance.json)
pins the source, actual records and closure; private raw audit files and APK stay
outside Git.

## Concrete reducer remains the next gate

Source-only recovery identifies an original manager guard requiring both a live
APN entity and StackControl APN membership before the connected listener can
advance service state. That static prerequisite is not runtime admission proof.
The complete conditional manager connected and loss paths, prior-request controls,
entity/manager state and predicates, APN/subscription/slot association, collaborator
effects and withdrawal behavior still need original execution before a shared
native phone-state model is justified. A direct setter or isolated guard result
would not complete those listener paths. No concrete reducer execution is added
by this capture Proxy.

Further source-only REA identifies an add-PDN-ready guard that can return from
the observed reason0 disconnected path before state withdrawal. The next oracle
must make that prior explicit. Preserving original request construction also
starts an original static HandlerThread; its exact quit/join and fixed cleanup
cutoff need their own receipt. Neither path executes in this notification probe.

The pinned [AOSP registration API](https://android.googlesource.com/platform/frameworks/base/+/a890d7b328dc436999d7de4f8a1dc3337b5c7944/telephony/java/android/telephony/ims/stub/ImsRegistrationImplBase.java)
separates framework registration callbacks, cached registration/URI observations
and capability notification responsibilities. This is public architecture
reference only, without a matching Shannon revision or validated wire-state
mapping. Original producer/reset, acknowledged remote cleanup, safe successor
registration, semantic IMS/calls/media, carrier behavior and matched hardware
remain unqualified.
