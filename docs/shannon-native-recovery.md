# Native Shannon replacement investigation

The user prioritised removing Android userspace on 12 October 2026. The existing
Telecom bridge remains useful test infrastructure, but is not evidence that
Shannon or a native replacement works. No physical phone is accessed.

## Current evidence

Source APK SHA-256:
`8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879`.
The complete dexdump is saved privately under
`artifacts/ims-native-audit/shannon.dexdump.txt` (698542 lines).

- `RilReqStackStartReq.getBytes`, code offset `0x190ce0`, constructs DataWriter,
  writes `mSimStatus` with writeByte(int), then `mStackType` with writeByte(int),
  and returns toByteArray. This establishes field order, not valid SIM states.
- The message enum initializer supplies 137 for AIMS_STACK_START_REQ and 138
  for AIMS_STACK_STOP_REQ. Their enum ordinals are separate values (52, 53).
- `OemSlsiRadioExternalProxyAidl` obtains the declared vendor radioExternal
  service at DESCRIPTOR + `/default`, installs response/indication callbacks,
  and forwards raw and segmented byte-array requests.
- `RilReqSetImsRegistrationState`, `RegistrationAdaptor` and
  `RilIndRegistration` provide concrete next locations to trace. Their names
  alone do not establish the direction or ownership of IMS registration.

## Original-code oracle

`bash tools/framework-suite.sh shannon-codec` starts the owned Android emulator
and builds a small reflection caller. It loads the original APK's DataWriter
class and invokes writeByte(int), writeShort(int) and writeInt(int) on 15 signed
boundary inputs each. No Shannon service is installed or started; no vendor
radio service is contacted. The additional proxy capture test is described below. The APK hash and SELinux enforcing state are checked.

The initial run exited 0 and executed all 45 reference cases. Evidence is
`artifacts/ims-native-audit/original-codec-output.txt`. The caller supplies no
implementation of the serializer. These original-code outputs are now available as independent Rust comparison
fixtures. The Rust comparisons are recorded below. Primitive serializer
agreement does not establish IMS registration or full protocol compatibility.

## First native codec result

The original stack-start and registration-state getBytes methods executed in
Android's runtime with a null channel; no send method was called. All 162 cases
completed. `native-ims` implements their payload encoders and three scalar
writers. Tests first failed for absent implementations, then passed against 207
original-code cases. Local clippy and format checks pass. See
`artifacts/ims-native-audit/codec-provenance.json` for source and fixture hashes.
These functions preserve low-bit truncation and little-endian integer order.
They do not establish valid state/RAT enums, request sequencing, modem
registration, audio, or the ability to remove the vendor radio service.

## REA registration ownership trace

REA 6.4.0 with the pinned JADX headless provider 0.7.1 successfully inspected the
same saved APK on 12 October 2026. Provider SHA-256:
`6e5eacf500b64292bfb73c49797c1958f6ee44646e43e868039ae7feb573ff75`.
The provider loaded 1,320 classes. This is static evidence; it executes neither
the APK nor a modem request.

The recovered incoming-reference chain is:

```mermaid
flowchart LR
  Connectivity[ImsConnectivityMgr] --> Stack[StackControl: selected SIM slot]
  Stack --> Adapter[RegistrationAdaptor]
  Adapter --> Request[RilReqSetImsRegistrationState]
  Request --> Network[Network sendRequest]
```

`ImsConnectivityMgr.sendRegistrationState` selects a connection value of zero
when the supplied boolean is false. When true, it uses one for CellConnectivity
and two for other RatConnectivity objects. APN_IMS and APN_RCS_INTERNET follow the
normal registration-state path; APN_EMERGENCY follows sendEimsRegistrationState.
Other APN types are not forwarded by this method. These numbers are local wire
values, not evidence that Android's public radio-technology enums can be reused.

StackControl selects the slot's RegistrationAdaptor. The adapter converts the
boolean to zero or one, sets the supplied connection value, and submits the
request to its network object. This supports treating the existing encoder as
state reporting within a larger connectivity path, not as a complete command to
register with a carrier. The source of the boolean, preceding transitions,
request/response transport and modem-side behavior still need recovery and
original-code comparison.

Local raw REA results remain in `artifacts/ims-native-audit/rea-*.json`; they are
excluded from the public source. Decompiled bodies are not copied into this
report. Tool-reported complete static references do not include proof of dynamic
or reflection-driven relationships.

