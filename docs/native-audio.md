# Native audio authority and original PipeWire boundary

The `native-audio` crate implements a device-neutral Rust focus and route
service with a real PipeWire backend. Its qualified scope is a private graph
with preallocated mono sources and routes. It is an early phone-service
component, not a finished Pixel audio implementation.

## Ownership and permissions

The service receives bounded newline-delimited JSON over a Unix socket.
Kernel `SO_PEERCRED` supplies the caller UID. Deployment must give the audio
owner, call owner and application principal distinct UIDs; startup rejects
an overlapping configuration. Neither a claimed UID nor a `media_role=Phone`
property grants call authority. Leases also belong to the authenticated
connection, so another process under the application UID cannot release an
existing connection's lease. Connection loss releases that connection's
leases.

A call lease interrupts media, selects its route and restores the remaining
media lease when released. Capture is denied at startup. Only the configured
call owner can grant or revoke capture, and revocation removes capture intent
and the real microphone-source link. Media and call sources are distinct
profile entries. Routes declare their allowed roles. The service has no
Pixel device names, Binder parcels or vendor protocol IDs.

The PipeWire runtime directory is private to the audio owner. Applications
can reach the service socket but cannot bypass it through either PipeWire
socket. This prototype qualifies one application UID; it does not provide
per-app sandbox identities, a production PCM/FD broker, or a policy for all
existing Omarchy applications that share one Unix UID.

PipeWire transports audio; session-manager policy decides linking and access.
The fixture runs WirePlumber with hardware monitors and its standard linking
policy disabled, making Rust the sole link owner. This proves the Rust policy
against actual original daemons. It does not prove integration with the stock
desktop WirePlumber policy or enable Bluetooth/ALSA device management.

## Desired state, observations and process lifetime

Responses distinguish desired lease state from observed graph links, backend
availability, graph epoch and observation revision. A graph mutation succeeds
only after actual PipeWire links are observed active. The backend discovers
current ports and rejects unsupported channel layouts rather than guessing.
It invokes documented `pw-dump`/`pw-link` programs as owned children; a native
libpipewire event binding and lower polling overhead remain future work.

PipeWire death invalidates the observation and advances the epoch while
retaining authorized intent. Restart does not silently repeat mutations. The
call owner must request reconciliation explicitly. A client with no leases
disconnecting does not trigger reconciliation. An uncertain command returns
its error and retained lease/desired state; callers must query state instead
of blindly retrying. Lease IDs are valid only during their current IPC
connection. Service restart loses its in-memory leases and capture grants;
durable call-service reconciliation is not implemented here.

Each active link belongs to a non-lingering `pw-link -m` child. All graph
children are spawned by the stable main actor thread. A safe Rust worker sets
Linux's parent-death signal before checking the expected parent PID, then
executes the original program. The post-registration check catches the race
where the original parent died between fork and signal registration. There is
no unsafe `pre_exec`, reliance on `Child` drop to detach workers, or cosmetic
post-crash unlink. Normal cleanup explicitly kills and reaps children with a
shared deadline; failure exits the authority rather than claiming cleanup.
Fixture PID 1 also reaps adopted workers during crash tests.

The service bounds connections (32 total:16 application,8 call owner and8 other
principals). Kernel credentials select the budget before worker admission, so
application connection exhaustion preserves call-owner capacity. Applications
share at most32 of the64 lease slots; an authenticated call can still interrupt
media after the application budget is exhausted. It also bounds request bytes (4096), queued
messages (64), snapshot bytes (1 MiB), tool execution and cleanup deadlines,
and queued-request response lifetime. Timed-out queued requests are marked
cancelled before the actor can apply them. These bounds are not a production
latency or power-efficiency qualification.
The actor currently waits synchronously for snapshots and reconciliation;
operations and a status request queued behind them can take seconds. The
150 ms polling interval is a scheduling trigger, not a responsiveness guarantee.
There is no installed stop protocol, service-manager unit or event-driven
production loop yet. This evidence does not qualify phone idle power.

