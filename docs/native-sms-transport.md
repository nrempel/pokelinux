# Native SMS transport recovery

The selected Pixel factory image declares standard `android.hardware.radio.messaging`
V3 endpoints `IRadioMessaging/slot1` and `IRadioMessaging/slot2`, alongside Samsung
`radioExternal/default`. The standard SMS endpoint is the first qualification
candidate for a native backend. The selected manifest has no `android.hardware.radio.ims`
declaration. A declaration does not establish running service availability,
permissions, actual hash agreement, carrier support or replacement of vendor userspace.

The public Android17 frozen V3 hash is
`30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb`. The source tag is
`android-17.0.0_r1`; its frozen ABI and documented semantics were fetched separately.
See the [frozen interface](https://android.googlesource.com/platform/hardware/interfaces/+/android-17.0.0_r1/radio/aidl/aidl_api/android.hardware.radio.messaging/3/android/hardware/radio/messaging/IRadioMessaging.aidl)
and [provenance](evidence/native-sms-transport-provenance.json). A device-neutral submission owner now connects the durable outbox to the
synthetic standard-V3 Binder boundary described below. No production Pixel
service, receive or acknowledgement adapter is connected.

## Standard SMS boundary

| Boundary | Actual fields and meaning | Qualification limit |
|---|---|---|
| `sendSms(serial, GsmSmsMessage)` | Signed request serial; SMSC hex string with length-prefixed BCD, or empty for default SMSC; TPDU ASCII hex excluding SMSC | One-way Binder completion does not establish submission. Slot belongs to the selected endpoint, not the PDU. |
| `sendSmsResponse(info, result)` | Response type, serial and radio error; result has message reference, acknowledgement PDU and network error code | Correlate method, serial, callback instance, endpoint and owner generation. A successful final send result is distinct from carrier delivery. |
| `acknowledgeRequest(serial)` | Intermediate request receipt acknowledgement | Cannot finish an outbox permit or prove delivery. |
| `newSms(type, pdu)` | Indication type and complete SMSC-prefixed SMS-DELIVER bytes | No receive serial, provider token or durable message identity. |
| `newSmsStatusReport(type, pdu)` | Indication type and complete SMSC-prefixed SMS-STATUS-REPORT bytes | Separate parser and delivery correlation required. The current SMS-DELIVER decoder does not parse status reports. |
| `acknowledgeLastIncomingGsmSms(serial, success, cause)` | A new command serial; receipt result and failure cause for the last incoming GSM item | Serial correlates the acknowledgement command. It does not identify the received message. |
| `responseAcknowledgement()` | Receipt acknowledgement for ACK_EXP radio callbacks | Separate from GSM network receipt acknowledgement and final send result. |

The [public indication contract](https://android.googlesource.com/platform/hardware/interfaces/+/android-17.0.0_r1/radio/aidl/android/hardware/radio/messaging/IRadioMessagingIndication.aidl)
requires the server to hold subsequent SMS and status-report indications until
last-incoming acknowledgement. This establishes the expected ordering contract,
not observed factory behavior. Durable receipt admission must precede positive
network acknowledgement. Replaying an uncertain acknowledgement after reconnection
could acknowledge a later item; no idempotence or safe reset guarantee has been
qualified. Equal bytes, body hashes and concatenation references cannot establish
that two arrivals are the same network message.

`setResponseFunctions` is one-way in the frozen interface. Callback replacement,
death, quiescence and ownership need real-boundary tests. The API also permits
colliding serials to produce separate responses, so server serial handling must
not be treated as durable deduplication. A native client should avoid outstanding
collisions and fence late callbacks by endpoint and owner generation.

The standard HAL also exposes `sendImsSms` with technology family, retry flag,
message reference and GSM/CDMA arrays. That schema does not establish an operational
IMS stack. Public IMS-provider APIs use separate operation tokens and separate
send-result, receive and status-report callbacks. The legacy radio-network IMS
registration query is deprecated from Android U and reports registration/family;
it is not a registration command. The inspected Shannon normal GSM path additionally
checks SMS-over-IP provisioning. Carrier registration and current availability remain
separate work.

## Shannon original-code observations

`SmsTransportOracle.java` executes the selected original APK in the owned SDK37
x86_64 emulator. Request constructors receive a null channel; no send method,
service lookup, Shannon service startup or vendor radio endpoint is used. `ImsSms`
constructors are bypassed and the local tracker map/logger are injected. These
checks exercise codecs and controlled local state, not component startup or modem behavior.

| Original boundary | Executed result |
|---|---|
| GSM request `getBytes` | 260 bytes: length byte + fixed12 SMSC area; length byte + fixed244 payload area; SMS-type byte; geolocation byte. Short areas zero-pad; oversized declared lengths cap at12/244. Changing token leaves payload unchanged. Twenty boundary cases pass. |
| GSM acknowledgement `getBytes` | Little-endian int result, low byte of vendor TPID, little-endian int cause: nine bytes. Five signed-boundary cases pass. |
| GSM response `update` | Int message reference, signed acknowledgement-length byte, fixed244 acknowledgement area, int error. Five cases pass; length128 becomes -128 and244 becomes -12. A native parser must validate bounds before using this field. |
| Incoming and status wrapper `update` | Unsigned byte TPID, unsigned byte length capped244, fixed244 payload area and optional verification byte. Sixteen cases pass. This does not independently establish the payload's end-to-end SMSC framing. |
| Tracker reuse/collision | Reusing a token replaces its tracker. Two trackers with the same message reference make the reference lookup return one of them. No durable unique identity is established. |
| Report acknowledgement failure | The original method removes its tracker before reaching the controlled null adaptor. The first attempt fails there; repeating it finds no tracker and returns without a local retry. This does not prove remote acknowledgement success or idempotence. |
| Discard acknowledgement | The controlled discard branch removes its tracker and skips the adaptor. |

Original enumeration confirms GSM send request121, GSM acknowledgement request123,
new-SMS indication3016 and status-report indication3017. These are vendor wire IDs,
not shared service identifiers. Android operation token, radio serial, returned
message reference and receive TPID have separate scopes. The incoming Shannon
callback generates a local key and stores the vendor TPID separately; no persistence
or restart uniqueness is qualified.

The initial launcher failed at runtime before the oracle implementation existed.
The first implemented run passed46 codec cases, then the fixture supplied `HashMap`
where the original field requires `ConcurrentHashMap`. Correcting the collaborator
produced a complete green run:50 codec/tracker outcomes and8 enum observations.
Run with `bash tools/framework-suite.sh sms-transport-oracle`. This original-code oracle introduces no native encoder or production
transport changes and has no ARM64 or physical evidence.

## Shipped native paths and remaining gates

REA static dossiers locate the standard send/ack forwarders and callback hooks in
`libril_sitril.so`. Publication calls declared-service checks and `addService`;
callback setup retains response/indication references. Send and acknowledgement
forwarders delegate through indirect targets. The core `libsitril.so` SMS service
calls SMS protocol builders, extracts response error/reference fields and uses
error mappers. Incoming messages have class/SMSC/TPDU processing and delayed work;
status-report processing is separate from send completion. Focused value-flow
records are retained privately. Indirect calls, external libraries, firmware,
reset and lifecycle behavior remain unresolved; no shipped native target was executed.

The production Pixel outbox path remains unconnected. A qualified successful final send callback
may support `TransportAccepted`; Binder completion, an intermediate request ACK,
a timeout or arbitrary error cannot. Public retry suggestions do not authorize
implicit retries of uncertain parts. Negative submission proof needs an independently
qualified adapter mapping. Delivery needs a separately parsed status report and
unambiguous per-slot/per-attempt association. Current submitted TPDUs use TP-MR zero,
so reference collisions must be resolved before delivery correlation is promised.

## Executed synthetic V3 Binder boundary

`native-messaging::transport::SubmissionOwner` owns the outbox and non-cloneable
claimed permits. It uses opaque endpoint/source/attempt identities. Backend wire
serials, slot selection, callback methods and parcels are confined to the separate
`native-sms-boundary` test crate. Claims persist `Uncertain` before exposing payloads;
one-way return and intermediate request acknowledgement leave that state unchanged.
Only a qualified final send-success event consumes a permit and stores
`TransportAccepted`. No negative submission classification or retry is added.

Cancellation persists first, then removes pending permits. Revocation, source
replacement, callback queue failure and service death remove local pending
ownership without refunding durable uncertainty. Late consumed events must still
pass the active source/attempt check. SQLite acceptance failure produces no
accepted receipt. Admission is bounded to8 endpoints and32 pending attempts; the
synthetic adapter additionally bounds its callback queue to64 and each session's
wire bindings to32.

The independent NDK peer uses the original Android37 AIDL compiler and exact frozen
public messaging3/radio3 schemas. Its generated dispatcher and response proxy
interoperate with generated Rust bindings across the actual emulator Binder driver.
Java only bootstraps the disposable instrumentation endpoint. The Rust client checks
the synthetic peer's V3 version/hash and observes its captured serial, SMSC and TPDU.
A separate control interface releases callbacks and barriers; it is test infrastructure,
not a radio API or production daemon.

The first live run failed at the intended assertion: one-way send return had stored
`TransportAccepted` where `Uncertain` was required. A second live regression exposed
an odd-length acknowledgement (`f`) consuming a permit. Qualification now requires
whole octets, and malformed final metadata leaves the permit available for a later
valid response. The final live green passed 10 scenario groups: frozen bootstrap;
durable-before-IPC and one-way return; intermediate ACK; malformed acknowledgement;
matching final response versus wrong serial/method/slot/type/reference and duplicate;
consumed and later callbacks after cancellation; same wire serial on a replacement
callback generation; terminal error without negative submission proof; consumed
callback after actual peer death; and SQLite reopen preserving uncertainty. Seven
additional actual SQLite ownership tests pass on x86 Linux and ARM64 user-QEMU.
The full messaging regression, formatting and clippy checks pass.

Run `python3 tools/prepare-sms-boundary.py`,
`bash tools/build-native-sms-boundary.sh`, then
`bash tools/framework-suite.sh sms-binder` on the prepared host.
[Provenance](evidence/native-sms-binder-provenance.json) records exact input/source,
binary and red/green hashes. This demonstrates synthetic transport qualification;
it does not establish real modem acceptance, delivery, ARM64 Binder or installed
Messages/background integration. The test adapter currently qualifies only a
SOLICITED `sendSmsResponse` with NONE radio error, a GSM reference in0..255,
bounded even-length ASCII hexadecimal acknowledgement data and network errorCode -1. It is not a
complete modem/error-policy adapter.

The next gates are factory runtime version/hash/permission and callback ownership,
qualified modem send/error mapping, bounded registration/reset and producer lifetime,
status-report parsing/correlation and installed native messaging integration.
A receive/acknowledgement owner was not inferred: standard callbacks expose no
durable receive identity, and last-incoming acknowledgement replay after reconnect
remains unqualified. Original-code or synthetic endpoint tests cannot substitute
for those modem guarantees.

Further original-code tests should exercise normal send-result error mapping with
controlled listeners, bounded malformed codec inputs, status-report framing and
colliding/delayed message references. Actual modem acknowledgement idempotence,
receive redelivery identity, carrier IMS registration, matched native dependencies,
permissions, background wake and later authorized hardware verification remain gates.
No physical phone was accessed. The owned REA sessions were explicitly closed,
exited, reaped and removed; the emulator owner was released after the green run.
