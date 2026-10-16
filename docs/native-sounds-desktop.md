# Fixed Sounds desktop launch

The lifecycle actor now accepts the fixed `launch_sounds` request on
`launch-sounds.sock`, independently of a Messages provider. Actual desktop
UID1000 may request only the root-installed `org.omarchy.Sounds` profile.
Unknown verbs, extra fields and other UIDs are denied. The endpoint accepts no
grant, route, executable or arbitrary app selection.

The request uses the existing `Apps` reservation, original BOOTTIME setup deadline,
owned child and per-app generation. Reply publication acknowledges the launch;
client loss during setup retains the existing cancellation path. Duplicate launch
is denied while the slot is occupied. Stop fences authority and retains the exact
child until the actor actually waits it. Messages keeps its own endpoint and
provider-dependent availability; Sounds creates no Messages recovery intent.

## Client and install metadata

`native-lifecycle` builds the small Rust `sounds-launch` client. It accepts one
absolute endpoint path, sends only `launch_sounds`, and uses the existing bounded
credentialed IPC helper. A successful reply requires root UID; this trusts the
root deployment rather than uniquely identifying a particular root actor.

The Sounds CMake target provides a desktop-file install rule. Its fixed entry
selects `/run/pokelinux/session/launch-sounds.sock`. The Rust helper belongs at
`/usr/libexec/pokelinux/sounds-launch`. The explicit ARM artifact index now includes
both the existing `lifecycle-lock` binary and this new client. No executable glob
or dependency version change was added.

These are compiled client and desktop metadata, not an executed desktop install.
The installed session owner, compositor Child custody, trusted profile, Source,
audio provider and independent policy provisioning remain required. The existing
headless actor supplies no Source/audio provider automatically. A desktop request
cannot grant `MediaRoute`, clear a durable lock or revive a failed Source.

## Executed host boundary

The baseline first wires the strict real socket but deliberately returns
`Unavailable` without reserving or spawning Sounds. This preparation is explicit;
a missing socket or compiler error is not the behavioral RED. Six real Linux
socket cases then produce three passing controls and three `Unavailable` versus
`Ok` failures. An actual UID1000 Messages launch and matching UID11010 child/wait
control establish working entry and ownership setup.

Only the missing Sounds dispatch branch changes for GREEN. The identical six
tests pass: actual UID denial, strict verbs on both endpoints, absent profile,
launch with no Messages provider, duplicate refusal preserving the same child,
and Stop with the exact actor-owned child wait. Apps execute `/usr/bin/sleep` as
UID11042 with empty supplementary groups; this is not the normal Sounds GUI.
Fourteen separate clients actually wait and reach both PIPE EOFs. Four app wait
records come from the existing bounded test observer after `Child::try_wait`:
three Sounds children and one Messages child. Their observation times bound the
actual waits; they are not exported production numeric-exit receipts.

The final packaging adds six explicit root-only ignore annotations without
changing any assertion body. With its required fixture root and built helpers,
the default library suite passes64 tests with20 ignored, followed by all six
explicit launch cases. Formatting, all-target Clippy, five host binary builds,
ARM64 binary builds and ARM library-test compilation pass. ARM compilation is
not ARM execution. The native fixed client itself is compiled, without a claimed
actual desktop invocation in this selection.

Two earlier preparation failures remain excluded: one existing test literal
needed the new private endpoint type, and the first runtime had insufficient
private `/opt` tmpfs space. A later packaging run omitted the required test-root
directory; its45-pass/19-setup-failure result is preserved. Source-unchanged runtime
setup supplies the directory and daemon helper, then passes64 plus six. Passed
host checks were retained; the not-yet-reached ARM compilation ran separately.
All selected controller waits, original cutoffs, merged PIPE EOFs, source pins,
exact container removal and same-daemon closure qualify.

[Hash-only provenance](evidence/native-sounds-desktop-provenance.json) separates
the scaffold, genuine RED, source-only GREEN and packaging validation.

## Normal ARM join remains open

The newer permission-aware normal Sounds binary separately passes30 ARM socket
behaviors plus setup/cleanup. Its normal actor composition has produced no
qualified frame or PCM group. A bounded passive replay binds the exact ELF,
UID11042 and correct capture descriptors, then observes early app death. A
separate private one-statement parent-wait diagnostic records `SIGXFSZ`25 with a
matching current-boot/exact-PID journal signal, while the private capture wrapper
imposes a global4MiB file-size limit. The particular write/file was not observed.
No raw core, memory, backtrace or environment was read or published.

The next controlled fixture repair replaces that global limit with dedicated
bounded capture storage, retaining the existing capture and functional oracles.
The diagnostic actor is not a production pass. Preserve the completed historical
[direct-stream ARM tracer](native-audio-arm-ui.md) as its separate selection.

Next qualify normal desktop launch through the installed Source/audio assembly:
genuine focus plus an independent grant permits one selection; null focus denies
new choices while an approved background lease may continue; terminal Source
loss retires app/audio ownership without revival or replay. Trusted restoration,
PAM/secure frames, full-shell installation, physical routes/AoC, carrier audio,
Pixel kernel/device policy and all physical phone qualification remain open.
