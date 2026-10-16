# Actual audio samples and ARM socket foundation

The unchanged final Rust audio owner now has additional actual host PCM
qualification. Separately, the native Sounds protocol client compiles and passes
its eleven socket cases under a generic ARM kernel. These two proofs do not yet
compose normal ARM UI input, an ARM audio graph, private-HOME Omarchy appearance,
or physical audio.

## Actual sample flow

Original PipeWire1.4.2/WirePlumber0.5.8 runs in a private network-free virtual mono
graph. The Rust owner controls playback links. An independent observer creates
only sink-monitor-to-recorder links, verifies their real identities/active state
and negotiated F32LE/48k/mono format, then measures the original `pw-record` bytes.
No expected-text artifact or service response substitutes for samples.

Media uses an observed440Hz test source. The call fixture feeds a finite authored
880Hz input through normal `pw-cat --playback --raw ... -`; that input controls
the source and is never substituted for the captured output. Both sinks must
provide full windows: discard9600 frames, then measure24000. Missing bytes do not
count as silence. Frozen tone/quiet thresholds precede all sample runs and remain
unchanged.

| Stage | Speaker monitor | Earpiece monitor |
|---|---|---|
| Media on speaker |440Hz | Silence |
| Media on earpiece | Silence |440Hz |
| Call preempts media | Silence |880Hz |
| Call release restores media | Silence |440Hz |
| Media release | Silence | Silence |

All ten measured windows pass, totaling240,000 frames. Media RMS is approximately
0.044194 and call RMS0.176777; every required quiet window is exactly zero. Root
independently decoded every retained window, recomputed its tone/RMS/peak checks
and rehashed the final owner artifact and selected sources.

The selected execution retains68 actual waits before their original local
cutoffs, applicable pipe EOF, source invariance and exact same-daemon CID absence.
Ten monitor stdout descriptors are DEVNULL; regular stderr logs are also excluded
from pipe-EOF claims. Native authority termination in this fixture is SIGTERM,
with adopted children reaped; this is separate from the earlier authenticated
Stop control. Local reaping does not establish physical mute or remote cleanup.
The observer's local tool/cleanup/capture budgets qualify this finite run, not
latency, endurance or one immutable process-wide deadline.

Six early setup failures remain preserved. A seventh run retains its actual
complete-window call-tone failure. Only authored call-source plumbing changed
before the subsequent passing run; no production-service RED is claimed. Later
composition uses the final current-observation repair without further production
changes. Exact [observer, fixture and frozen oracles](../tools/native-audio-output-selector/pcm/README.md)
are public; raw captures and machine-specific execution logs remain private.

## Separate actual ARM foundation

A disposable guest on the generic Arch ARM kernel installs32 official signed
packages from a complete recorded dependency closure, leaving all353 existing
package versions unchanged. The retained public Arch signing keyring verifies
every detached signature. Actual ARM executables report PipeWire1.6.9 and
WirePlumber0.5.18; version/help/ELF checks are prerequisites, not a running audio
graph.

Guest-native GCC16.1.1/Qt6.12 compiles the normal Sounds binary and socket test
binary as ELF183. All eleven unchanged actual Unix-socket cases pass, plus test
setup/cleanup. The peer is authored and QTest is GUILESS; this executes real
kernel sockets/credentials/deadline behavior, not a GUI, real Rust audio IPC,
PipeWire daemon routing or PCM. The AudioClient/test bytes match the published
host foundation.

Independent review rehashes87 selected and88 external inputs, verifies the signed
package archive/tool hashes, actual kernel/test logs and49 setup waits/EOFs. Root
also rehashes all87 selected files, eleven published Qt inputs and both ELF183
outputs. QEMU exits0 before a closed97,517,568-byte QCOW2 layer is exported. The
exact same-daemon container is absent and source/base inputs remain unchanged.
Future runs must use a fresh disposable overlay above that read-only layer and
its exact retained root backing; the layer is test preparation, not a phone image.

This ARM binary uses the earlier file appearance reader and qualifies no private
HOME theme composition. The next normal UI cohort must use the current matching
schema3 Rust appearance owner/client, with no old-schema fallback. The older
schema1 palette execution remains reference evidence only.

## Next tracer and hardware gates

Continue normal Qt input through the Rust owner to an actual ARM virtual graph
and independently measured PCM, with call interruption/restoration, real window
close and lease cleanup, backend loss/no replay, and a live genuine Omarchy theme
change. Qualify the actual Pixel10 output/window metadata and unscaled native
capture before presenting screenshots. No normal Sounds UI result is claimed here.

Production launcher/current lifecycle permission, installed service management,
app PCM brokerage, durable recovery, desktop audio-policy coexistence, physical
speaker/microphone/mute, AoC/DSP, USB/Bluetooth, volume/DND/haptics, suspend/power,
matched Google-kernel audio and carrier/device qualification remain open. No
physical phone was accessed. [Hash-only provenance](evidence/native-audio-pcm-arm-foundation-provenance.json).
