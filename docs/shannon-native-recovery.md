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
