# Native radio service installation

This package layout is for the native Samsung radioExternal transport owner and
its service-account diagnostic client. It does not provide a modem service,
native IMS/call state or a qualified Pixel deployment profile.

Install the matching `radio-service`, `radio-worker` and `radio-status` executables
under `/usr/libexec/pokelinux/`, the unit under `/usr/lib/systemd/system/`, and the
sysusers entry under `/usr/lib/sysusers.d/`. Provision the named `pokelinux-radio`
account through sysusers. The unit creates private runtime and persistent state
directories, runs without capabilities, and uses `Restart=no` and
`KillMode=control-group`. Explicit Stop uses the native status client; the service
retains and actually waits its own worker before normal exit. Manager fallback
and process exit never establish remote vendor cleanup.

A root-provisioned `/etc/pokelinux/radio.conf` must select all four values:

```text
BINDER_DEVICE=/absolute/path/to/the/selected/binder/device
BINDER_SERVICE=the.selected.service.name
LEASE_MS=30000
SLOT_ID=1
```

These are placeholders, not Pixel defaults. SLOT_ID must be an explicit
nonnegative i32 selected by the device profile; there is no inferred slot or
modem topology. The device profile must establish the
actual service identity, interface and access policy, grant the service account
access to that device, and order the endpoint provider before this unit. No
Android/vendor producer is installed by this package. The selected lease is
between 1,000 and 86,400,000 milliseconds; the service converts it once to an
absolute CLOCK_BOOTTIME deadline. It never renews that session or implicitly
registers again. This bounded transport lease is not a finished phone lifetime
or recovery policy.

Run `radio-status /run/pokelinux/radio/status.sock status` as the service account
for private diagnostics. Each packet is checked against its connected Linux
principal. The JSON distinguishes `observed-active` transport history from
`unavailable` or `stopping`, and retains one optional historical raw or registration-wire observation.
It does not expose app permissions or cellular/IMS state. `stop` uses the same
restricted socket and observes the exact server process exit after acknowledgement;
that client is not a reaper. Systemd separately owns the service's actual wait.

The admission journal under `/var/lib/pokelinux/radio` is persistent. The service
refuses a new worker if that journal already exists, including after a clean
Stop or failed startup. There is no reset command. Removal is not a supported
recovery operation without separately qualified remote cleanup/reset evidence.
Installed runtime socket removal is systemd's RuntimeDirectory lifecycle, not a
service-side unlink of an unknown socket.

Filesystem startup/stalls, hostile endpoint identity, exact-executable blocked
Binder/output-pressure behavior, installed device policy, suspend/wake and physical
hardware still need their own evidence. Installation in a synthetic ARM guest does
not certify phone deployment.

Status output accepts a pipe, socket or terminal, and rejects regular-file or
block-device stdout because those writes cannot be bounded with `O_NONBLOCK`.
To retain a status result, drain its pipe in a separately owned caller. Stop is
silent and observes the exact server pidfd; the service manager owns its wait.