### State owner and duplicate transitions

The next recovered caller is `ApnEntity.notifyServiceState`, reached from
`ApnEntity.setServiceState`. The core three-argument overload updates radio
technology when the requested state is IMS_REGISTERING, before comparing the
old and new state. Equal states return without assigning state or calling the
notification method. Different states are stored and then notified. Thus a
native implementation must not discard all repeated-state work before the
radio update.

The notification method suppresses the registering notification and callbacks
when its initialization type indicates handover (value one). Other branches
select registration reporting and then invoke registered callbacks with
per-callback exception handling. JADX reports duplicated-block warnings for
this method; branch details need DEX cross-checking and runtime comparison
before being treated as a complete behavioral contract.

REA cannot trace a single overloaded method name: setServiceState has four
signatures. A DEX instruction scan finds direct callers in ImsCallManager,
ApnManager.disableApn, ImsConnectivityMgr.setCurrentServiceState overloads,
and StackControl.sendSelfCapability, plus delegating overloads in ApnEntity.
This is an instruction-level inventory, not evidence that all those paths run
in a particular registration sequence.

Next verification boundary: original APN state transitions with controlled
collaborators, preserving notification order, duplicate suppression, handover
and callback failure behavior. The current serializer oracle does not cover
these behaviors. Do not implement guessed carrier registration transitions
from this partial call graph.

## Original Binder proxy capture

The `shannon-codec` suite now also runs `BinderOracle` in the owned emulator.
REA identified `IOemSlsiRadioExternal.Stub.Proxy.sendRequestRaw` as transaction
code 2, one-way, with an interface token, four integers in argument order and a
byte array. Original-code execution checks these facts against a local Binder
capture object. The oracle checks that `Stub.asInterface` actually selects the
original generated proxy; it does not look up or contact a vendor service.

Five cases cover signed integer boundaries, distinct field values, null versus
empty byte arrays, high-bit bytes and a 257-byte sequence. The endpoint checks
transaction code, flags, absence of a reply parcel, token, field order, payload
and absence of trailing fields. A sixth case returns false from the endpoint;
the original proxy throws RemoteException without retrying. All six passed on
Harbour alongside the previous 207 serializer cases. The new harness first
failed before the oracle class was implemented; the completed suite exited zero.

[Public provenance](evidence/binder-provenance.json) records source and output
hashes plus the small generated result transcript. Raw analysis and emulator
logs remain in ignored `artifacts/ims-native-audit/` files.

This is an **in-process capture**, not a Binder-driver or vendor-service test.
It does not establish argument meanings, service permissions, callback lifetime,
segmentation, cross-process behavior, modem responses or carrier registration.
No native Rust transport is claimed. The next transport boundary requires
recovering callbacks and segmented requests, then testing a Rust client through
an actual cross-process Binder endpoint before involving vendor hardware.

### Segmented proxy and caller boundary

The same oracle now exercises the original `sendRequestRawSegment` proxy.
It uses one-way transaction 3: interface token, four integer fields, byte array,
then two integer fields. Five capture cases include null/empty arrays and signed
boundary values for the two trailing fields; another case checks RemoteException
on a rejected transaction without retry. These are proxy serialization checks,
not validation that these inputs are valid radio requests.

REA and DEX inspection of `OemRIL.sendMultiFrame` identify the first four fields
as request serial, client ID, request ID and instance ID. The trailing fields
are the zero-based segment index and the complete payload length. This field
meaning is static caller evidence, separate from the executed proxy checks.

**Initial static boundary finding (runtime follow-up below):** `sendMultiFrame` only enters its loop for payloads
larger than 2,048 bytes. It computes the last segment length using remaining
bytes AND 2,047. The DEX has this literal mask at code-unit offset `0x0048`
(method code offset `0x19426c`), consistent with REA. For a 4,096-byte input,
this implies a 2,048-byte first segment followed by an empty segment, leaving
2,048 bytes unsent by this method. This is a static finding, not a reproduced
runtime defect or proof of a reachable carrier failure. Before implementing
Rust segmentation, execute this original method with controlled collaborators
and inspect its dispatch caller to determine reachable lengths. Do not silently
copy the mask into a native implementation or assume a fix is protocol-safe.

### Runtime confirmation of the boundary defect

