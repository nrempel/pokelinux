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

The proposed simulation-only adapter supplies the nominal extent through the
matching compositor's public output API. Actual DRM/EDID dimensions remain
recorded separately; no Pixel kernel/display qualification follows. The positive
metadata boundary and native-buffer capture are still pending. User previews
require a compositor-announced native1080 ×2424 capture buffer encoded directly,
with no resize or guessed fractional export scale. Older screenshots remain
historical at their original resolution/density. Deliberately wrong-density
negative measurements remain private test evidence.
