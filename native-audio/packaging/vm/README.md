# Explicit VM private audio backend

This site bundle selects a private PipeWire graph for a disposable generic ARM
VM. It is not enabled by the generic audio unit and supplies no Pixel hardware
routes. Install through the explicit authenticated `install-native-services`
stager, with the existing audio unit/sysusers rule, selected audio owner and
`pokelinux-audio-backend-ready` Rust helper.

| Source | Installed path |
| --- | --- |
| Three `.service` files and the `.target` | `/usr/lib/systemd/system/`, same basenames |
| `20-vm-backend.conf` | `/usr/lib/systemd/system/pokelinux-audio.service.d/20-vm-backend.conf` |
| `pokelinux-audio-backend.tmpfiles` | `/usr/lib/tmpfiles.d/pokelinux-audio-backend.conf` |
| `audio-profile.json` | `/etc/pokelinux/audio-profile.json` |
| `audio.env` | `/etc/pokelinux/audio.env`, root0600 |
| `pipewire.conf` | `/etc/pokelinux/vm-audio/pipewire.conf` |
| `wireplumber.conf` | `/etc/pokelinux/vm-audio/wireplumber/wireplumber.conf.d/native-audio.conf` |

All other files are root0644; executables are root0755. The site retains the
byte-identical original profile at `/etc/pokelinux/audio-profile.json`. Its test
roles are CALL_UID0 and APP_UID11042, distinct from backend UID11040. Set newly
assembled bundle file modes to the manifest before staging; Git does not encode
0600 for the environment file. Installation issues no role admission, permission
grant, lease or playback request.

Create the selected sysusers/tmpfiles entries during image preparation, then
enable all three backend services, the audio service and the backend target while
stopped. This bundle performs no activation on the build host. The one tmpfiles
allocation owns shared runtime/state directories at11040:11040/0700; stopping one
service cannot remove another service's sockets. No desktop bus is borrowed.

Bus and PipeWire ExecStartPost use the Rust helper to wait up to five BOOTTIME
seconds for a nonblocking connection with the expected MainPID/UID. Startup is
limited to15s and shutdown to5s. `After=` includes ExecStartPost completion,
[as documented by systemd](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#ExecStartPre=).
No implicit restart or fixed startup sleep hides an unavailable endpoint.

Stop the backend target to stop the whole selected stack: Rust drains first,
then WirePlumber, PipeWire and the private bus. Rust retains its mixed shutdown;
the upstream daemon units use cgroup shutdown. PID1 service outcomes and cgroup
death are distinct from direct-parent reaping and vendor cleanup.

The graph has a driver, two virtual test sources and three null sinks. Hardware,
standard linking policy, logind and portal policy are disabled. The availability
probe must observe no source links, desired route or lease; it never runs a tone,
source.call producer or PCM client. This establishes no user playback, microphone
capture, phone service permission, physical route or suspend behavior.
