# Sounds and the current native appearance service

The later [complete normal ARM tracer](native-audio-arm-ui.md) qualifies the
separately selected direct-stream UI/audio/theme path. Evidence and remaining
gates below retain this checkpoint's own scope.

Sounds now consumes the current Rust appearance service through a shared
`PhoneAppearanceClient` and `PhoneAppearance.qml`. Its fourth launch argument is
an absolute appearance socket, replacing the earlier appearance-HOME argument.
The normal binary compiles and its installed component loads successfully;
normal ARM GUI/audio/theme composition remains the next gate.

## One matching contract

The client accepts only current schema3. It verifies the root service's peer
credentials and each packet's sender identity, bounds frames and ancillary data,
checks absolute BOOTTIME request/publication deadlines and rejects old schemas,
stale revisions and changed data at the same revision. Appearance supplies public
presentation data, never session or route authority. The audio client and its
connection-owned Media lease remain independent.

Current RGBA16 colors, scale, font-size and spacing tokens feed the QML theme
object directly. The new QML component imports QtQuick only: it needs neither
the old file reader nor JavaScript parsing. Its availability follows the current
publication lease. Full font-family/settings integration and rendered appearance
fidelity remain open; forwarded tokens alone do not prove them.

The shared client and matching QML reuse existing qualified current-schema code
through exact class/QML filename renames. The selected29 existing Linux IPC cases
are mechanically renamed as well. Root verifies their inverse mapping and checks
the matched ARM appearance service's Cargo/source/test inputs against current
Main; only its README differs. The older schema1 broker/client is reference
infrastructure and is not selected by Sounds. No old-schema alias is added.

## Concrete checks

The original reuse selection passes strict host compilation and eleven unchanged
audio socket cases. The later [input repair](native-audio-readonly-input.md)
passes18 host audio behaviors without changing appearance/QML/main. Twenty-nine
actual appearance IPC controls pass, including sender UID/PID, restart/loss,
malformed/oversized/FD refusal, actual deadline/late-receive checks, schema1/2
refusal, exact RGBA16/alpha/revision behavior and QML style-token forwarding.
Each probe retains its actual forked peer wait. The earlier505-case campaign is
not rerun or claimed by this reuse.

CMake installs exactly the normal Sounds executable and `PhoneAppearance.qml`.
A separate QCoreApplication/QQmlEngine check mounts only that exact install tree,
loads the installed URL, observes Ready/create with no errors and verifies a
forwarded fixture value. It creates no window or preview. The component already
passed: no missing-resource RED or production packaging repair is claimed.
One initial probe-output ownership setup failure remains preserved separately.

Host build/probe receipts retain explicit local-daemon identity, actual waits,
applicable EOF, exact-CID absence and unchanged selected source hashes. Root
independently rehashes the16 selected files, exact inverse renames, peer results,
compiled binaries and the matching service source. [Hash-only evidence](evidence/native-audio-appearance-client-provenance.json).

## Remaining integration

Build and execute this exact current cohort in the finite normal ARM tracer:
ordinary UI input, Rust-owned virtual audio and measured sink samples, call
interruption/restoration, loss/no replay, real window close and live original
Omarchy palette changes. Keep HOME private and qualify actual Pixel10 output,
window DPR and unscaled native capture. Host IPC/component success is not this
composition, trusted foreground authorization, a production launcher or physical
audio. No phone was accessed.
