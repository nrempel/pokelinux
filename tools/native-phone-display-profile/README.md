# Pixel 10 virtual display profile

The profile selects portrait 1080 × 2424, nominal virtual physical extent
65 × 146 mm, and native Qt window DPR 2.625. Google's published density is
422 PPI. Dividing the pixels by the rounded nominal millimeters gives
422.03077 and 421.70959 PPI; the validator keeps the explicit ±1 PPI tolerance.
The nominal millimeters are a derived simulation profile, not a measurement of
the physical phone. The scalar stock Android density reference is 420, giving
420/160 = 2.625. Android dp units and Qt logical units are not equated.

The profile is in `sim/display-profiles/pixel10.json`. The tools operate on
actual output observations and preserve DRM, EDID, Wayland and Qt fields
separately. They never resize an image or set a font DPI to claim physical
display density.

## Current qualification

One actual ARM system guest negative run, `density-red-2`, reached a real
1080 × 2424 DRM/Wayland output with an exposed Qt 6.12 window at DPR 2.625.
Its unchanged default physical extent was 270 × 610 mm, so the strict profile
validator rejected it. No UI PNG was captured. The frozen `density-red-1`
plan was never executed and is not evidence.

The selected guest also reported integer Qt screen DPR 3 with fractional
window DPR 2.625 and logical bounds 411 × 923. The versioned consumer v2 records
both products, uses the actual target window DPR for the fractional physical
coordinate basis, and still rejects this frozen wrong-density observation.
It requires logical bounds to differ from native pixels divided by window DPR
by less than one logical pixel per axis. The executed negative consumer and
evidence remain immutable. See [`adapter-review.md`](adapter-review.md) for
the matching primary source path. The normal-API correction is implemented and
the actual `density-green-3` system guest passed three groups: physical metadata
reaches Wayland/Qt, the native 1080 × 2424 buffer matches the decoded PNG without
resizing, and normal plugin unload restores the original physical extent.
`density-green-2` remains a separate compile prerequisite failure caused by the
Qt/Hyprland `HANDLE` include order; no adapter or capture ran in that label.

The positive guest reports native physical density 422.03077/421.70959 PPI and
Qt physical DPI times target window DPR 421.59115/421.51387 PPI. Its screen DPR
3 and screen-DPR product 481.81846/481.73014 remain diagnostics. Actual DRM/base
EDID stays 270 × 610 mm (detailed timing 274 × 615 mm); the virtual physical
profile reaches Wayland and Qt through the public compositor output API.
The captured measurement surface is not a phone app preview. The raw PNG and
compiled authored ELFs remain private; `observed-evidence.json` exports only
safe authored metadata, exact hashes and qualifications.

## Selection and execution

`plan.py` freezes a fresh selection. It does not boot a VM:

```sh
python3 tools/native-phone-display-profile/plan.py \
  --label density-red-new --output /tmp/pokelinux-pixel10-density-red-new
```

Inspect the selected files, exact Docker command, resource limits and hashes
before running the reviewed selection. Existing evidence labels are preserved.
`run-selected.py` requires the independently reviewed hashes, verifies all
selected inputs, takes the existing shared backing lock read-only across the
runtime, directly waits the container CLI and rechecks immutable inputs:

```sh
python3 tools/native-phone-display-profile/run-selected.py \
  --selection-dir /tmp/pokelinux-pixel10-density-red-new \
  --selection-sha256 REVIEWED_SELECTION_SHA256 \
  --command-sha256 REVIEWED_COMMAND_SHA256
```

The current selection uses a no-NIC QEMU snapshot of the existing read-only
backing, with four guest CPUs and 4096 MiB inside a six-CPU/eight-GiB container.
It compiles the measurement probe against the actual retained guest Qt ABI,
owns its compositor/probe/seat children directly and waits each in cleanup.
No app lifecycle, Source, policy, grant, vendor runtime or phone is selected.

## Measurement sidecars

`qt-output-probe.cpp` observes the actual DRM active mode and EDID, actual
`wl_output` geometry/current mode, and actual Qt screen/window metadata. It
rejects common Qt font/scale environment overrides and requires native
Wayland. It has a private measurement surface; it is not a phone UI.

`measure.py` writes a new JSON sidecar binding the profile, observation and
optional raw PNG by SHA-256. It checks exactly one matching output/screen,
native mode, physical millimeters, window DPR and physical density. If a raw
PNG is supplied, its original IHDR must be exactly 1080 × 2424:

