# Explicit Messages VM profile

Install `messages-profile.json` at `/etc/pokelinux/messages-profile.json`,
root0:0/0644, through the finite authenticated `install-native-services` manifest.
Select the unchanged production `pokelinux-messages.service`, default-feature
`messages-service`/`messages-stop`, and `lifecycle-launcher`. Its normal six-argument
entry opens local history only, with no Binder dependency or receiving selector.
The unit creates its root-owned private state/runtime paths at startup; enable it
while stopped during image preparation. Staging does not activate host services.

The profile names the real normal Messages ARM app at UID11010, its six normal
arguments and the shared installed appearance QML. In the headless history
scenario the executable/QML are protected unlaunched dependencies. No app account,
Wayland session, unlock, focus, permission grant or app data release follows from
this profile. A graphical session still needs its own complete deployment and
qualification; this test checks the production storage service with zero app
principals and locked policy.

The separate `messages-seed-fixture` is built only with `isolated-fixture`, kept
under an explicit fixture path and invoked once during disposable image
preparation with `NATIVE_MESSAGES_ISOLATED=1`. It precommits two authored
unqualified raw GSM occurrences. The production service is never built with that
feature. Require the Inbox database to remain absent and the Messages unit
stopped before retaining the prepared image.

Two distinct subsequent cold boots must perform no seed, installation, reload,
enable or start operation before the frozen history probe. The first boot must
reconcile the exact raw IDs/provenance into two plain-text records; the next must
retain those IDs and contents. Both require actual writer/readonly database
ownership, no incoming operation/counter, native PID1 Stop completion and empty
cgroups. This is clean-shutdown/reboot persistence, not power-loss, incoming SMS,
app-read or carrier evidence.
