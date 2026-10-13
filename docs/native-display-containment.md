# Native compositor containment and owned setup

The native Rust display boundary creates a protected application listener through
the original compositor's security-context-v1 protocol. Applications retain their
direct socket PID/UID/GID and focused pidfd; a proxy does not substitute its own
identity. The Rust supervisor owns one limited SDK process and keeps steady-state
status and Stop independent of Wayland read/dispatch.

This is a supervised primitive and an original-compositor integration checkpoint.
The current Messages lifecycle actor does not yet consume it. Production locker
admission, PAM restoration, app-generation lifetime, secure frames and Pixel
deployment remain separate requirements. No physical phone was accessed.

[Clean provenance](evidence/native-display-containment-provenance.json) pins the
current compiled sources and284 retained proof files. Original protocol/Qt,
supervisor and host component selections remain distinct.

## Original source and protocol

The [upstream security-context-v1 XML](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/staging/security-context/security-context-v1.xml)
specifies a prepared listening socket and a close descriptor. The compositor
continues accepting after the creator's Wayland connection disappears, and the
close descriptor's hangup ends future admission. These rules do not imply
disconnection of existing clients. App and instance metadata do not grant phone
permissions or prove a launched process generation.

The selected original Hyprland0.56.2-4 ELF is the same package used by the
[matching-build observation](native-session-build-boundary.md). Public source at
`efb50993780079460b0cbed1363e2166a2de1d9f` explains its implementation:

- [Compositor global filter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/Compositor.cpp)
  is installed before clients connect. The native boundary uses it unchanged.
- [Security context](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/SecurityContext.cpp)
  accepts the actual application socket and preserves its sandboxed-client record.
- [Protocol classification](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/managers/ProtocolManager.cpp)
  determines allowed globals. Clipboard/data protocols remain available; this is
  not a complete information-flow sandbox.

The implementation uses `wayland-client=0.31.15` and
`wayland-protocols=0.32.13`, with the pure Rust backend and generated client/staging
bindings. Cargo.lock pins the complete dependency closure. The independent
bounded Python wire observer is test infrastructure. No proprietary source,
vendor binary or decompiled implementation is published.

## Child protocol driver

`native-display::Context` consumes an established compositor UnixStream and a
prepared UNIX/STREAM/listening/CLOEXEC descriptor. It authenticates actual socket
credentials and compares the socket-origin source pidfd with the already-owned
launched source watch in a pinned protected procfs view. It restores CLOEXEC on
the creator connection before the SDK owns it. The original BOOTTIME deadline
must be future and at most five seconds away.

The driver discovers the v1 manager, supplies the listener and close-pipe read
end, sets canonical metadata, commits, destroys the creator objects and waits
for an ordered original-server sync. It then drops the entire creator transport,
registry, queue and local transferred descriptors. Only its source watch and
close-pipe writer remain. Source death fails the context; Stop drops that writer.

The SDK can drain input and allocate before application validation runs. Outer
deadlines and a bounded advertised-global count cannot interrupt those internal
loops. The driver is therefore confined to the supervised child. Its `poll`
method is not an actor-safe dispatch API.

## Native supervisor ownership

`Supervisor::spawn` qualifies prepared inputs before launching the same protected
Root UID/GID0 executable with a fixed worker selector. A private nonblocking CLOEXEC
seqpacket becomes child stdin through safe `Stdio`; no pre-exec callback or
externally supplied worker socket path is needed. Both endpoints enable
SO_PASSCRED before any bootstrap/reply can queue.

One bounded bootstrap carries an original deadline, source identity, app/instance
metadata and fresh supervisor nonce. Exactly four owned CLOEXEC descriptors carry
the creator connection, expected source watch, prepared listener and parent
watch. Successful transfer drops every parent bootstrap copy. The child verifies
actual Root sender credentials, original parent identity and the already-owned
parent handle against its socket-origin parent watch. Truncated, extra or
unexpected ancillary data is refused and all received handles are dropped.

Before the SDK runs, the child installs PDEATHSIG=SIGKILL, checks the original
parent again, and sets hard/soft address-space128MiB, open-file64 and core0 limits,
plus NoNewPrivs. Its parent watch is also checked between SDK turns; kernel
parent-death signalling remains effective while a turn is blocked or stopped.
These limits bound the SDK process; they do not constitute a complete sandbox.

The parent owns `Child`, its direct unreaped child's pidfd, the original source
watch, the private control endpoint and exact expected Ready identity. Each poll
receives at most one512-byte packet; it never invokes the Wayland SDK. Ready must
match actual child credentials, nonce, instance and original deadline, with no
received descriptors. Source and child must remain live before and after acceptance.
The five-second deadline applies to initialization; a ready context continues
to depend on source/child lifetimes without an invented five-second context lease.

Stop, timeout and observed source/child failure first fence admission, close
control/bootstrap copies and signal the child. Signalling, pidfd readiness and
process absence do not refund ownership. Only actual `Child::try_wait` completion
clears `occupied`. Post-spawn setup errors return terminal owned state, and wait
errors retain it. A caller must continue polling while occupied. Dropping an
unreaped instance fails closed by aborting its owner rather than detaching a
blocked worker. There is no implicit restart or retry.

