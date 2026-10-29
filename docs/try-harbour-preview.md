# Try the Harbour VM preview

This disposable development preview runs genuine Omarchy and native Sounds using
the retained, tested Harbour seed. It is not a phone image or a general installer.
The launch command and an external automated RFB client pass end to end. The
human SSH/VNC workflow below is a recipe awaiting its first human validation.

Use a new case name each time; existing directories are deliberately never reused.
The example below uses `human-demo-1`. Keep both terminal sessions open.

On your computer, establish the tunnel using your usual Harbour SSH destination
(replace `ubuntu@harbour` if your existing login uses another name):

```sh
ssh -N -o ExitOnForwardFailure=yes \
  -L 127.0.0.1:5901:/tmp/pokelinux-native-x86-composition-human-demo-1/viewer.sock \
  ubuntu@harbour
```

OpenSSH supports forwarding a local TCP port to a remote Unix socket; the explicit
loopback address keeps this local port on your computer.
[OpenSSH manual](https://man.openbsd.org/ssh#L).
The tunnel can start before the viewer socket exists. Successful tunnel creation
alone does not establish that the viewer is ready.

In a second terminal on Harbour, run as `ubuntu`:

```sh
python3 /home/ubuntu/Projects/pokelinux/tools/native-omarchy-human-preview/launch.py \
  --base /tmp/pokelinux-native-x86-composition-human-preview-root-1 \
  --name human-demo-1
```

Wait for a JSON line containing `"viewer_ready": true`. Then connect an RFB/VNC
client to `127.0.0.1:5901`, using the None/no-password VNC authentication mode inside
your authenticated SSH tunnel. No particular human VNC client is qualified yet.
The server allows 60 seconds for the first connection and a maximum five-minute
session. Disconnecting the viewer closes the VM; it is a single-client session.
Change the case name in both commands for a later attempt.

Use the Omarchy bar → Apps → Sounds. Sound permission remains withheld, so audio
controls are disabled. Human client behavior, longer interactive sessions, consent,
playback and phone hardware remain open. Do not interpret the returned framebuffer
size as fresh measurement of the application's QWindow density; that distinction
is documented in the [normal-entry checkpoint](native-omarchy-human-preview.md).

The launcher copies only selected inputs, preserves the seed and private artifacts,
checks the two retained 2GiB free-space floors and the 12GiB available-memory
requirement, and replaces itself with the existing
foreground runner. That runner keeps process/container custody and reports shutdown.
`--prepare-only` performs the copy and validation without starting a VM. The seed,
external offline assets, pinned Docker image and existing passwordless `sudo`
configuration are Harbour prerequisites; this does not provision another host.
