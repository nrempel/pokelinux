# Native Omarchy viewer input

These original fixture sources implement the qualified private Unix RFB path in
[the viewer report](../../docs/native-omarchy-viewer.md). They are execution
snapshots, not a standalone preview installer or phone component.

Copy the nine selected sources alongside the reconstructed consumers. `guest.py.patch`
and `run-selected.py.patch` apply directly to the named public cold bases in
`consumer-pins.json` using `git apply --unidiff-zero`; they do not stack on the menu
patches. Both reconstructions were checked. Copy the pinned shared `owned.py` from
`tools/native-callback-spans/owned.py`. The complete selected image, readonly VM
backing, deployment manifest, matched SDK/compositor, services and unchanged fixture
dependencies remain necessary.

The fixture uses x11vnc over a private Unix socket, with TCP disabled. Its extracted
bundle is described by `viewer-bundle.json`: Debian trixie x11vnc 0.9.17-1,
libvncclient1/libvncserver1 0.9.15+dfsg-1+deb13u2, libxtst6 2:1.2.5-1 and
liblzo2-2 2.10-3+b1. The original bundle manifest records static extraction evidence;
the later full VM separately qualifies actual execution. No binary packages are
included here.

Boot owns the QEMU, display, server, worker and one initial native-window setup.
Every input action keeps a readonly geometry/focus check. Guest and host clocks
remain separate, with bounded request/ready/commit/done exchanges. Parent retirement
requires actual waits and pipe EOF; it does not claim remote event queues drained.

The small authored-byte parser consumer can be run without a VM:

```sh
python3 -B tools/native-omarchy-viewer/test_rfb_frames.py \
  --module tools/native-omarchy-viewer/rfb_frames.py
```

Its nine cases retain the count, raw-size, encoding, coverage and caller-cutoff
checks while accepting arbitrary protocol padding. The executed baseline failed
on valid nonzero padding; the corrected reader passed. These parser checks alone
do not qualify a socket, UI, human client or phone.


The consent VM now identifies type3 (`ServerCutText`) before its final frame.
The reader discards this unused text without applying it to the host clipboard or
retaining/logging its contents. It accepts at most eight messages of at most4096
bytes each per frame read, under the caller's original deadline. Oversized,
truncated and excess messages remain failures; unknown types still report their
numeric type and fail before reading their payload. See
[RFC6143 section7.6.4](https://www.rfc-editor.org/rfc/rfc6143.html#section-7.6.4).

```sh
python3 -B tools/native-omarchy-viewer/test_rfb_cut_text.py \
  --module tools/native-omarchy-viewer/rfb_frames.py
python3 -B tools/native-omarchy-viewer/test_rfb_message_error.py \
  --module tools/native-omarchy-viewer/rfb_frames.py
```

Eight actual socketpair cases with authored protocol bytes pass strict RED→GREEN,
including bounded text followed by a real decoded frame, empty text, limit-size
messages, and oversized/excess/truncated refusals. The nine original parser cases
and three unsupported-type socket controls pass. An initial struct-packing setup
failure and incorrect authored text-length expectation are preserved separately;
the final corrected consumer is run against both old and repaired readers.
These socket cases do not qualify the complete VM retry or its final shutdown.


A separate real x11vnc/Xvfb pair also passes RED→GREEN with a fixed authored
isolated X11 cut buffer: the old reader observes type3 and refuses it; the repaired
reader discards one29-byte message, receives the full1080×2424 raw frame and
reaches stream EOF. Both cases close the server/display and remove their exact
containers. This reproduces the message-class defect, not the original VM's text
contents or which guest application produced them. The full VM retry remains open.

The separate `selected-qt-cases.patch` replaces the historical fixed Qt34/32
assertion with exact ordered names from frozen `source/qt-cases.json`, matching
total/behavior counts and the selection's expected total. It applies with
`git apply --unidiff-zero` to the eighth permission acceptance runner (SHA256
4e40909c0e12ed81922f0cd4e330bd76bf99d1af118a76850ffb2747bca2394c).
Historical patches and pins above remain unchanged. The eighth guest, PCM and
viewer pass, but its outer result fails on that count. Exact actual-result
RED→GREEN and five altered-result refusal controls pass. A ninth fresh VM run
now passes the repaired checker, guest PCM sequence, final frame/EOF and complete
outer closure. Historical failed results remain unchanged.

### Installed-menu keyboard navigation

The separate `permissions_acceptance_viewer.py` and
`permissions_acceptance_input.py` support the bounded installed-app input flow.
They permit Page Down, Down and Return alongside click, Alt+Tab and normal close.
Each key has one down/up pair; held-key cleanup follows the packet's down bit,
including cancellation during the hold. Requests keep the existing sequence cap,
deadline handoff and no-retry behavior. These helpers do not establish which app
the menu selected; the installed test must verify its actual service/window.

`test_acceptance_navigation.py` runs the real peer subprocess and guest handoff
over a UID1000 Unix socket. Its RFB server and black pixels are authored test
inputs. Normal completion checks exact packets, both frames, EOF and process
wait0. Cancellation checks a matching key-up, failed peer wait1 and no replay.
This is host socket evidence, not GUI, ARM or phone evidence.

From the repository root, as UID/GID1000:

```sh
menu_input_source=$(mktemp -d)
menu_input_checks=$(mktemp -d)
cp tools/native-omarchy-viewer/permissions_acceptance_*.py tools/native-omarchy-viewer/rfb_*.py tools/native-omarchy-viewer/viewer_protocol.py tools/native-omarchy-viewer/viewer_guest.py "$menu_input_source/"
cp tools/native-omarchy-human-preview/human_guest.py tools/native-installed-services/owned.py "$menu_input_source/"
python3 -B tools/native-omarchy-viewer/test_acceptance_navigation.py --host "$menu_input_source" --guest "$menu_input_source" --output "$menu_input_checks/normal"
python3 -B tools/native-omarchy-viewer/test_acceptance_navigation.py --host "$menu_input_source" --guest "$menu_input_source" --output "$menu_input_checks/abort" --abort
```
