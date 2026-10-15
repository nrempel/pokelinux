# Original registration-indication routing

The retained original APK now qualifies the narrow raw-indication route needed
by the native registration decoder. In a fresh SDK37 x86 emulator,17 raw dispatch
cases and8 separately selected parser calls complete. Original
`RilIndRegistration.getMessageId().getCode()` returns3012 for
`RILC_UNSOL_AIMS_REGISTRATION`; observed enum ordinal106 is not a wire identifier.
This adds original-reference evidence. Native publication remains the next gate.

## Executed boundary

The [authored oracle](../framework/shannon-registration-oracle/RegistrationRouteOracle.java)
constructs the original AIDL indication adapter with a configured slot and a
captured listener. It invokes the original generated Stub's `onTransact` code1
in-process. The original concrete callback delivers `OemRilResult` to that
listener; the capture stops there. The supplied flags value1 is authored input,
not a recovered producer flag or a Binder-driver interoperability result.

Separately, a fresh original registration message with a null adaptor channel
provides its ID association and parses the entire captured byte array. No
`OemRIL`, helper/channel dispatch or semantic `onMessage` executes. The probe
never constructs `NetworkAdaptor`, installs the APK, activates a vendor service
or accesses a modem or physical phone.

| Control | Actual original observation |
| --- | --- |
| Complete minimal/extended packets, configured slots0/1 | Exactly one capture, exact message and slot, every byte retained, reported length equals full array length |
| Swapped slots and negative incoming slot | Zero listener entries |
| Adjacent3011/3013 and unknown message IDs | Forwarded by the generic adapter; registration-ID selection must be explicit in the native backend |
| Empty, truncated header/body and one-byte trailing arrays | Delivered intact to the listener; all five separate original parser calls return normally, with default/partial or retained populated fields |
| Null array with matching slot | `NullPointerException` before listener entry |
| Null array with wrong slot | Filtered without listener entry |
| Extra parcel integer | `BadParcelableException` before listener entry |

The eleven delivered records report type2, serial−1 and status0. These are
observed metadata values; no IMS registration or operation-success meaning is
assigned. Three complete parser cases preserve state255, feature128, signed
mask−2147483648 and status65535. Root also checks all recorded URI/reason counts,
present extension fields and authored text for those complete cases. Empty
original text uses its `UNKNOWN` sentinel; the native decoder continues to expose
empty bytes and absent optional fields instead.

## Native policy and remaining integration

The existing [whole-packet Rust decoder](native-ims-registration.md) remains
unchanged. Its earlier121 original comparisons and24 tests per x86/ARM64
user-QEMU target are not rerun or enlarged by this checkpoint. Truncated/trailing
payload refusal is deliberate native policy; original normal return does not
make a partial packet valid. Unknown wire values remain opaque.

The next actual ARM Binder test will prove that an exact raw3012 indication for
an explicitly configured slot reaches the production owner's PreparedEvent,
then require complete typed historical publication. Current production accepts
raw responses and returns `Unsupported` for indications. Decoding must precede
revalidation of current session, source revision, preparation token, terminal
state and suspend-aware deadline. Fragments must not enter this complete-array
path. The installed service will retain one revision-tagged diagnostic observation,
with peer loss withdrawing availability independently of retained history.

Full original helper/channel routing, producer identity, fragment serialization,
reset/cleanup acknowledgement, safe successor registration, state/RAT/features,
URI identity, native IMS/calls/media and physical hardware remain unqualified.
There is no new Rust runtime, ARM kernel runtime, screenshot or phone result here.

## Runtime and closure

The private original APK, authored source and selected SDK files are hash-pinned.
SDK/source/APK inputs are read-only; a new private HOME and AVD hold all writes.
The owned container has no network and receives only `/dev/kvm` as an explicit
host device. Its emulator reports SDK37, x86_64, QEMU, SELinux Enforcing,
1080×2424 and logical density420. This is headless reference execution, not visual
qualification or measured physical pixel density.

Compilation uses the SDK37 `android.jar`, javac source/target8 and D8 min-api31
on every generated class, including inner classes. The resulting DEX jar and
original APK are separately hashed after transfer. One fixed `app_process`
invocation uses that exact classpath and no runtime arguments. Its shell parent
performs `wait` and records exit0 and subsequent process absence; this is a
source-backed actual wait receipt, not syscall tracing. The owned adb command
also waits0 with bounded output and EOF before its original cutoff.

All129 direct container children close within their original phase cutoffs.
The foreground emulator waits0, the retained adb server is terminated and
waits SIGTERM15, and the foreground Docker CLI waits0. A fresh exact-container-ID
absence check, same-daemon check and input rehash close the outer run. Root
independently rehashes465 input bindings and checks the17 case records and8 parser
calls. [Bounded provenance](evidence/native-ims-registration-routing-provenance.json)
contains hashes and observations; proprietary inputs and raw logs remain private.

The first preserved setup attempt compiled successfully but its foreground ADB
server aborted on the numeric listen hostname before emulator/original execution.
A fresh frozen driver changes only that listener to the documented `localhost`
form and passes. This setup failure is not a behavioral RED. The source reference
is the [AOSP ADB manual](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md).
