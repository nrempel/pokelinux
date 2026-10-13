# Virtual physical-size adapter contract

The adapter is implemented using the public API below. The actual
`density-red-2` run qualifies the negative physical-density boundary at
1080 × 2424. After a separately preserved include-order compile prerequisite
failure, `density-green-3` qualifies corrected physical metadata, raw native
PNG dimensions/pixels and normal unload restoration. The measurement surface
does not qualify a phone app preview.

The simulation profile supplies the physical extent of a virtual display. The
adapter uses the installed Hyprland plugin API and Aquamarine output fields
to publish that extent through the compositor. It leaves the actual DRM
connector and EDID observation intact and record their separate precision.

## Exact public API path

The retained Hyprland is v0.56.2, commit
`efb50993780079460b0cbed1363e2166a2de1d9f`. Aquamarine is 0.15.1.

1. A normal `PLUGIN_INIT(HANDLE)` first compares
   `__hyprland_api_get_hash()` with `__hyprland_api_get_client_hash()` and refuses
   an ABI mismatch.
2. The adapter parses the selected JSON profile with actual guest Qt 6.12
   `QFile`, `QJsonDocument` and `QJsonObject`. No Qt application or DPI override
   is instantiated. It rejects any changed profile identity, pixels, nominal
   millimeters or selected DPR.
3. `State::monitorState()->monitors()` must contain exactly one enabled output.
   Its public `name()`, `pixelSize()`, `scale()` and `output()` must identify
   `Virtual-1`, 1080 × 2424, 2.625, and the existing Red Hat QEMU Monitor. The
   adapter refuses every other output and saves its original physical extent.
4. The only metadata change is the normal public field assignment:

   ```cpp
   monitor->output()->physicalSize = Hyprutils::Math::Vector2D{65, 146};
   monitor->m_events.modeChanged.emit();
   monitor->scheduleDone();
   ```

   The actual values come from the validated profile rather than constants
   in this illustrative snippet.
5. The matching public output protocol listens to
   `monitor->m_events.modeChanged` and calls each bound resource's
   `CWLOutputResource::updateState()`. That method publishes the same
   `m_output->physicalSize` in `wl_output.geometry`, followed by `done`.
   A newly bound output also receives `updateState()`.
6. A normal plugin exit restores the saved extent only if the retained weak
   monitor reference still denotes that same output, and emits the same normal
   update. There are no function hooks, offsets, casts to private memory,
   Source events, policy calls or grants.

Matching installed public header hashes:

| Header | SHA-256 |
| --- | --- |
| Monitor.hpp | `4e9d88475f614f83015ed799d31425e35b47eb796dd91a4d8ec8f8ae88442621` |
| MonitorState.hpp | `cf04c62fe25a86fc3e17d5abdd6aad87e18d86fd4c327e68745108ec83d39488` |
| protocols/core/Output.hpp | `d6388b58409bc3437a92152cd355fdcda2dba8d6083b9bf5ea1a7a2f42f0f97f` |
| PluginAPI.hpp | `6a47b6bf31e781bf40297f7385ff7a18082812f252f5039961c2dae26a5ef740` |
| Aquamarine Output.hpp | `1568dcf54e01fc42079b5d68d7189d135f3a2c57aceed9cd0d3f58c964a5a37d` |

The matching [output protocol source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/core/Output.cpp)
is SHA-256 `74ab6553e39c3587eb5b3850dbc8eb69b85e4a2aa738ca2353ea2837739c0f35`.
The matching [monitor source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/output/Monitor.cpp)
is `9cf88e154eb5dae676c79d37b5b055ca6134838857cecdbb89a3b747a6821927`;
it supports `debug.disable_scale_checks=true` for the selected fractional
logical bounds. Both negative and positive selections retain that setting.

## Fractional Qt measurement prerequisite

The actual negative observation reports Qt 6.12 `QScreen` DPR 3 and
`QWindow` DPR 2.625, with logical bounds 411 × 923. Its physical extent is
270 × 610 mm. `QScreen.physicalDotsPerInch` uses the rounded logical bounds,
while the Wayland screen DPR uses integer `wl_output.scale`. Multiplying those
two screen values therefore does not reconstruct the native pixel density on
this fractional output.

The matching Qt 6.12 primary sources show this directly:

- [QScreen physical DPI](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/kernel/qscreen.cpp)
  uses `size() / physicalSize() * 25.4`; source SHA-256
  `6ef80d60d720d4e824bfcc07b203ea506059d1ef98240fd7d357f3db342f3cc1`.
- [Wayland screen geometry and DPR](https://github.com/qt/qtbase/blob/v6.12.0/src/plugins/platforms/wayland/qwaylandscreen.cpp)
  uses XDG logical bounds for geometry and integer `mScale` for DPR; source
  SHA-256 `45f2f45e977ba0668b926b3bb6c33283b3906bd4061b6471f2ead01f9c9383ff`.
- [The QScreen DPR documentation](https://doc.qt.io/qt-6.12/qscreen.html#devicePixelRatio-prop)
  explicitly prefers the target window DPR when fractional Wayland differs.

The versioned physical Qt measurement rule uses this observed coordinate
basis. The implemented consumer v2 retains the
actual screen DPR and its mismatched product as diagnostics, and qualifies
native pixels/mm and physical DPI multiplied by the actual target window DPR,
both against 422 ± 1 PPI. The measured logical extent must differ from
native pixels divided by window DPR by less than one logical pixel per axis.
The native pixel, DPR, physical extent, density tolerance and capture guards
remain unchanged. Consumer v2 was executed against the immutable actual
negative observation before producer implementation and still rejected it.

## Actual positive selection

The guest compiled this small adapter with its matching public SDK and
normal guest compiler, loaded it via `hyprctl plugin load`, and then started the
unchanged actual DRM/Wayland/Qt output probe. The profile, resolution, monitor
scale, compositor settings, kernel, backing image and measurement probe remain
selected explicitly. All original-density negative evidence stays immutable.

Only after correct metadata qualified was a native output PNG captured.
The standard full-output screencopy protocol announced a 1080 × 2424 SHM
buffer, stride4320. The helper encoded that buffer without a resize or
fractional export scale and verified the decoded PNG pixel hash against the
oriented native pixels. The normal plugin unload restored270 × 610 mm on the
same retained output. All five direct guest children and the container CLI
were actually waited, with immutable backing/kernel/initrd hashes unchanged.
The private native capture and binary bytes are not published in these tools.
