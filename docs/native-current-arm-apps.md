# Current ARM app builds and Rust providers

The pre-network-card Settings and Messages-with-battery cohort passes a native ARM
production CMake Release build, protected installation, loader checks and
component execution. This uses the current Settings600 engine and all four
matching ARM Rust archives. It executes neither normal app main nor a window;
default ARM Cargo and a combined installed image remain open. Earlier ARM
Messages/Sounds builds and graphical attempts below retain their own cohorts.

## Current Settings and Messages-with-battery

The frozen64-file source cohort matches the public base
`21a1fa9e2fa1b0ad00f2627e14db378f071d7674`. Production CMake links explicit
matching-target archives; this epoch does not rebuild them with guest Cargo.
The later local-network Settings card and its additional Rust/Qt adapter are
outside this64-file cohort; its59fb0ef1 ELF is not relabelled as that frontend.

| Boundary | Actual current result |
| --- | --- |
| Native tools and Qt | AArch64 kernel7.1.6, Qt6.12.0, GCC16.1.1, CMake4.4.4 and Ninja1.13.2. Exactly eight trusted offline tools extend353 packages to361; all original versions and50 Qt identities remain unchanged. |
| Normal Settings | New AArch64 ELF `59fb0ef1`,6,293,992 bytes. One Settings and one appearance archive supply exactly6+11 strong C APIs. |
| Normal Messages | New AArch64 ELF `c7bb89e2`,6,139,672 bytes. Messages, appearance and readonly battery archives each link once and supply exactly12+11+4 strong C APIs. The optional app observer is off. |
| Native loader and components | Five actual `ldd -r` checks pass. UID11050 Settings executes C ABI layout/refusal/actual join and constructs its real unavailable adapters, normal embedded QML and installed shared theme QML. Messages executes its frozen nine C ABI groups and three unchanged offscreen installed/live-appearance/policy groups as UID11010, with all three real adapters. |
| Protected install | Two normal binaries are root0:0/0755; their two desktop entries and shared `PhoneAppearance.qml` are root0:0/0644. All five installed hashes match. |

The components show no window and execute no normal six-argument main. Their
unavailable endpoints establish no positive brightness/battery service reply,
Source permission or visible state. Original200ms Settings Stop and actual join
remain enforced. The earlier Settings6f9 and Messagesc52 normal ELFs are not
reused or relabelled. Current source/header/lock matching qualifies the reused
archives; separate historical Rust tests are not counted as new executions.

Exactly56 Scope direct waits and56 merged PIPE EOFs close within their original
cutoffs:15 host and41 guest. Seven separately observed nested waits exit0—one
Messages C ABI child and six root component peers—with no additional PIPE EOF
claim. Compiler descendants are not counted as separately observed waits. The
exact container exits0 and is removed on the same daemon;103 selected and11
external input pins remain unchanged.

The functional result is saved before the inherited shutdown request. That
request uses the existing Python timeout/cleanup path and has no separate
poweroff wait/PIPE EOF receipt. This epoch claims neither a direct QEMU wait
nor QMP shutdown evidence. The original720s guest work/724s close and900s outer
bounds remain unchanged. [Hash-only current provenance](evidence/native-current-settings-messages-arm-qt-provenance.json)
keeps this boundary distinct from normal app/window execution, default ARM
Cargo, a combined image, the parked Messages daemon-provider join and hardware.

## Earlier Messages and Sounds cohort

