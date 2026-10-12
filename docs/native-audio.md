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

Responses distinguish admitted desired intent from cached graph observations,
backend availability, graph epoch and observation revision. `ok:true` accepts an
intent; it does not claim that a graph mutation completed. `intent_revision` and
`application` (`pending`, `observed`, or `uncertain`) report application separately.
Only an active-link observation for the matching submitted intent can settle it.
Availability describes the last accepted sample, not instantaneous device health.
The backend discovers current ports and rejects unsupported channel layouts.
It invokes documented `pw-dump`/`pw-link` programs as owned children; a native
libpipewire event binding and lower polling overhead remain future work.

One owned backend process performs original-stack IO. The authority polls bounded
nonblocking pipes and can serve status or authenticated stop while a real tool or
backend is blocked. Each job has a token and an absolute three-second
`CLOCK_BOOTTIME` budget, checked again after consuming reply bytes. Applying links
also checks an inner two-second boot-time deadline after graph replies. Expiry,
malformed output, wrong tokens or process death fence observations before signaling
the child. The authority retains that child until `try_wait` reaps it; it never
starts a replacement while the old process owns capacity. Those budgets qualify
this recorded local boundary, not hard system latency or physical suspend behavior.
Process creation and OS scheduling remain outside a hard response guarantee.

Permanent backend failure retains authorized intent, marks application uncertain
and denies new acquisition/route changes. The call owner must explicitly request
`reconcile` before recovery can apply it. PipeWire death similarly invalidates
observations and advances the graph epoch; daemon restart and an unrelated
connection's departure do not repeat old mutations. Lease IDs remain scoped to
their authenticated IPC connection. Service restart loses in-memory leases and
capture grants; durable call-service reconciliation is not implemented here.

Capture revocation, capture release and a lease-owning connection's departure
retire the current backend before new authorized work can run. This immediately
fences its publication while actual unlinking remains a separate observation.
A privileged `stop` clears intent, advances its revision and returns
`phase:stopping`, `cleanup:pending`; that response does not claim cleanup. The
frontend stops admission, retains and joins its threads, and exits only after
local owners finish. An unreaped worker or unfinished stop owner causes process
abort at a bounded deadline. An unexpected actor exit also refuses to detach a
live frontend. There is no installed service-manager unit yet.

Each active link belongs to a non-lingering `pw-link -m` child spawned by the
stable backend main thread. Both backend and tool worker set Linux's parent-death
signal before checking their expected parent PID. The check catches parent death
between fork and signal registration. Original-stack snapshot readers remain
owned inside that backend; normal helper cleanup explicitly kills/reaps and joins
them. If trusted tool cleanup wedges, the authority can retire the whole backend.
The fixed original tools and container configuration qualify this child tree;
arbitrary plugins and descendants retaining stdout are outside the scope.
Fixture PID1 reaps adopted helpers during crash tests. Reaping proves local
process reclamation, not remote cleanup, physical microphone mute or DSP shutdown.

The service bounds connections (32 total:16 application,8 call owner and8 other
principals). Kernel credentials select the budget before worker admission, so
application connection exhaustion preserves call-owner capacity. Applications
share at most32 of64 lease slots; an authenticated call can still interrupt media
after application exhaustion. Request bytes are bounded to4096, queued messages
to64, snapshot bytes to1MiB and worker frames to65536. Timed-out queued requests
are cancelled before later actor admission. The150ms observation polling trigger
and10ms actor scheduling loop are not an idle-power or general latency guarantee.


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
real daemon test passes16 stages with24 recorded graph snapshots and checks:

- Actual active links and running virtual sources/null sinks.
- Spoofed call authority and direct application access to the private graph.
- Call interruption, owned route changes and restoration of media.
- Call admission after application connection and lease budgets are exhausted.
- Capture revocation and rejection of subsequent capture.
- Connection death, PipeWire death and explicit recovery after restart.
- No uncertain-intent reapplication when an unrelated connection drops.
- A registered real worker whose original parent exits before its guard.
- Authority `SIGKILL` removing an active virtual capture link.
- Responsive status while original PipeWire is stopped, and a permanently stopped
  owned backend killed/reaped with no implicit recovery.

A separate authenticated stop run keeps original PipeWire stopped while the
service clears intent and responds, then observes unlinking after the daemon
resumes. The final runs measured status at2.3ms and stop at0.64ms under these
conditions; those measurements are evidence, not service-level guarantees.

The original stack is PipeWire 1.4.2 and WirePlumber 0.5.8 from the dedicated
Debian container. Source versions and Cargo dependencies are recorded in
ignored evidence. `artifacts/native-audio/` preserves the initial failing
component and daemon-boundary runs, the two real lifetime regressions, graph
snapshots, final verification and child-reaping receipts. ARM64 component
execution uses QEMU user mode; the original-daemon integration executes on
native x86-64. Those evidence classes must remain separate. The initial [provenance](evidence/native-audio-provenance.json) records the
committed synchronous component and its runtime reds. The current
[process provenance](evidence/native-audio-process-provenance.json) records owned
source/binary hashes,16-stage and stop-only results, cleanup receipts and executed
reds: original status blocked for1.79s and privileged stop returned
`invalid_request`. A first supervised run then exposed fresh links being killed
on the first cookie observation; establishing graph identity before linking
made the same capture-revocation/media-restoration case green. Fixture/setup
failures remain separate from those actual behavioral failures.

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
