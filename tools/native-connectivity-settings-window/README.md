# Normal Settings connectivity window fixture

These are the original core sources of one generic x86_64 VM case: the normal
Settings executable opens through the real per-app `display::Factory` on normal
upstream Hyprland. Ordinary uinput Enable/Disable touches reach the real Settings
Rust SDK, credentialed actor, private connectivity Main and NET_ADMIN worker.
The case uses authored unlocked/foreground facts and a persisted `NetworkControl`
grant. It does not qualify genuine Source/authentication, the full session-owner
Main, an installed ARM image, internet access or a physical phone.

The recorded run exercised byte-identical core sources in an owned VM envelope.
This directory supplies source, not a new measured public launcher, VM image,
provider binaries or private execution receipts. The normal appearance endpoint
is unavailable in this case; the ordinary QML fallback is used, without a new
theme provider.

**Files and existing dependencies.** `guest.py` owns normal compositor, profile,
input and window observations. `controller.py` owns the isolated network case.
`service_connectivity_window_tests.rs` is its actual lifecycle-libtest consumer.
`settings_build.py` builds only the normal Settings CMake target;
`settings_readiness.py` observes rendered control labels. `display.py` supplies
the existing native-capture function. `local_compositor.py` installs the selected
compositor/SDK cohort; it imports only the ELF inspection helper from
`qt_native.py`. `qt_native.build` is not selected.

Stage these files together as `/results/source`, with this exact dependency
allowlist from the repository:

| Staged name | Public source |
| --- | --- |
| `owned.py` | `tools/native-installed-services/owned.py` |
| `closure.py` | `tools/native-audio-preview/closure.py` |
| `peer.py` | `tools/native-connectivity-actor/peer.py` |
| `measure.py` | `tools/native-phone-display-profile/measure.py` |
| `physical-size-adapter.cpp` | `tools/native-phone-display-profile/physical-size-adapter.cpp` |
| `qt-output-discovery.cpp` | `tools/native-phone-display-profile/qt-output-discovery.cpp` |
| `raw-output-capture.cpp` | `tools/native-phone-display-profile/raw-output-capture.cpp` |
| `pixel10.json` | `sim/display-profiles/pixel10.json` |
| `PhoneAppearance.qml` | `native-app/PhoneAppearance.qml` |

The public `wlr-screencopy-unstable-v1.xml` is a separately selected protocol
input. Stage it at `/results/source/wlr-screencopy-unstable-v1.xml`; the unchanged
capture build requires SHA256
`0f50f6187965b02782a12ba346ad950d6031454f6af6e1ac5b341a279ef16d56`.
Keep the protocol's license notice. The [display-profile tools](../native-phone-display-profile/README.md)
describe this capture provider. The cohort-specific `public-api-pins.json` is
also a separately selected SDK metadata input, qualified against the chosen base
and compositor SDK by `local_compositor.py`; its recorded hash is in the
provenance. The source bundle excludes that generated SDK snapshot.

The selected guest needs Python/Pillow/evdev,
udev/uinput, seatd, D-Bus, CMake/Ninja, C/C++ compilers, Wayland client/scanner,
libdrm and the matching Qt modules. The recorded guest used Qt 6.12.0 and an
unchanged qualified 367-package image, with no package installation or network downloads.

**Build the copied test overlay.** Use a disposable repository copy at the
revision accompanying these sources. Preserve the normal lifecycle sources;
apply only this test overlay in that copy:

1. Add the `pokelinux-display = { path = "../native-display" }` dev-dependency.
2. Copy the published `native-session-owner/src/config.rs` and `display.rs`
   byte-for-byte to copied lifecycle `src/installed_config.rs` and
   `src/installed_display.rs`. Add these cfg(test) declarations to its `lib.rs`:

   ```rust
   #[cfg(test)] extern crate self as pokelinux_lifecycle;
   #[cfg(test)] #[path = "installed_config.rs"] mod config;
   #[cfg(test)] #[path = "installed_display.rs"] mod installed_display;
   ```

3. Copy this directory's `service_connectivity_window_tests.rs` into that copied
   `src`, and register it in copied `service_restore_tests.rs`:

   ```rust
   #[path = "service_connectivity_window_tests.rs"]
   mod connectivity_window_tests;
   ```

4. In the copied `ActorControl::observe_gui_publication`, expand only its UID
   filter from `credentials.uid.as_raw() != 11010` to
   `![11010, 11050].contains(&credentials.uid.as_raw())`. This observes actual
   successful actor replies; it does not instrument the normal app or claim a
   Qt `clicked` signal.

This keeps Factory and actor traits in the same libtest crate identity. Use the
normal path-dependency closure and cached registry/toolchain. The offline lock
update may add only the local display dev-dependency edge; preserve every
registry package version/checksum. Record the generated lock and compiler JSON.
The actual build stops at `--no-run`:

```sh
case_copy=/work/repository
cargo check --offline \
  --manifest-path "$case_copy/native-lifecycle/Cargo.toml" \
  --lib --message-format=json
cargo test --no-run --locked --offline \
  --manifest-path "$case_copy/native-lifecycle/Cargo.toml" \
  --lib --message-format=json
```

Select the actual lifecycle libtest executable from its `compiler-artifact`
message. At runtime the controller executes exactly:

