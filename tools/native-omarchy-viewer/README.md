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


Unsupported server messages remain refused. The error now records only the numeric
message type, without reading or publishing its payload. Three actual socketpair
controls with authored types pass RED→GREEN for that diagnostic and verify the
payload remains unread; the nine existing parser checks also pass:

```sh
python3 -B tools/native-omarchy-viewer/test_rfb_message_error.py \
  --module tools/native-omarchy-viewer/rfb_frames.py
```

This does not identify the message rejected by the earlier joined consent/audio
VM: its original error omitted the byte. No additional RFB message is accepted,
and the final-frame/EOF requirement remains unchanged.