The snapshot stdout reader remains owned and is joined after its child is
reaped. EOF after reaping is qualified only for the recorded, trusted
`pw-dump`/`pw-link` tools and container configuration, which execute the fixed
original programs directly. It is not a bound for an arbitrary tool or plugin
that launches descendants retaining stdout. Changing that scope requires
further lifetime qualification; the implementation does not detach a blocked
reader and continue claiming success.


## Verification

Use temporary build capacity; Harbour is not a runtime dependency:

```sh
tools/build-native-audio.sh prepare
tools/build-native-audio.sh build
tools/build-native-audio.sh integration artifacts/native-audio/NEW_RUN
```

Preparation installs packages in a dedicated build image only. Compilation,
component checks and runtime tests use network-none containers. Runtime is
unprivileged, read-only apart from a private tmpfs and its evidence output;
there are no host sound devices, host D-Bus mounts, host network interfaces,
proprietary inputs or physical phone access.

Red-green component tests cover call interruption/restoration, connection
ownership, capture revocation, UID separation and privileged admission under
application lease exhaustion. All five pass on x86 and ARM64 user-QEMU. The
real daemon test passes14 stages with22 recorded graph snapshots and checks:

- Actual active links and running virtual sources/null sinks.
- Spoofed call authority and direct application access to the private graph.
- Call interruption, owned route changes and restoration of media.
- Call admission after application connection and lease budgets are exhausted.
- Capture revocation and rejection of subsequent capture.
- Connection death, PipeWire death and explicit recovery after restart.
- No uncertain-intent reapplication when an unrelated connection drops.
- A registered real worker whose original parent exits before its guard.
- Authority `SIGKILL` removing an active virtual capture link.

The original stack is PipeWire 1.4.2 and WirePlumber 0.5.8 from the dedicated
Debian container. Source versions and Cargo dependencies are recorded in
ignored evidence. `artifacts/native-audio/` preserves the initial failing
component and daemon-boundary runs, the two real lifetime regressions, graph
snapshots, final verification and child-reaping receipts. ARM64 component
execution uses QEMU user mode; the original-daemon integration executes on
native x86-64. Those evidence classes must remain separate. The public
[provenance](evidence/native-audio-provenance.json) records the current source
hashes, real fixture cleanup receipts and retained runtime red phases. Both
application-exhaustion regressions failed at runtime before the budget fixes;
build or fixture setup failures are recorded separately.

## Remaining phone and Pixel gates

This implements virtual-graph focus, routing and capture-link permission
behavior. It does not implement ringtone/DND/alarm/vibration policy, volume
calibration, HFP controls, a general app streaming API, device discovery or
installed Omarchy service management. Removing a virtual graph link does not
prove physical microphone mute, DSP shutdown, acoustic silence or two-way
carrier audio.

The Pixel audit found a separate AoC RTP codec transport and telephony routing
path. The matched closed HAL's mute operation reaches both ordinary streams
and telephony. Native replacement still needs those contracts, matched kernel
and firmware qualification, USB/Bluetooth routing, carrier audio, service
recovery, power/wake coordination and later authorized hardware tests. No
proprietary binary is executed or published by this suite.

## Primary references

- [PipeWire session-manager responsibilities](https://docs.pipewire.org/page_session_manager.html)
- [PipeWire link command and non-lingering monitor lifetime](https://docs.pipewire.org/page_man_pw-link_1.html)
- [Exact PipeWire 1.4.2 access-module source](https://github.com/PipeWire/pipewire/blob/1.4.2/src/modules/module-access.c)
- [WirePlumber linking policy](https://pipewire.pages.freedesktop.org/wireplumber/policies/linking.html)
- [WirePlumber components/profiles](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/components_and_profiles.html)
- [Linux parent-death signal contract](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)
