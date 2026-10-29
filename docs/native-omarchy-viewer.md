# Genuine Omarchy through viewer input

A complete cold x86 VM run now passes viewer-driven menu launch, Sounds close,
fresh menu reopen, final image receipt and normal shutdown. It uses genuine
Omarchy, the Rust session owner and actual native app windows at the
[Pixel 10 display profile](native-phone-display-profile.md).

<img src="assets/omarchy-viewer-sounds-reopened.png" width="320" alt="The reopened native Sounds app received through the viewer connection" />

This image is decoded losslessly from the actual 1080×2424 RFB framebuffer; no
capture resizing or content editing was used. Permission remains withheld, so
route and Play controls are disabled. This milestone does not demonstrate playback.

## What passed

Seven bounded actions travel through a private Unix RFB connection, x11vnc, SDL,
QEMU and the guest's actual input drivers: bar, Apps, Sounds, Super+W, bar, Apps,
Sounds. The guest opens input devices readonly for identification; it does not
inject uinput events or substitute a compositor close command.

Both app instances retain UID/GID 11042, empty supplementary groups, the same
normal ELF and separate protected Wayland connections. App PIDs change from 2025
to 2278 while the same Rust actor remains. Actual app-window DPR is 2.625, output
is 1080×2424, and both measured Pixel profiles pass. Each app instance also passes
48,000 independent virtual PCM frames of permission-withheld silence.

The final viewer frame contains 10,471,680 raw bytes in one complete rectangle.
The actor receives normal manager Stop after Ready; actual guest shutdown, QEMU
exit 0, socket/pipe EOF and removal of the exact container all pass. Seven host,
eight boot, five bootstrap, 199 guest and seven readonly-helper process receipts
retain actual waits and required EOF within their original limits. Input hashes
remain unchanged and cleanup reports no errors.

## Corrections and preserved failures

The initial guest selected a legacy vmmouse instead of the added virtio tablet.
The explicit `q35,vmport=off` VM selection retains the keyboard and selects the
intended absolute pointer.

The window-manager-free Xvfb display did not apply SDL's fullscreen request:
the actual host window remained 640×480 at (220,972). One owned initial
`XMoveResizeWindow` sets the native viewport before the first RFB frame. A separate
paused, diskless display check verified this primitive; every subsequent input
still requires the exact readonly 1080×2424/origin geometry check. This configures
the host window; it does not resize the guest capture.

The next full run completed the UI sequence but failed the final frame reader's
combined padding/count assertion. Its failed header was not recorded, so its
padding/count values remain unknown. The decoder incorrectly required padding to
be zero, contrary to [RFC 6143 section 7](https://www.rfc-editor.org/rfc/rfc6143.html#section-7).
The same small authored-byte consumer fails before the correction and passes all
nine cases afterward, preserving every image bound and caller cutoff. The complete
subsequent VM run passes and records actual final-frame padding 21 with one rectangle.
Earlier failed runs remain failed; their partial UI and cleanup facts stay separate.

## Limits and next step

This is automated viewer-protocol integration over a private Unix socket, with
no TCP listener. A human client/SSH connection workflow is not yet qualified.
The app capture wrapper remains fixture infrastructure. The next step is a bounded
human preview using the normal app entry point, followed by visible Allow/Revoke.
The separate [Rust startup deadline fix](native-audio-startup-deadline.md) has its
own controlled-host evidence and was not substituted into this frozen viewer cohort.

There is no consent, preview Play, ARM, physical display/touch or phone boot claim
from this run. The [full phone completion matrix](smartphone-goal.md#completion-matrix)
remains open. [Evidence](evidence/native-omarchy-viewer.json) and
[original fixture sources](../tools/native-omarchy-viewer/README.md) describe this checkpoint.
