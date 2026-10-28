# Fresh Omarchy and native Sounds startup

One fresh x86 KVM run now starts the selected PID1 profile, initializes genuine
Tokyo Night theme state, brings up the real Omarchy shell and launches the native
Sounds window through the Rust session owner. Real compositor focus identifies
the app's UID11042 process. There are no authored Source snapshots, theme-only
host, manual first Source load or preparatory compositor warmup in this case.

![Fresh Omarchy and Sounds with audio permission withheld](assets/cold-omarchy-sounds-no-grant.png)

The captured Sounds frontend is the frozen selection used by this run. It
predates the current Rust client and playback/level/mute controls in the
repository. A successor run now boots the current app, but authorized playback
with the current service remains an integration gate. The original result does
not qualify those newer controls. The
[hash-only evidence](evidence/native-cold-omarchy-sounds.json) records the compared
frontend source hashes separately from the executed result.

The selected audio daemon also predates the current client: source comparison
finds that its status reply lacks the now-required `policy_settled` and `preview`
fields. Source comparison identifies this mismatch; the new actual baseline
below records the visible failure separately.
A replacement normal x86 Release build from current source passes 56 existing
scoped tests, formatting and strict Clippy; 52 frozen source pins and 17 actual
wait/EOF completions were independently checked. The current owner and appearance
daemon retain exact matching source bindings.

A new actual cold baseline builds and installs the current frontend and Rust
client archives: 34 Qt cases (32 behaviors plus setup/cleanup), five C ABI cases
and 29 appearance checks pass. Genuine Omarchy, fresh Source, the normal current
window and the Pixel10 display profile pass; 48,000 measured initial frames are
silent. Its profile keeps the `source.media` selector but omits the old fixture's
permanent media producer.

This baseline deliberately retains the historical audio daemon. An operator grant
succeeds, followed by exactly one touch on the enabled Reconnect button. Eight
valid subsequent captures still show no route controls and disabled Play. This
is an actual desired UI failure, with complete guest/VM/container closure, rather
than a build or capture setup failure. Play and the new finite-preview observer
never start. The current-daemon replacement run remains pending; operator grant
is test administration, not a user-consent interface. The baseline's overall
functional and positive manager-Stop-after-Ready qualifications remain false.

This run withheld audio permission. The captured “Audio reply refused” message
does not establish that withholding permission was its sole cause: the selected
frontend predates the current actor/client protocol. Independently, direct audio
access was denied and measurement found zero RMS/peak across 48,000 PCM frames.
This does not demonstrate authorized playback or a healthy current app/service pair.
The visible direct-Hyprland startup warning and desktop-oriented shell layout
remain development rough edges; the capture is unmodified.

The native capture is 1080 × 2424 pixels. The actual app QWindow reports DPR2.625,
with the selected420 logical density and approximately422PPI physical metadata.
The integer QScreen DPR3 is retained only as a diagnostic. Native-buffer and PNG
pixel hashes match, with no resizing. This is a simulated Pixel10 display profile,
not a physical Pixel result.

The deployed profile contains253 files. Its genuine Omarchy base has196 files:
54 QML files receive the existing explicit Commons.Color namespace correction,
and142 retain their original bytes. Forty genuine theme assets are included
(the license overlaps the base). The normal Rust provisioner and presentation
launcher are the separately qualified Release binaries. The launcher inherits
only two compositor presentation values and execs Quickshell in the same PID;
Source permissions remain separate.

PID1 reports successful theme oneshot completion with an empty cgroup. The native
owner reaches Ready, and the positive manager Stop-after-Ready case passes.
Audio and appearance finish with their authenticated Stops and PID1 exit0; app,
compositor, guest child, QEMU and exact-container closure all qualify. Selected
inputs remain unchanged. Quickshell closes by manager SIGTERM with successful
unit status; its process exit is not relabeled as exit0. The previous Color and
unset-instance warnings disappear, while missing shell backend/helper integrations
remain visible in the journal. A preceding static preflight label mismatch happened
before container creation and remains recorded separately.

[Hash-only evidence](evidence/native-cold-omarchy-sounds.json) and
[executed consumer sources](../tools/native-cold-omarchy/) record the scope.
Earlier Source/appearance failures remain failed; this new passing epoch does
not change their results.

Remaining gates include actual failed-provision PID1 propagation, authorized
permission and playback flows in this cold composition, live theme changes,
launcher/menu interactions, trusted authentication/locking, shell bindings to
native phone services, ARM graphical integration and a reproducible full preview.
Calls/IMS, carrier SMS, camera, suspend and every physical-phone gate remain open.
This one cold-start slice is not completion of the smartphone system.
