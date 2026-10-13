# Omarchy Mobile reuse review

The public [Omarchy Mobile repository](https://github.com/derekross/omarchy-mobile)
is useful source material for Pokélinux's mobile desktop. This review pins commit
`4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7` (2026-10-13). It is a source inspection,
not execution of that project's scripts or an independent hardware qualification.
No phone or proprietary input was accessed.

The inspected implementation uses QML/JavaScript for presentation, Python for
sensor/SIT service logic, Bash for setup, and C for selected device utilities.
The pinned tree contains no Rust source. Reuse of service or policy logic means
selecting its contracts and porting them to Rust; genuine Omarchy presentation
may retain QML. Do not install its Python daemons as Pokélinux's final services.

Treat every candidate as unverified for our requirements. Static inspection does
not establish correct protocol behavior, safe lifetime handling or faithful
hardware state. Adoption requires a genuine RED for the intended contract and
actual boundary tests, including negative and ownership cases. Upstream tests
and reported hardware success remain separate evidence rather than acceptance
criteria supplied by the upstream author.

## Concrete reuse candidates

| Candidate | Observed implementation | Pokélinux integration requirement |
| --- | --- | --- |
| Mobile Omarchy launcher and bar | The [launcher](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-shell/plugins/omamobile.launcher/Launcher.qml#L225) uses actual `Commons.Color` and `Style`, minimum touch/input sizes, a swipe dock and on-demand keyboard focus. | Evaluate selected components with genuine Omarchy. Native app launch must still enter the installed Rust lifecycle service. Its detached shell commands do not supply our child ownership or permissions. |
| On-screen keyboard and touch | [User setup](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-shell/bin/omamobile-setup-user#L78) provisions squeekboard and avoids competing input-method ownership. | Exercise actual text-input/input-method boundaries and focus, including lock admission. Retain Pokélinux's 1080×2424,422PPI,420 logical density/2.625 profile; the upstream Pixel7 profile is different. Do not bulk-run the setup script. |
| Rotation policy | [Autorotation](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-sensors/lib/autorotate.py#L48) preserves scale and updates both monitor and touchscreen transforms. | Port policy into the portable Rust sensor/display boundary and test actual compositor output and touch mapping. Its compatibility fallback is outside our no-backwards-compatibility target. |

The shell plugins consume appearance inside the desktop session. They do not
solve the distinct native app UID's access to the private desktop theme home.
The [installed appearance RED](native-appearance-service.md) remains the next
service implementation boundary.

## Correctness checks identified by source review

The [rotation state update](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-sensors/lib/autorotate.py#L136)
records `self.current` before applying the transform; the fallback calls at
lines74–76 have unchecked results. The source can therefore skip a repeated
orientation after an apply failure. This is a static finding, not an executed
upstream failure test. Before porting, reject a compositor operation or remove
the output, restore it and repeat the same orientation. Success must follow
observed display and touch state. The synchronous control helper also lacks a
subprocess timeout; held-call responsiveness needs actual owned-worker tests.

The [launcher action](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-shell/plugins/omamobile.launcher/Launcher.qml#L125)
uses detached launch and dismisses the drawer. Its presentation feedback is not
our admission or process-ownership evidence. Test native launcher failures,
current app/context identity, close/reap and reopen. Deferred layer-shell focus,
OSK interoperability and lock-screen input remain execution hypotheses: test
actual Source focus and restricted app-listener bindings rather than infer them
from QML flags or setup-script success.

## Hardware and service boundaries

The reviewed [device descriptor](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/device/cheetah/device.conf)
targets Pixel7Pro/cheetah/gs201 and a mainline kernel. Its AoC sensor interfaces,
audio routes, speaker calibration and S5300/CPIF modem paths are device backend
references. They do not qualify Pixel10 or replace its initially matched Google
kernel/modules/firmware. Calibration-dependent setup and flash scripts are not
part of this inspection.

The [roadmap at this commit](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/docs/ROADMAP.md)
reports substantial Pixel7 hardware progress, including a partial ultrawide camera
path and fingerprint work. It still lists calls, SMS and mobile data as missing.
These are upstream reports, separate from Pokélinux's original-code, synthetic
Binder, generic ARM kernel and physical-device evidence.

Its [SIT implementation](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/packages/omamobile-modem-sit/lib/sit.py)
offers open-source framing, token, indication and ownership references. Its
vendor-chroot and Python daemon architecture does not replace Pokélinux's native
Rust permission owner, uncertain-registration fence or vendor cleanup gates.
Reuse should start with independently selected contracts and failure cases.

## Licensing and adoption status

The [root licence](https://github.com/derekross/omarchy-mobile/blob/4ee1339c2ebc5baf5be9b0167ffdfd0e1ea071f7/LICENSE)
and reviewed shell/sensor/audio/SIT packages are MIT. Preserve the separate MIT
widget attribution. Device/kernel material has GPL exceptions; the AOSP-derived
boot-control port retains Apache-2.0. Check each selected file and dependency when
adopting it, and retain its notices.

No upstream implementation has been merged, run or certified by this review.
The immediate sequence remains native appearance publication, installed live
theme changes, then the current lock/receiving tracer composition. Mobile shell,
keyboard and rotation work can use these sources at their own actual boundaries.