```sh
case_driver=/usr/libexec/pokelinux-fixture/driver
"$case_driver" --exact \
  service::restore_tests::connectivity_window_tests::normal_settings_window_connect_packet_lifecycle \
  --ignored --nocapture --test-threads=1
```

**Select the guest inputs.** An owned generic x86_64 VM preparer supplies
`/results/selection.json` and immutable inputs. The checked interface is explicit:

- `providers` supplies paths relative to `/results`, byte counts,
  SHA256 and ELF machine 62 identity for the libtest driver, connectivity Main, worker
  and normal session-owner helper. The helper runs only its existing
  app-entry/storage-worker/display-context-worker modes. The Main/worker and
  helper retain their separately qualified build lineages.
- `normal_settings_build` selects the normal frontend source root/file pins and
  exactly three matching static archives: Settings, Appearance and Network
  Status, each with its SHA256. The frozen configure closure has 26 source
  rows, including test-source paths referenced by normal CMake configuration.
  Only target `pokelinux-settings` is built, with `-fPIC`, no Cargo or component
  execution. Its actual generated ELF/link/loader proof fills the Settings
  provider; there is no future output hash supplied in advance.
- `source_built_compositor` supplies one independently qualified normal
  Hyprland ELF, matching public SDK, build/qualification inputs and package
  inventory pin, as checked by `local_compositor.py`. The measured upstream
  cohort has one declared protocol-allowlist source change. A new input cohort
  needs its own qualification; private archived envelopes are not distributed.

Preserve executable basename `pokelinux-settings`. The trusted actor role is
`org.omarchy.Settings`; the expected ordinary Wayland window class is
`pokelinux-settings`. The app receives six normal absolute arguments through
Apps/Factory, with UID:GID 11050:11050 and no test-control endpoint. The normal
Main sets HOME/XDG/Wayland from those arguments. The guest rejects an inherited
session owner or second compositor; it does not migrate or start the full
session-owner configuration.

After staging and qualifying those inputs, the original in-guest entry is:

```sh
python3 -B /results/source/guest.py
```

Run it as trusted guest root under an owned VM envelope; the measured outer
container used UID:GID 1000:1000, no network and only the generic KVM device.
Its writable results directory was owned by 1000:1000 with mode `0755`, with frozen input
files read-only. The compositor is UID 1000 with the existing `seatd -g alarm`
setup and `SEATD_VTBOUND=0`. Normal executable ancestry is root-owned/protected.
Initial network-fixture capabilities are CHOWN, DAC_OVERRIDE, SETUID, SETGID,
KILL, SETPCAP, NET_ADMIN and SYS_ADMIN. The actor retains the first six; only
the backend worker retains NET_ADMIN. The controller/packet peer/Main drop
capabilities after setup. App E/P/I/A are zero with NNP 1; its recorded bounding
set is not claimed zero. Network namespace handles and actual child ownership
remain retained through cleanup.

**Original bounds and observations.**

| Owner | Work | Close |
| --- | --- | --- |
| Host | 810 s | 840 s |
| QEMU / boot owner | 780 s | 800 s |
| Guest | 660 s | 700 s |
| Network controller | 60 s | 70 s |

GUI setup has one 30 s bound. Rendered readiness uses at most 8 original full
frames under one `min(parent_end, now+1s)` cutoff per gate, without sleep or
renewal. The first gate precedes the one 12 s functional clock; the second stays
inside that clock and precedes release of the second mapped window. Each
external action has its original 1 s bound. Quiet uses one 800 ms phase including
gate delivery; armed-to-final-EAGAIN is recorded separately.

Output and mapped-window identity must match 1080 × 2424, nominal 65 × 146 mm and
scale 2.625 before input. Readiness checks only the known central Enable/Disable
label ROIs against fallback foreground/muted colors. It binds the exact normal
QML resource, PID/exe and output before/after capture. State-text regions require
semantic review of the original native PNG; this is not OCR or an app getter.
There is no resized capture, fixed readiness sleep or effect retry. An unready
frame budget is visual setup failure; screenshots cannot replace the functional
packet/completion assertions.

The positive case requires initial fresh disabled/no-control state and an empty
independent IPv4 readback; persisted grant and rendered Enable readiness; one
normal touch and matching Completed(true); exact fixed address/default-route
facts and nonce1; normal compositor close and actual Apps wait0; fresh app/SDK/
listener generation, unchanged grant, two fresh enabled states and nonce2 while
the background connection survives; rendered Disable readiness, one touch and
Completed(false); independent address removal, fresh ENETUNREACH and quiet;
normal second close; then backend/peer/namespace cleanup before the compiled
positive assertion. App stdio is null, so its actual waits have no PIPE-EOF
claim. SDK terminal pidfd/proc/listener observations are separate from the wait
owned by its existing Context Supervisor. Scope waits/PIPE EOF and the native
worker's Main-derived wait/output EOF are counted separately.

The recorded native [Disabled/Enable-ready frame](../../docs/assets/settings-connectivity-disabled.png)
and [reopened Enabled/Disable-ready frame](../../docs/assets/settings-connectivity-reopened-enabled.png)
are original output captures. Qt component and C ABI evidence remain separately
scoped in the [Qt report](../../docs/native-settings-connectivity-qt.md) and
[C ABI report](../../docs/native-settings-connectivity-cabi.md). This composition
adds normal-window coverage; it does not retroactively identify the cause of an
earlier failed input or qualify every click, physical networking or a full phone
session.