The earlier normal Messages and Sounds cohort passes a native ARM production CMake
Release build and protected installation with the shared appearance Rust client.
Their unchanged nine- and six-group C ABI consumers execute successfully in the
generic Arch ARM QEMU guest. That build does not execute either normal application main or window.
That build/install/component milestone is separate from the earlier
[pre-port Messages ARM result](native-messages-rust-client.md#native-arm-production-cmake-build-and-installation)
and the [current prepared x86 Messages window](native-messages-appearance-rust-tracer.md).

## Native production CMake and installation

| Boundary | Closed result |
| --- | --- |
| Native tools and Qt | The actual353-package base gains exactly eight trusted offline tools, becoming361 with every original version unchanged. CMake4.4.4/Ninja1.13.2 and GNU16.1.1 build against Qt6.12.0 under the generic7.1.6 ARM kernel. All50 retained Qt libraries/plugins match before and after. |
| Normal app links | Each normal ELF contains exactly one matching app Rust archive and one shared appearance archive. Messages has12+11 strong public APIs; Sounds has13+11. All three archives expose their exact12/13/11 APIs. |
| Native loader and components | Four actual guest `ldd -r` checks resolve the normal apps and consumers. The unchanged frozen Messages9 and Sounds6 groups pass, with `-UNDEBUG` after Release's `-DNDEBUG`. These are C ABI component executions, not Qt window or audio-routing results. |
| Actual installation | Both normal binaries are root0:0 mode0755. Both desktop files and the identical protected `PhoneAppearance.qml` are root0:0 mode0644. All five hashes are checked after both CMake installs. |

The source cohort has73 exact files at
`d896aa322a47161cbee9a9a708e189c4980661c6`. All selected non-Markdown code matches
the later documentation base `6cf1ca7216fb20fc9759e7739437be86207d8529`; only
the selected Messages README has an explicit documentation bridge. No older
adapter or normal app ELF supplies the result. The six production arguments and
UID11010/11042 requirements are bound through the unchanged compiled mains and
installed bytes; their successful runtime admission is not claimed.

The closed review verifies114 selected and25 external input pins. It checks the
four retained AArch64 normal/consumer ELFs, exactly-once archive links, native
loader output, component stdout and protected installed bytes. The explicit
matching archives are used through production CMake; ARM default Cargo is not
executed. The appearance eight-group and Qt29 suites, old seven Messages worker
controls and five ordering cases are not repeated in this epoch.

## Current Rust crossproviders

A separately closed optimized crossbuild supplies the current Sounds ARM engine:
451 ELF members are AArch64 and exactly13 strong APIs match the current header.
Its13 source rows match the published source above. The current Rust owner,
Messages/Sounds fixed launchers, audio and appearance services also crossbuild in
optimized Release mode with strict Clippy and actual empty Cargo feature sets.
The separate pre-start Messages seed selects only `isolated-fixture`. All six
service/fixture ELFs are AArch64; the independent review verifies175 distinct
committed source blobs across these two crossbuild cohorts.

Those crossbuilds close20 host waits/40 PIPE EOFs and34 foreground shell waits
with regular logs. They establish compiled providers only: the service ELFs do
not execute or resolve ARM dynamic dependencies in those epochs. Their result
must not be merged with the app guest's native loader/component observations.

## Preserved failure and closure

The first dual-app epoch remains failed. Its Messages archive-symbol audit exceeds
the initial10s wait cutoff and is terminated; passing earlier build/install and
consumer commands do not qualify that aggregate. The successor changes only the
three archive `nm` bounds from10 to30s, retaining normal-ELF `nm` at10s and all
production inputs, consumer assertions and overall bounds. No feature RED or
production repair is attributed to this harness-bound correction. Root's path
mistake and immediate capacity/source review are explicitly postlaunch, not a
completed prelaunch guard.

The successful epoch closes36 guest direct waits and36 merged PIPE EOFs within
their original cutoffs. One nested Messages C ABI child wait adds no PIPE EOF.
The outer Docker CLI has one actual wait; two recorded inspection CLI waits are
separate and carry no PIPE EOF claim. The console is a regular file. CMake/Ninja
compiler descendants are not presented as separately observed direct waits.
Exact container absence, same daemon, shared read-only backing/input closure and
the single720s work/724s guest-close/840s QEMU/900s outer bounds pass.

[Hash-only provenance](evidence/native-current-arm-apps-provenance.json) binds the
closed result, independent reviews, exact providers, source bridge, installed
files and preserved first failure. Frozen preparation flags are preserved;
actual execution and closed review supply this later qualification.

## Parked native full-shell attempt

Two later native x86 full-shell epochs remain failed with no functional groups.
The first stops before installation at copied database-signature policy. Its sole
successor adds `DatabaseOptional`, preserving mandatory trusted local package
signatures. Package installation, fresh current app builds and their nine/six C ABI
checks then pass separately, but the owner exits1 with `Kind(PermissionDenied)`
before compositor/Source/menu execution. The exact cause is not established.
Real user-manager start/Stop is a separate observation, not a successful session.
[Compact failure provenance](evidence/native-full-shell-parked-provenance.json)
preserves both failures. This x86 attempt is parked. The later ARM build and setup attempts below use
the normal upstream compositor configuration.

## Whole native ARM compositor build

A separately closed native ARM build produces the normal upstream Hyprland0.56.2
ELF from `efb50993780079460b0cbed1363e2166a2de1d9f`, with only the existing
logical-output allowlist patch. GNU16.1.1 builds the required main library with
C++26/O3 and no LTO. The resulting AArch64 ELF and its complete matching498-file
SDK pass independent readback and729 review checks. This is a locally built
compositor, not a signed Arch compositor package. The448 configured C++ commands
include14 auxiliary commands that remain unbuilt and unqualified.

This build establishes compilation and SDK identity only. It does not execute
the compositor ELF, its dynamic loader, a compositor main, Source or a window.
Its24 guest direct waits/24 merged PIPE EOFs are separate from the app-build
counts above. The outer console is a regular file; the Docker CLI and two
inspection CLI waits add no PIPE EOF claim. QEMU exit0 is supported by the owned
foreground wrapper, without QMP or a separately recorded direct QEMU wait.
The original3600s work and4200s outer bounds and recorded input/container closure
pass.

## Current ARM normal-window setup attempts

Two later prepared ARM window attempts remain failed. The first fails entry
preparation because the entry timestamp environment value is missing; it produces neither an entry result nor a
guest result. No guest-case closure or functional result is inferred from its
successful QEMU shutdown.

The second installs exactly43 trusted packages over the353-package base,
reaching396 while retaining every original version and all50 Qt identities.
It installs the new compositor and all498 matching SDK files, and native
`ldd -r` checks pass for the compositor, normal Messages and its frozen consumer.
It then fails setup with `PermissionDenied` when starting that consumer from the
shared export. The consumer does not execute; no Source or app functional group
runs. Fifteen actual guest direct waits/merged PIPE EOFs and guest-case cleanup
are recorded before this setup failure.

Post-run inspection of all four exported app/consumer files finds host0755 but
`user.virtfs.mode=0100644`, which the9p mapping preserves. The next source
preparation copies and installs the consumers into the actual guest filesystem
with0755 before execution. That packaging correction has no runtime
qualification in this record. Both failed attempts retain actual QEMU exit0,
Docker CLI exit0, exact container absence, the same daemon and unchanged inputs;
these closure observations do not turn either failure into a window pass.

The prepared window cohort intentionally retains the older audio ARM ELF
`c4a33c45377194d75d8e7af40360f4be62992690cf908cc6d1498303def49306`.
It is not the HEAD service cohort. The SIGTERM-to-Stop change published at
`8d9e28dd6d0d41cfbb4ba62fcb9cfcf2efd5b901` separately crossbuilds the new audio
ARM ELF `14ef29d04272a19bf6673fd837fcbd50d29f7e75bae146ee2615db25fe7e4963`;
its installed ARM/PID1 behavior remains unqualified here.

[Hash-only build and setup provenance](evidence/native-arm-compositor-window-setup-provenance.json)
binds the compiled artifacts, both preserved failures and the separate audio
cohorts. No active or future attempt supplies a result to this record.

## First prepared current ARM runtime remains failed

The consumer install correction allows a third epoch to pass all five native
loader checks and the unchanged nine/six consumers. It starts the actual new
compositor ELF, builds the matching adapters and reaches real Rust Source
readiness. The prepared warmup's native capture passes; it is not a Messages
capture. The fixed launcher also produces an observed UID11010 Messages process
with bound parent and argv. Final same-PID executable-hash, mapped-window and
normal-app density/capture assertions are not reached.

The owned-app registry check then resets. Before cleanup, the authenticated owner
snapshot records `sample_expired`: sequence462 is101,804,273ns old at validation,
beyond the unchanged100ms contract. Access is withdrawn. This records availability
failure under the ARM simulation, not a proven CPU cause or a production repair.
The timing branch is parked; no further Source or cold-graphics campaign is selected.

Zero app-window groups pass. Registry exit1 and unexpected theme-host255 remain
failed, and guest-case closure stays false. The observed86 primary waits/93 PIPE
EOFs still close within their original bounds;95 adopted waits and one nested
C ABI child wait add no PIPE EOF claim. Owner Stop client, owner, audio and
appearance exit0 separately, external app/compositor pidfds are dead with `/proc`
absence, and QEMU/CLI exit0 with exact CID, daemon and input closure. These facts
do not convert the failed window aggregate into a pass. No new app screenshot
is presented. The same [hash-only record](evidence/native-arm-compositor-window-setup-provenance.json)
binds this result and its independent closed review.

A different [installed audio/appearance boot slice](native-installed-services.md)
now passes both preparation and fresh PID1 autostart with unavailable domain
state and actual Stop/Main0. It selects the new SIGTERM audio ELF14ef29; it does
not supply a healthy graphical session or retroactively repair this frozen
older-audio normal-window selection.

## Remaining integration

Complete normal ARM main/window qualification, current app permission/live-theme/audio behavior,
current full ARM shell/service/Source composition and ARM default Cargo remain
open. The crossbuilds do not themselves provide runtime qualification; the failed
GUI epoch and separate installed boot retain their own scopes. Current
Sounds GUI, cold startup, native display cleanup, suspend, modem/carrier and
physical Pixel hardware remain unqualified. No x86 SDK, GUI result or historical
ARM app result is inherited. No physical phone was accessed.
