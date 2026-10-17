# Brightness service deployment

Install the selected service executable as `/usr/libexec/pokelinux/brightness-service`
and the unit as `/usr/lib/systemd/system/pokelinux-brightness.service`.
The required root-owned `/etc/pokelinux/brightness.conf` explicitly supplies
`POKELINUX_BRIGHTNESS_BACKEND`, `POKELINUX_BRIGHTNESS_CLASS_ROOT` and
`POKELINUX_BRIGHTNESS_DEVICE`; this packaging supplies no default device or backend.

`linux-backlight` requires `/sys/class/backlight` and the explicitly selected
backlight device. `synthetic-tree` selects a disposable filesystem fixture and
must be chosen explicitly by its deployment. An unavailable backend never
selects the other backend automatically. The process tests use only temporary
SyntheticTree attributes and establish no driver or physical luminance result.

The private runtime directory is root:root 0700 and its SEQPACKET endpoint is 0600.
Only a trusted root lifecycle actor may Inspect or send an explicit Set; apps
receive no endpoint or device-path authority. The actor keeps its verified
connection from Inspect for a later Set. Polling never reconnects or sends Set.

Stop fences publication and retains a live worker until actual wait and PIPE EOF.
The unit also owns the entire cgroup and bounds shutdown; killing it does not
roll back a property write that might already have reached the selected backend.

The production entry assumes a trusted single-threaded exec with normal SIGCHLD
wait semantics. Its termination guard separately checks inherited SIGTERM state.

The child checks the original BOOTTIME cutoff after preflight and immediately
before mutating open/write. Clock checks and a kernel syscall are not atomic:
an already-admitted write may finish later. Cancellation and reap do not roll
back a property update.
