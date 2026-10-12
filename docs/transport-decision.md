# Local transport decision

2026-10-12. Source audit of the current Rust and Android implementations.

The implemented control path is QML -> compiled Qt process adapter -> Rust CLI
-> same-UID Unix socket -> Rust daemon -> mutually authenticated TLS 1.3 over
loopback TCP -> Android framework adapter. There is no HTTP server or cloud
dependency. JSON describes bounded control requests and observations. It does
not carry voice media, camera buffers or graphics.

## Decision and remaining deployment condition

Keep the existing authenticated loopback transport for current offline
integration. It supplies a paired identity independent of Android/Linux UID
mapping and uses the AndroidKeyStore identity through the standard Java TLS
implementation. This is not a claim that TCP is required by Android, or that
shared loopback has been proven in the final Pixel system layout.

Android's public [LocalSocket API](https://developer.android.com/reference/android/net/LocalSocket)
supports Unix-domain sockets, peer credentials and file-descriptor passing.
A custom system can use a Unix endpoint with explicit UID/domain policy. Such
a design needs tested endpoint ownership, namespace reachability, peer checks
on both sides and SELinux rules. An abstract socket name alone is not an
identity check. Replacing TCP would not remove command authorization, durable
admission, stale-observation rejection or recovery requirements.

[AIDL has Java, NDK and Rust backends](https://source.android.com/docs/core/architecture/aidl/aidl-backends).
Several service-management APIs remain platform-only, and the Rust backend
depends on Android's native Binder library. Direct Binder from Arch therefore
needs a demonstrated runtime and service-discovery boundary; it is not a
drop-in replacement for this APK endpoint. Binder remains relevant inside the
retained Android stack.

Do not add parallel transport implementations or fallback listeners. Before
freezing the device deployment, compare the implemented transport with a
concrete Unix-socket layout. Choose one target on demonstrated reachability,
identity policy, maintenance and measured wakeup cost. Preserve no legacy
transport solely for earlier test versions.

## Source findings

- `framework/bridge/GatewayService.java` explicitly binds IPv4
  `127.0.0.1:37179`, requires TLS 1.3 client authentication and checks the enrolled
  client leaf pin before parsing a request. Workers and queues are bounded.
- `runtime/src/enrollment.rs` accepts numeric loopback addresses only, requires
  private enrollment ownership/modes and rejects a zero port. It does not
  resolve a configured hostname into a destination. TLS server-name validation
  is separate from the numeric connection address.
- `runtime/src/tls.rs` verifies the certificate chain, server name and leaf pin,
  disables session resumption/early data, uses a total BOOTTIME deadline and
  does not retry a command. A failure after an application write attempt remains
  unknown.
- Android accepts at most 4,096 request bytes and 65,536 response bytes. The
  Rust transport previously used the response limit for requests too. The audit
  added a failing real-socket test and split the limits. Oversized requests now
  fail before connection. The exact maximum request and response still pass.
- `native-app/PhoneObservations.qml` schedules open-app reads every 500 ms,
  alternating controls and dial status when the adapter is available. Each
  remote read creates a new TLS connection. This is not background ringing or
  a measured battery-efficient event channel.

## Evidence and outstanding checks

The isolated native ARM64 Linux TLS suite has eight passing cases. It covers
mutual authentication, wrong root/name/pin, framing, fragmentation, incomplete
and oversized replies, stalled handshakes, total deadlines and request limits.
Logs: `artifacts/rust-native-local/tls-request-limit-{red,green}.log`.
Related Rust regression and clippy/fmt results are saved under
`transport-audit-*` in that directory. These are synthetic-peer tests, not an
Android framework run or a deployed Pixel namespace test.

The changed source was rebuilt as native ARM64 debug binaries and passed all
43 Qt/QML behavior cases (51 Qt entries), including automatic observations and
touch-driven Answer/Dial through the authenticated synthetic peer. This run
used fresh binaries directly, not the earlier 0.1.1 package archive. The archive
has not been rebuilt for this change. See `qt-tested-binaries.sha256`,
`qt-sources.sha256`, `qt-run.json` and `transport-audit-result.json` for scope.

Required offline deployment proof remains:

1. Boot the intended Android/Linux namespace layout with enforcing SELinux;
   run the exact installed daemon and APK with no ADB forwarding. Record socket
   bindings and namespace identities. A different namespace must fail closed,
   without changing to wildcard/LAN binding.
2. Test unpaired Android apps and Linux UIDs, listener port preoccupation,
   unauthenticated connection saturation and restart. A rogue listener must
   receive no application data; saturation must not block local Telecom UI or
   strand the bridge after attackers disconnect. TLS authenticates peers but
   does not prevent local denial of service.
3. Exercise enrollment expiry/replacement, revoked controls/roles, APK and
   daemon restarts, process death after admission, lost replies and exact
   operation recovery through the real framework adapter.
4. Test installed service activation, user-session exit, screen lock and
   suspend/resume. Background incoming-call observation and notifications need
   service ownership independent of an open app.
5. Measure open-app and idle connections, CPU, wakeups and recovery latency.
   Decide event delivery separately from message encoding. JSON versus a binary
   encoding does not by itself solve polling or repeated handshakes.

Carrier registration, IMS, voice audio, real suspend/wake and Pixel hardware
remain hardware-dependent even after the offline deployment checks pass.

## Independent critique

Pro transport review (private transcript)
recommends testing Unix sockets with explicit peer authorization first for a
controlled system image. It considers mTLS defensible where paired identity
simplifies the boundary, and does not consider Binder necessary for these small
messages. This does not establish which option the final system can support.
The next deployment design must test that choice rather than treating APK
constraints or existing code as a reason to keep TCP permanently.

BRIDGE VERIFIED: Chat selected, composer Pro 5 of 5, one marked user turn and
one terminal assistant response, Copy/Rate/Share present and Stop absent;
marker `PRO-BRIDGE-20261012-f91c` matched. Zero context rounds. Owned tab closed
after extraction. Public LocalSocket and AIDL API claims were independently
checked against the primary sources above. This is a design critique, not a
deployment test.