```sh
python3 tools/native-phone-display-profile/measure.py \
  --profile sim/display-profiles/pixel10.json \
  --observation OBSERVATION.json \
  --raw-png RAW_NATIVE_OUTPUT.png --sidecar NEW_SIDECAR.json
```

An existing smaller capture, a resized PNG, Qt font DPI alone, or a matching
profile file without actual output measurements does not qualify this boundary.

## Reuse in a joint guest

The joint guest must select the same profile and matching public SDK, verified
by `public-api-pins.json`, and compile the three authored programs with the
commands used by `guest.py`. The standard capture XML is a separately selected
public input pinned to the matching Hyprland commit, SHA-256
`0f50f6187965b02782a12ba346ad950d6031454f6af6e1ac5b341a279ef16d56`.
Generate its normal client bindings with `wayland-scanner`; the guest needs no
network. Freeze the resulting adapter/probe/capture ELF hashes in that guest's
selection before interpreting its result. The successful `density-green-3`
receipt freezes the public authored ELF bytes and executable modes for reuse:
probe `610f4654d639f16406f1559fadbc98e9ca32c386ca7a8dd924d8527cf6b9be64`,
adapter `098b6a7c32090d74bad5af9c3cec899bf7ce24100159df5d1ddaed5182ba5c08`,
capture `383a0a4448fa4a9dfd5202e57b8dd92cc46d93b52df7a127920d62517dcbe5c8`.
Hash-only earlier probe builds are not exported reusable binary selections.

Use a 1080 × 2424 virtual GPU output and the actual compositor monitor rule:

```lua
hl.monitor({ output = "", mode = "1080x2424@60", position = "auto", scale = 2.625 })
hl.config({ debug = { disable_scale_checks = true } })
hl.permission("/tmp/pixel10-raw-output-capture", "screencopy", "allow")
```

The last permission is only for the isolated capture tool. It does not grant
a phone app screencopy or any Rust service authority.

Before starting the compositor, give it absolute paths in
`POKELINUX_DISPLAY_PROFILE` and `POKELINUX_DISPLAY_PROFILE_RECEIPT`. The receipt
must be a new writable path owned by the compositor's user. Load the matching
simulation adapter through the normal control interface:

```sh
hyprctl plugin load /tmp/pixel10-physical-size-adapter.so
/tmp/pixel10-output-probe NEW_OBSERVATION.json
python3 tools/native-phone-display-profile/measure.py \
  --profile SELECTED_PIXEL10.json --observation NEW_OBSERVATION.json \
  --sidecar NEW_METADATA_SIDECAR.json
```

Continue only if the metadata command exits zero. The probe opens a fullscreen
measurement surface and exits after observing it; run it before a joint
Source/focus fixture starts, so it does not steal that fixture's focus. The
adapter remains loaded for the later UI fixture. It refuses every output
except the exact selected single QEMU `Virtual-1` at 1080 × 2424 and scale2.625.
It publishes only the profile's physical extent through the normal output API.

After metadata qualifies, capture the full native output buffer while the
desired actual UI is mapped:

```sh
/tmp/pixel10-raw-output-capture Virtual-1 NEW_NATIVE_OUTPUT.png NEW_CAPTURE_RECEIPT.json
python3 tools/native-phone-display-profile/measure.py \
  --profile SELECTED_PIXEL10.json --observation NEW_OBSERVATION.json \
  --raw-png NEW_NATIVE_OUTPUT.png --sidecar NEW_CAPTURE_SIDECAR.json
```

The capture program requires a compositor-announced 1080 × 2424 SHM buffer and
65 × 146 mm output. It encodes that buffer directly. It preserves native stride,
format and hash, applies only a protocol-declared vertical orientation when
required, and verifies the decoded PNG's pixel hash against the oriented native
pixels. It does not use a logical export scale, `grim -s`, region resize or
font DPI. A capture or density failure remains failed. This tool's measurement
surface is not a phone app preview; the joint guest must bind its actual UI and
authority evidence separately.

After the guest's UI fixture finishes, the normal command
`hyprctl plugin unload /tmp/pixel10-physical-size-adapter.so` restores the saved
physical extent only for the same retained output identity. Do not unload it
between metadata qualification and the fixture's capture.
