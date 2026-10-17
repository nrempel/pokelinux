# Native display brightness

The new Rust brightness component separates deployment-selected backlight I/O,
local observation/intent ownership and the lifecycle actor's app permission.
This is a qualified component boundary. No production brightness daemon, Settings
window, installed service or physical display change is qualified yet.

## Attribute backend

A deployment explicitly chooses LinuxBacklight under `/sys/class/backlight` or
SyntheticTree for authored test files. Linux selection requires the fixed class
path and sysfs filesystem; it never falls back to a synthetic tree. Applications
cannot supply a device name or filesystem path. The component tests never open
host sysfs or a physical phone.

The backend reads bounded canonical decimal attributes: positive max_brightness,
brightness in range and optional actual_brightness in range. Missing optional
actual is distinct from malformed input. The Linux ABI distinguishes the requested
brightness attribute from actual_brightness; even the latter may be the driver's
stored value when hardware readback is absent. Neither is a luminance measurement.
See the [upstream backlight documentation](https://docs.kernel.org/gpu/backlight.html).

An explicit percent is converted with bounded integer half-up rounding. Set
validates the input attributes, opens the existing brightness attribute, makes
one write attempt and reads attributes independently afterward. A short or failed
write is never resent. A changed maximum or mismatched requested readback refuses
confirmation without rescaling/retrying. Attribute reads are sampled, not atomic
with external driver changes; the local expected revision is not sysfs CAS.
The caller must retain this potentially blocking operation in a child. That
process implementation is the next integration boundary, not a result of these
library tests.

## Local ownership and freshness

One ticket occupies the owner until supplied retirement. Set invalidates the
previous observation/revision on admission. Cancellation or original BOOTTIME
deadline expiry withdraws publication immediately, while retaining occupancy;
a late successful result cannot confirm the operation. Only a matching ticket
can release its slot. A fresh readonly observation after retirement may establish
current attributes, while preserving the earlier Set's uncertainty. A new write
requires a new explicit intent and a current positive readback revision.

The controlled-clock tests use one-second collection deadlines and observations
expiring two seconds from collection start. Calling retired in these tests is a
policy input: it proves no process wait, pipe EOF or kernel suspend behavior.

## App permission and lifecycle

The lifecycle actor now understands DisplayBrightness for the fixed
org.omarchy.Settings principal, UID11050. This names a future installed app; it
does not install or launch one. Typed brightness_set and brightness_status requests
carry a percent, expected readback revision and bounded lifetime, without device
identifiers or app-supplied capabilities. The configured provider entry explicitly
supplies the backend; unconfigured entries return Unavailable.

Existing registered-principal, permission, Source/session, foreground, lock,
background and suspend checks stay unchanged. Dispatch rechecks the current
readback revision and occupancy. Publication rechecks the original authority and
readback deadlines on every retry. Provider inspection may establish a fresh
readonly connection; a failed Set submission is terminal and never automatically
reconnected, queued or resent. Pending backend custody remains distinct from
whether an app reply is still publishable. Reaping cannot undo a property write.
The separate native-suspend authority and readonly battery service are unchanged.

## Actual RED and GREEN

All14 backend/policy groups compile against the stub:12 assertions fail and two
negative rejection groups already pass. The same frozen tests then pass: seven
actual synthetic regular-file/class-symlink I/O groups and seven controlled-clock
ownership groups. Literal rounding vectors include u32::MAX. A deliberately
hardlinked synthetic maximum/brightness topology changes the maximum during the
single write; actual post-readback refusal and final75 establish no rescaled
retry. That topology is authored filesystem evidence, not a kernel driver.
Format, strict all-target Clippy and optimized host/ARM64 library builds pass.
ARM64 is a crossbuild only, not an executed service. Each component RED/GREEN
container closes10 actual host CLI waits and20 PIPE EOFs with original bounds,
container absence and selected inputs unchanged. See the
[component provenance](evidence/native-brightness-component-provenance.json).

The lifecycle first preserves API-absence compiler errors separately, then runs
a valid compiled consumer: existing Query/grant controls pass, while the new Set
and grant assertion fails at runtime. An initial nine-group implementation passes.
A further compiled regression then exposes two failures: stale unsent revision
and expired queued readback. A brightness-session-only repair makes both controls
pass. The final ten focused groups, frozen consumers, formatting, strict Clippy
and host Release build pass. Eleven adjacent media and four Source groups pass
on the first candidate; their sources and common predicates are unchanged by the
later brightness-only correction. Controlled authority/provider inputs and actual
local SEQPACKET EOF, extra-packet, EAGAIN and credential boundaries stay distinct.
Across all five lifecycle epochs,50 host CLI waits/100 PIPE EOFs close. These
checks do not execute the full actor with a configured backlight. See the
[lifecycle provenance](evidence/native-brightness-lifecycle-provenance.json).

## Remaining integration and hardware gates

Next are retained real child processes, responsive daemon IPC/Stop, a concrete
provider adapter, normal Rust Settings client and themed UI, explicit installation
and a synthetic VM end-to-end tracer. Linux driver execution and matched Pixel
panel/backlight discovery, brightness response, physical readback, suspend/resume,
lock behavior and power/drain measurements remain separate gates. No phone has
been accessed, and this component does not advance calls, IMS or SMS hardware
qualification.