Spawn includes filesystem/exec startup and is distinct from the bounded
steady-state poll/Stop path. Sole reaping ownership and normal SIGCHLD semantics
and a long-lived spawning actor thread are caller requirements. A local reap
proves descriptor release, not that the
original compositor processed the close-fd hangup or disconnected accepted apps.

## Chronological evidence

Before `native-display` existed, `original-direct-app-red-1` gave UID11010 the
original direct Wayland ACL. Privileged globals were visible, so the desired
absence assertion failed on the actual original ARM compositor. All ten tracked
children were reaped.

The separate listener-family red admitted a real TCP listening descriptor before
the UNIX guard; its desired rejection failed. The creator-exec red-v2 observed
the exact original socket inode inherited through a real exec before CLOEXEC
restoration. Their corrected host-kernel component selections pass. Earlier
compiler/setup diagnostics and the first unsuccessful exec predicate remain
unselected diagnostics rather than behavioral reds.

`original-context-green-4` uses frozen build4 and passes fifteen original ARM
observation groups. An actual UID11010 client has no direct original-runtime
access, binds allowed globals, receives filtered advertisement and gets actual
`wl_display.error` for forged original IDs of session-lock, security-manager and
screencopy. Filtering persists after creator transport disappearance and context
closure. Future connects fail after closure while accepted clients remain filtered.

The unchanged normal production Qt ELF is a native mapped Messages window through
the restricted listener. Its PID362, UID/GID11010 and live socket-origin/SCM_RIGHTS
focus handle match the independently owned Qt child. All seventeen direct
children are reaped. These are fifteen observation groups, not fifteen independent
production policies. The app's data endpoint is absent: Messages body/grant/session
authority does not follow from this identity result.

Before any supervisor source existed, `original-supervisor-control-red-1` stopped
that selected current protocol child in actual kernel State T. Neither status
nor stop responded within the unchanged250ms observation budget. Explicit
SIGKILL/wait reaped the child; all seven direct children were reaped. This red
tests required responsiveness, not compilation of a missing future parent.

Frozen `original-supervisor-build-1` passes six separately selected actual ARM
epochs, with22 observation groups,43 direct-child reap records and six SDK-worker
reap observations. PID values can repeat across separate VMs; these are per-epoch
ownership records, not a count of globally unique processes:

| Epoch | Actual boundary |
| --- | --- |
| Held control | Source is stopped before launch, then the initializing SDK worker is stopped. Status3.92ms and stop acknowledgement4.52ms deny admission while occupancy remains until actual reap. Kernel limits and NoNewPrivs are inspected. |
| Initialization deadline | The held worker crosses the unchanged original five-second BOOTTIME deadline; no Ready or restart occurs, and it is actually reaped. |
| Queued late Ready | Parent is stopped after bootstrap transmission. The original compositor commits the real context and a UID11010 probe binds allowed globals. Parent resumes after its original deadline and denies the queued completion without emitting Ready, then reaps. |
| Source death | Authentic Ready precedes actual original-compositor SIGKILL. The retained source watch denies admission and the owned SDK process is reaped. |
| Parent death | Parent is SIGKILLed while the SDK worker is stopped. Kernel parent-death policy kills it; the guest subreaper performs actual waitpid with SIGKILL status. |
| Normal | Authentic Ready permits future listener admission. Status4.06ms and Stop4.83ms remain responsive; Stop denies first and occupancy clears only after reap. |

The first queued-ready attempt stopped the parent before it transmitted bootstrap
handles. The filtered probe timed out; all children were reaped. The corrected
test first observes initializing status while the source remains stopped, which
establishes the parent's initial poll. This test-driver correction changes no
native code or deadline; the failed epoch remains an unselected diagnostic.

The last four fresh epochs include shared input locks and exclusive label locks;
two run concurrently against the same read-only snapshot backing. Earlier held
and deadline fixtures remain separately frozen. No earlier source variant is
claimed byte-equal to the final runner. Current Rust descriptor/exec components
also pass3/3 on the actual host kernel; a default-user permission failure and
ENOSPC setup attempt remain unselected diagnostics.

Control timings are observed guest execution budgets, not guarantees against
arbitrary scheduler or filesystem stalls. Local reaping, source loss and accepted
client filtering retain their separate meanings.

## Remaining integration

Next compose the restricted listener and supervised owner with the existing
current-authority actor and its exact launched app generation. Root must own
source/executable/listener provisioning and app UID isolation, deny direct
privileged runtime access, and retain app/context/resource lifetimes together.
Existing accepted clients and previously released data require their own
lifecycle and presentation fences.

Trusted Quickshell/PAM lock admission and restoration, every privileged global,
clipboard/data restrictions, delegated children/popups, secure frames,
suspend/resume/wake, complete installed service configuration and genuine theme
operation remain open. The Arch ARM7.1.6 system-QEMU result does not qualify the
retained Pixel6.6 kernel/config/KMI, Google modules/firmware, display drivers or
physical-device behavior. No matched kernel replacement follows.
