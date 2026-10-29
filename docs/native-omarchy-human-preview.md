# Normal app entry through an external viewer

A fresh cold x86 VM now runs genuine Omarchy with Sounds selected directly as
`/usr/libexec/pokelinux/pokelinux-sounds`. An external automated RFB client opens
bar → Apps → Sounds. The guest observes the actor-owned native executable and
six arguments, including its protected per-app Wayland socket. There is no app
capture wrapper or capture-file reset in this session.

The unchanged profile consumer first fails against the previous wrapper selection,
then passes the direct executable with UID 11042 and all six arguments preserved.
This configuration check is separate from the later full VM result.

The fresh deployment uses the matched normal audio Main and session owner from the
[Rust startup correction](native-audio-startup-deadline.md). Actor 1409 launches
Sounds 1893; the retained app ELF matches the previous normal build. Permission stays
withheld throughout. This does not qualify playback or visible permission controls.

## Session lifetime

The viewer starts after actual shell, Source, appearance and display discovery.
Guest and host clocks remain independent. Each side retains its original work
limit and a maximum 300-second dwell; the server allows 60 seconds for a first
connection. No internal automated viewer worker is attached in this mode.

Separate actual Xvfb/x11vnc boundary checks pass the full 60-second no-client timeout
(server exit 2 after owned Stop) and an RFB-init disconnect (server exit 0). Their
post-readiness prerequisites are synthetic; these checks are not VM evidence.

The full VM separately passes the external client's three menu actions, normal-app
process observation, complete 1080×2424 frame receipt and RFB EOF. Client disconnect
then leads to normal audio/appearance Stops, manager Stop after Ready, guest
shutdown, QEMU exit 0 and removal of the exact container. Guest and host cleanup
report no errors. No physical phone is accessed.

## What remains

This is an automated external-client checkpoint, not verified human access. A
reproducible operator launch command and SSH/VNC client workflow remain to be
qualified. A connected client remaining until the dwell limit also needs its own
runtime check. No public TCP listener was introduced.

The guest performs no automated input, but the separate external smoke client does.
It uses three fixed coordinates from the prior menu layout; it does not inherit the
previous seven-action test's per-action viewport checks or close/reopen proof.

The native output is discovered and the returned framebuffer is 1080×2424. Normal
app stdout is discarded by the actor, so this run has no fresh QWindow DPR metadata.
Its frame remains private and is not promoted as a newly qualified screenshot. The
[earlier viewer screenshot](native-omarchy-viewer.md) keeps its own measured scope.

[Evidence](evidence/native-omarchy-human-preview.json) and
[original sources](../tools/native-omarchy-human-preview/README.md) record this step.
Phone boot, physical hardware and the full [completion matrix](smartphone-goal.md#completion-matrix)
remain open.
