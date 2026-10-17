# Native radio service installation

This package layout is for the native Samsung radioExternal transport owner and
its service-account diagnostic client and root-only one-shot operator. It does
not provide a modem service,
native IMS/call state or a qualified Pixel deployment profile.

Install the matching `radio-service`, `radio-worker` and `radio-status` executables
under `/usr/libexec/pokelinux/`, the unit under `/usr/lib/systemd/system/`, and the
sysusers entry under `/usr/lib/sysusers.d/`. Provision the named `pokelinux-radio`
account through sysusers. The unit creates private runtime and persistent state
directories, runs without capabilities, and uses `Restart=no` and
`KillMode=mixed`. There is no ExecStop command. Manager Stop sends SIGTERM to
Main, whose retained Rust termination guard fences work and initiates the same
once-only cleanup as socket Stop. Main owns and actually waits both its transport
worker and finite intent writer before normal exit. The two-second native cleanup
budget and five-second manager fallback are separate. Neither process exit nor
manager fallback establishes remote vendor cleanup.

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

Before startup, also provision `/etc/pokelinux/radio-add-pdn.json` as a regular,
non-symlink root:service-group file with mode0640, in a root:service-group0750
`/etc/pokelinux` directory. Main validates these exact modes/ownership and copies
the strict schema1 typed configuration before spawning. See the
[operator contract](../../docs/native-radio-operator.md) and
[authored test configuration](../../tools/native-radio-operator/config.json).
The fixture values are public synthetic inputs; they are not a Pixel/carrier
profile. Full-system composition of this shared configuration directory remains
an explicit deployment gate. Startup filesystem work can block; live commands do
not open or sync these files.

Root0:0 may admit one command03 with the current incarnation, fixed route,
cached payload digest and original BOOTTIME deadline. This carries neither a raw
payload nor a filesystem path. Main reserves the operation, then requires durable
writer sync, its matching Finish and actual writer wait0 before one worker permit.
This privileged backend interface is not an app permission API. It provides no
second operation or automatic retry.

Run `radio-status /run/pokelinux/radio/status.sock status` as the service account
for private diagnostics. Each packet is checked against its connected Linux
principal. The JSON distinguishes `observed-active` transport history from
`unavailable` or `stopping`, and retains one optional historical raw or
registration-wire observation plus
independent typed `request_history`. RADS version2 is the only current protocol;
request-history revisions are separate from transport revisions. Root operator
authority does not grant service-account Status/Stop access.
It does not expose app permissions or cellular/IMS state. `stop` uses the same
restricted socket and observes the exact server process exit after acknowledgement;
that client is not a reaper. Systemd separately owns the service's actual wait.

Registration admission, writer-launch and add-PDN operation records under
`/var/lib/pokelinux/radio` are persistent uncertainty fences. Any preexisting marker
refuses startup with exit78 before configuration input, spawning or Binder lookup,
including after a clean Stop or failed startup. There is no reset command.
Removal is not a supported
recovery operation without separately qualified remote cleanup/reset evidence.
Installed runtime socket removal is systemd's RuntimeDirectory lifecycle, not a
service-side unlink of an unknown socket.

Startup filesystem stalls, production endpoint identity, actual unkillable tasks,
manager fallback, installed device policy, suspend/wake and physical hardware still
need their own evidence. Generic ARM held-worker/writer checks qualify their
recorded local lifecycle boundaries only; they prove no vendor reset.
Installation in a synthetic ARM guest does
not certify phone deployment.

Status output accepts a pipe, socket or terminal, and rejects regular-file or
block-device stdout because those writes cannot be bounded with `O_NONBLOCK`.
To retain a status result, drain its pipe in a separately owned caller. Stop is
silent and observes the exact server pidfd; the service manager owns its wait.
