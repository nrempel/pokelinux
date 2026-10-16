# Actual virtual-sink PCM observer

These authored sources are the exact files executed for the qualified host
sample-flow checkpoint. Mount this directory read-only at `/proposal`, a fresh
writable result directory at `/results`, and the selected native audio target
directory read-only at `/target`. The fixture expects
`/target/debug/pokelinux-audio` and UID1000 owner/1001 call/1002 application.
Start it with `python3 -u /proposal/monitor_fixture.py` inside its own original
PipeWire/WirePlumber runtime, with no network, host devices or host D-Bus.
The recorded runtime is Debian PipeWire1.4.2/WirePlumber0.5.8, image
`sha256:b6818cecd61a5e378fecbf25d7b0979360cf29fda9ad0801b4a21e2054f69678`.

After real Media/Call IPC establishes the stage's playback links, run
`python3 -u /proposal/pcm_observer.py STAGE /results/STAGE` as the audio owner.
The five stage names and their fixed expectations are in `ORACLES.json`.
The observer owns only sink-monitor-to-recorder links; it never creates a Media
or Call playback link. Both sink recorders must deliver complete F32LE/48k/mono
windows, including the sink expected silent. It checks actual graph identities,
active monitor links, negotiated format and captured bytes.

A caller must retain and reap the fixture, clients and observer, enforce its
original deadlines, retain applicable pipe EOF and exact same-daemon container
absence, and preserve failed outputs. A successful service reply or missing
samples cannot satisfy the PCM oracle. The qualified machine-specific host driver
remains private and is bound by hash; this directory is not a generic production
launcher. ARM reuse needs its own actual runtime and artifact qualification.

See [the recorded evidence and limits](../../../docs/native-audio-pcm-arm-foundation.md).
