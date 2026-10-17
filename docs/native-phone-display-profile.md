# Pixel 10 UI validation profile

Future UI runs and screenshots target the Pixel 10's portrait1080 ×2424 native
pixels and published422PPI. Google lists a6.3-inch display. These are the
[official display specifications](https://store.google.com/product/pixel_10_specs?hl=en-US),
not physical-device measurements by this project.

The virtual profile uses nominal65 ×146mm, calculated from those pixels/PPI and
rounded to millimetres. The resulting axis densities are422.03 and421.71PPI;
the qualification tolerance is422 ±1PPI. Do not describe these nominal extents
as Google's measured panel dimensions. The retained matched Android image's
`ro.sf.lcd_density` is420, corresponding to a selected UI DPR of2.625. Android
dp and Qt units still need actual integration measurement.

The actual negative ARM kernel/Hyprland/Qt6.12 run already uses1080 ×2424 and
window DPR2.625. The virtual GPU instead reports270 ×610mm through Wayland/Qt,
approximately101PPI. It fails the intended physical-density boundary. No preview
PNG was captured. Original kernel/backing images and the negative selection
remain unchanged.

Fractional Wayland produces QScreen DPR3 and QWindow DPR2.625, with rounded
logical bounds411 ×923. Qt explicitly recommends the target window DPR when
these differ in its [QScreen documentation](https://doc.qt.io/qt-6.12/qscreen.html#devicePixelRatio-prop).
Qualification records both values. It requires exact native pixels and nominal
physical extent, window DPR2.625 ±0.001, native pixels/mm and Qt physical DPI
multiplied by that window DPR each within422 ±1PPI, and less than one logical
pixel of rounding per axis. The mismatched screen-DPR product remains diagnostic.
A corrected versioned consumer still rejects the same frozen negative observation
before any producer change; it does not relabel the original result.

The simulation-only adapter now supplies the nominal extent through the matching
compositor's normal public output API, guarded by its exact SDK/ABI identity and
single selected virtual output. Actual DRM/base EDID remains270 ×610mm and its
preferred detailed timing reports274 ×615mm; these values are preserved separately.
No Pixel kernel/display qualification follows from the virtual metadata change.

The positive GREEN3 selection passes three actual ARM groups: physical metadata
propagation, native capture pixel equality, and normal unload restoration to the
same output's original270 ×610mm. Wayland/Qt reports65 ×146mm and WindowDPR2.625.
Native axis densities are422.03077/421.70959PPI; Qt's rounded logical/window basis
gives421.59115/421.51387PPI. Integer ScreenDPR3 and its481.8PPI product remain
diagnostic. A separate earlier compile fails before adapter load because of the
Qt/Hyprland HANDLE macro; the minimal include-order fix precedes GREEN3. That
prerequisite failure is not a behavioral density failure.

The actual standard screencopy protocol announces1080 ×2424 with stride4320.
The helper encodes this SHM buffer directly and verifies decoded PNG pixels
against the native pixel representation. No resize/export scale/orientation
correction was used in this selected run. Five directly owned guest children
and the container are actually waited, with immutable input rechecks under the
same readonly lock. Root independently verifies93 private bindings and the PNG's
native dimensions/CRC/hash, and views the mapped measurement surface. See the
[provenance](evidence/native-phone-display-profile-provenance.json),
[authored evidence](../tools/native-phone-display-profile/observed-evidence.json)
and [reusable tools](../tools/native-phone-display-profile/README.md).

This is a measurement surface, not a phone app preview. Each future UI fixture
must retain the selected adapter, run actual metadata checks before Source/app
focus begins, and encode its native output buffer without resizing. Older
screenshots remain historical at their original resolution/density. Deliberately
wrong-density negative measurements and raw measurement captures stay private.

## Later normal Messages capture scope

The [prepared native x86 Messages tracer](native-messages-prepared-tracer.md)
retains the adapter, ordered profile application, native1080x2424 output, nominal
65x146mm density, scale2.625, actual mapped app bounds, independent registry/output
measurements and whole native-buffer/PNG equality. Its four unmodified captures
show the normal app, rather than a measurement surface.

This epoch does not sample the normal app's QWindow getters. It therefore does
not qualify the full direct-Qt window-DPR/physical-DPI product defined above. Its
explicitly bounded external output/window qualification and original native
captures remain separate from the earlier complete ARM measurement-surface
qualification; no old Qt-window value is inherited.
