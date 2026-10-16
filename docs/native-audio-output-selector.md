# Native media output selector: host foundation

The Rust audio owner now exposes configured logical Media choices and resolves
selected IDs through its private PipeWire profile. A native Qt Sounds window and
bounded Unix-socket client compile and pass their host protocol tests. This
checkpoint does not yet qualify normal UI input through an installed ARM audio
graph, PCM content, a production launcher, or physical audio.

## Logical routes

Trusted profiles use one schema, without an old-format adapter:

```json
{"sources":{"media":"source.media"},"routes":[{"id":"media-a7","label":"Speaker","target":"route.speaker","roles":["media"]}]}
```

There are1–32 configured routes. IDs are unique1–64-byte ASCII letters, digits,
underscore, dot or hyphen. Labels are nonblank/control-free UTF-8 of at most96
bytes. Private targets are nonempty/control-free and at most256bytes; each route
has1–3 distinct known roles. Node names belong to the backend profile, while
shared consumers submit logical selectors.

Only successful `status` replies add `media_routes`, sorted by ID and filtered to
Media-capable choices. Each entry contains only `id` and `label`. This is a
configured catalog, not discovery or proof that a selected target exists.
Acquire/route replies retain their existing shape and omit the catalog. Existing
restricted graph diagnostics remain; the app does not interpret their node or
port names. Capture and call authority remain distinct from Media selection.

## Current application

An acknowledgement accepts intent. `application:observed` additionally requires
the latest matching-token/current-intent reply to be available and to contain
exactly the desired active owned-source links. Missing, wrong, duplicate or
nonactive links withdraw application. Unrelated sink-monitor links lie outside
that owned-source set. A read-only refresh can observe restoration of the same
intent; it never recreates links or requests reconciliation.

The first actual graph RED accepted a catalog-selected opaque ID but never
created its private-target link. Only target resolution changed for GREEN. A
separate real same-daemon RED removed an applied link and then substituted a
wrong target: three fresh observations of each still reported historical
application. Only the owner's observation update changed for that GREEN. The
same fixed controller then observes withdrawal and restoration, with unchanged
intent/epoch and no extra acquire, route or reconcile.

## Sounds client

The presentation uses Qt/QML and the Rust service remains the route authority.
One persistent authenticated connection owns one Media lease. The event loop
uses nonblocking `QLocalSocket`, checks the service UID through `SO_PEERCRED`,
allows one request in flight and bounds replies to64KiB. Each connect/request has
an absolute one-second BOOTTIME deadline, checked by a watchdog and again around
reply parsing. No thread or blocked worker is detached.

The client obtains IDs/labels from status, sends logical acquire/route requests,
and keeps requested output separate from matching observed application. It
handles call interruption/restoration without acquiring another lease. Backend
unavailability disables mutations; this app cannot reconcile the owner.
Connection loss clears the lease and choices. Reconnect requires explicit user
input and starts with status, without replaying an uncertain mutation. Closing
the window closes its connection; eventual link cleanup still needs an independent
graph observation.

`native-audio-ui` builds `pokelinux-sounds` and eleven actual Unix-socket test
cases with strict compiler warnings. The selected normal binary expects its own
UID11042 and service UID11040; the planned call fixture uses11041, distinct from
existing Camera and Messages principals. Its six arguments select app HOME,
runtime directory, Wayland display, appearance HOME, audio socket and installed
appearance QML directory. This foundation still uses the shared file appearance
reader. The separate qualified native palette client must be composed before a
private-HOME Omarchy theme run; no HOME ACL or copied palette is a substitute.
There is no registered Sounds desktop launcher or installed service unit here.

The normal binary emits at most eight distinct bounded mapped-frame metadata
lines for independent display qualification. Compilation is not evidence that
its window has the required DPR or that a capture has native dimensions.
Future rendering must qualify the [Pixel10 profile](native-phone-display-profile.md)
and unscaled1080×2424 capture before screenshots are presented.

## Recorded checks

| Boundary | Qualified evidence |
|---|---|
| Rust components |12 host tests: five existing authority cases and seven profile-boundary cases; fmt/clippy with warnings denied; host and ARM ELF compilation |
| Original host daemons | PipeWire1.4.2/WirePlumber0.5.8; six catalog/authority/recovery stages,13 current-graph observations, original16-stage regression and a separate stalled-PipeWire Stop control pass on final source |
| Qt host socket client | Qt6.8.2; compiled RED has six missing acquire requests after successful credential/catalog setup; source-only dispatch GREEN passes all11 cases, with unchanged test bytes |
| Normal Sounds | Strict host compilation, distinct-principal/display-metadata cohort also passes the same11 protocol cases; no GUI execution claimed |
| ARM and hardware | Audio ARM ELF183 compilation only in this checkpoint; normal UI, graph/sample composition, matched Google kernel audio and physical devices remain unqualified |

The Qt cases cover exact64-bit lease reuse, pending/observed distinction, call
interruption/restoration, backend loss, explicit reconnect/no replay, connection
close, wrong service UID, an actual delayed event-loop reply, oversized frames,
private-target catalog refusal and regressing graph observations. Their peer is
authored: they do not qualify real owner recovery or eventual graph cleanup.
The delayed reply qualifies deadline refusal, not actual system suspend.

New GREEN daemon runs retain explicit local-daemon identity, actual waits/EOF and
exact missing-CID diagnostics. Earlier receipts used a weaker inspect-error
predicate; separate fresh same-daemon audits establish present absence rather
than inventing original-time observations. Hash-only bindings are in
[evidence](evidence/native-audio-output-selector-provenance.json). Proprietary
inputs and private raw logs are excluded.

## Remaining integration

Continue normal installed ARM UI input through the Rust owner to actual virtual
links and independent sink PCM, call preemption/restoration, close/lease cleanup,
backend loss/no implicit replay, and a live original Omarchy theme change. The
current signed ARM package preparation is not that integration result.

Current per-app lifecycle/foreground permission, a normal launcher, service
management, PCM/FD brokerage, durable recovery, desktop WirePlumber coexistence,
ALSA/USB/Bluetooth, AoC/DSP/microphone/mute, DND/volume/haptics, suspend/idle power,
matched Pixel kernel and physical/carrier qualification remain open. No phone
was accessed and no phone model is qualified by these tests.
