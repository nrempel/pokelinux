# Original conditional registration admission

A fresh owned SDK37 x86 run completes the original conditional connected path at
slot1: four pure admission combinations and three complete connected cases, with
10 records and15 verified original method declarations/classloader bindings.
This is original-reference evidence under explicit authored priors. It adds no
Rust, ARM, device or UI result; the prior59 host tests were unchanged and not rerun.
No behavioral RED was intended.

## Selected graph and observed results

The [authored driver](../framework/shannon-registration-oracle/connected/runtime-src/org/omarchy/audit/RegistrationConnectedOracle.java)
and its two peripheral subclasses use explicit constructor bypass for the owner,
listeners, StackControl, optional entity and network/adaptor objects. Original
ApnManager, CellConnectivity and registration-state request constructors execute;
original class initialization may execute. Original admission, both connected
overloads, state setters, manager propagation and inherited predicates remain
unmodified. Virtual RCS/trigger and call-manager effects are captured at explicit
cuts; the helper accepts only the exact request and never forwards it.

The graph binds APN_IMS and selected slot1, optional live entity, and absent or
exact local StackControl membership APN_IMS→(1,0). This membership is an authored
prior. Source-only recovery shows local membership can be installed before send;
it is not remote request acceptance. Retry/handover flags start false, RAT/PSFB
are UNKNOWN, observer lists empty, and no WiFi entity exists.

| Case | Original result |
| --- | --- |
| Pure admission: entity/request00,01,10,11 | false,false,false,true; captured state unchanged |
| Connected with no entity or no membership | Normal full completion, unchanged state and no request/trigger/call continuation |
| Connected with both priors | Entity and manager stored/direct state become IMS_REGISTERED; inherited cellular/IMS predicates true and WiFi false |

The admitted case completes one captured REG_CONNECT trigger and one captured
APN_IMS call update, with no handover effect. It constructs one original
RILC_REQ_SET_IMS_REGISTRATION_STATE request: enum code48, state1, RAT1. The helper
captures it without forwarding, serialization or modem submission. All cases have
normal completion, null escaped/sticky capture failures and restored singleton
bindings. Partial state mutation would not qualify completion.

Even in the admitted case, original CellConnectivity.isConnected remains false,
its marker remains0 and mIsAddPdnReady remains false. These conditional stored
state/predicate observations do not establish acquired APN, live IMS connectivity,
subscriber admission, request success or remote cleanup.

## Build identity and owned closure

The [six authored Java files](../framework/shannon-registration-oracle/connected/)
are separated into three runtime sources and three compile-only API stubs.
Separate javac outputs, empty sourcepath and explicit inputs prevent implicit
stub compilation into runtime. D8 receives all five runtime classes only. Its
seven actual class definitions comprise those five, Graph$0 desugaring and the
exact checked no-code LambdaMethod annotation. All four compiled API stub classes
are absent from runtime DEX and CLASSPATH. Original target definitions therefore
come from the matched APK, with15 actual declaration/loader identity checks.

Root first preserved a failed overly strict guard against prior real D8 output,
then qualified only that exact non-executable compiler annotation and refused
eight malformed/foreign/executable-marker/stub controls. These pure helper checks
are compiler-output qualification, not a production behavioral RED.

Original SolicitedMessage initialization starts its static HandlerThread. The
run retains its exact static field, Looper, Java thread and Linux TID identity.
Quit request to actual join return takes57.587042ms; native task absence is
observed1.125622ms later, before the original fixed25s cleanup cutoff. The20s
case/25s cleanup cutoffs never renew, and original infrastructure closure is
required alongside full connected completion.

The shell actually waits0 for app_process; ADB waits0 with bounded EOF. All110
direct container children close, emulator waits0 and the retained foreground ADB
server waits SIGTERM15. Owned Docker CLI wait0/EOF, fresh exact CID absence, same
daemon and unchanged source/SDK/APK inputs close the outer run. A fresh HOME/AVD,
read-only447 SDK/input pins, networknone and only KVM are used. There is no APK
installation, host ADB, real service/modem, auth/PAM, phone or GUI operation.
The [bounded provenance](evidence/native-ims-registration-admission-provenance.json)
pins exact sources, results and closure; private APK, raw records and audits stay
outside Git.

## Next native boundary

Next is a private conditional RegistrationObserved reducer plus deliberate native
Unavailable on disconnected, unknown, expired or dead input, retaining history.
That conservative policy would not assert deregistration, APN removal, remote
cleanup or retry authority. It is planned, not implemented by this checkpoint.
Authored readiness priors remain private test inputs; no production assume-ready
configuration is permitted. Full native APN ownership/acquisition is a production
gate. Original asynchronous loss/retry and the add-PDN-ready early return remain
unexecuted; operations depending on them need their own qualification.

The earlier [message notifications](native-ims-registration-notifications.md) and
[installed wire history](native-radio-registration-installed.md) retain their
separate scopes. These runs do not compose a full original wire-to-manager chain,
carrier/IMS operation or matched hardware.