`SegmentationOracle` now executes the original `OemRIL.sendMultiFrame`, original
AIDL adapter, and original generated Binder proxy against a local capture endpoint.
It bypasses constructors with a test-only allocator and injects the request map,
client/instance identifiers, original DataWriter and the original proxy connection.
This avoids service startup; it does not verify constructor or lifecycle behavior.
The capture validates all routing fields, zero-based indices, full-length metadata,
and each delivered byte against the corresponding input position.

| Input bytes | Captured segment lengths | Delivered bytes |
|---:|---|---:|
| 2049 | 2048, 1 | 2049 |
| 4095 | 2048, 2047 | 4095 |
| 4096 | 2048, 0 | 2048 |
| 4097 | 2048, 2048, 1 | 4097 |
| 6144 | 2048, 2048, 0 | 4096 |

All five observations were reproduced by original-code execution in the owned
Harbour emulator. [Provenance and transcript](evidence/segmentation-provenance.json)
record the test sources and output hashes. The existing 207 serializer cases
and 12 proxy cases also passed in this run.

REA and DEX inspection of `invokeRequestRaw` confirm that input lengths greater
than 2,048 select this method without an exact-multiple exclusion. This establishes
the dispatch condition statically; the runtime oracle invokes sendMultiFrame
directly. It does not establish that a real IMS request has either affected size,
that the vendor service accepts these requests, or that a carrier failure results.

The native implementation must preserve the complete payload. These observations
are regression evidence about the original implementation, not a requirement to
reproduce its data loss. A corrected splitter still needs vendor-endpoint and
hardware validation before protocol compatibility can be claimed.

## Native lossless request splitting

`native-ims::requests::RequestParts` now selects a single raw request for payloads
of zero through 2,048 bytes and indexed segments for larger payloads. It borrows
the input bytes, preserves the complete payload and carries the original signed
full length on each segment. Lengths outside the signed 32-bit field are rejected
before iteration. Null payloads are not represented by this API.

The Rust code intentionally differs from the original at exact multiples: 4,096
bytes become two complete 2,048-byte segments, not a full segment and an empty
one. Tests compare the observed non-multiple boundaries and require lossless
behavior at the defective boundaries. A reconstruction test checks every length
from 2,049 through 16,385, including indices, total lengths, content, segment
bounds, borrowed storage and iterator exhaustion. Raw empty/threshold cases and
signed length overflow are checked separately.

This is a native request planner, not a Binder transport. It does not implement
service discovery, callback handling, request correlation, retries or vendor
acceptance. Those must be verified through the production transport before any
claim of working native radio communication.

## Vendor callback contract: static recovery

REA inspection of the original generated interface establishes these next
transport contracts. They are static findings, not executed vendor callbacks:

- `setResponseFunctions` uses synchronous transaction 5. Its parcel contains
  the interface token, response Binder reference, indication Binder reference,
  and client-name string. It reads the remote exception header and then an int.
  The int's application meaning still needs caller tracing and runtime checks.
- Response callback transaction 1 reads a typed RadioExternalResponseInfo and
  a byte array. Transaction 2 adds two integer fields after that array. The
  original stub enforces no trailing data before invoking the implementation.
- RadioExternalResponseInfo writes a size-prefixed parcelable containing five
  ints in order: slotId, serial, rilcMsgId, length, error. The prefix includes
  itself in the byte count. Nullability, shortened parcelables, response length
  semantics and error values require further tests; names alone do not define
  a safe production parser.

The original proxy and stub analysis is retained privately in
`artifacts/ims-native-audit/rea-register-callbacks.json`,
`rea-response-on-transact.json` and `rea-response-info-write.json`.
The native service-manager callback test in [native-binder.md](native-binder.md)
qualifies a general Binder primitive, not these vendor-specific methods.

### Executed response metadata fixtures

`ResponseInfoOracle` uses Android Parcel.writeTypedObject with the original
RadioExternalResponseInfo class. Seven rotated signed-boundary input rows produce
28-byte fixtures: a four-byte presence marker plus a 24-byte size-prefixed
parcelable. Those original bytes are checked in at
`native-radio/tests/fixtures/response-info.tsv`; the factory APK is not included.

Generated Rust AIDL bindings serialize to exactly those bytes and deserialize
all five fields from them. The tests also reject selected invalid sizes and
trailing data. This validates metadata encoding independently of the vendor
service. It does not establish response status meanings, length consistency with
payloads, request matching, segmented response assembly or carrier operation.
