# Native Hyprland session facts build boundary

An authored observation plugin compiled and loaded into the installed Arch Linux
ARM Hyprland `0.56.2-4` compositor under an actual ARM64 Linux `7.1.6-1` kernel.
The selected fourth run passed eleven API and original-protocol observation
groups. The compositor ELF and all 497 installed Hyprland headers match the
cached distribution package payload. Four hundred non-generated source headers
also match the official upstream release archive.

This qualifies a small **FactsObserver plugin build path**, using the exact
installed package's headers and compiler. The fixture observes original
compositor behavior; some passing assertions deliberately expose unsafe inputs
for a naive observer. It does not qualify a production adapter, authenticated
source transport, Rust actor bridge, trusted locker, app permission change or
Messages publication gate. Those components require separate evidence under the
[session ingress contract](native-session-contract.md). See the
[authored provenance and artifact manifests](evidence/native-session-build-boundary-provenance.json)
and [reproduction instructions](../tools/native-session-build-probe/README.md).

The evidence class is original installed compositor and Linux kernel execution
with authored plugin, native window, Quickshell layer and Wayland lock fixtures
inside system-QEMU. It is not ARM64 user-mode emulation or physical-device
execution. The original Omarchy lock service and its PAM/delegation contract were
not executed by this probe.

## Exact build selection

| Input | Selected identity |
| --- | --- |
| Compositor package | Arch Linux ARM `hyprland 0.56.2-4`, `aarch64`, build timestamp `1791044002` |
| Cached package archive SHA256 | `4183f5212200625c278eefbbf70c912f1e8803df7ed99a8a2988beb4f4c74e04` |
| Installed compositor ELF SHA256 | `ae9a9e4129263e74df3f8a0a62beefd00f9bf4224caf51e96ebf0482ab74a85a` |
| Compositor GNU build ID | `d42f21e7b9bd77d03e8cb1778c9a2dd5986ae86b` |
| Runtime/header upstream commit | `efb50993780079460b0cbed1363e2166a2de1d9f`, tag `v0.56.2` |
| Official upstream release archive SHA256 | `03ad3f5ef152ff44116ffd56fcf808486211ecabf4f0ba567108ee746ba5cd2e` |
| Installed package `.BUILDINFO` PKGBUILD SHA256 | `73ec2161a62dd11d84f15fa36daa1cff0540f57fdeacaf7605143bd3ff6284f7` |
| Guest compiler package | `gcc 16.1.1+r12+g301eb08fa2c5-1`; compiler reports `16.1.1 20260430` |
| Guest binutils package | `2.46+r70+g155188ea10a7-1` |
| Selected probe ELF SHA256 | `ac673f13ee21d1ea42b7d509e04703bac7bc40661c08d064172c4177d9a5e60b` |
| Probe GNU build ID | `62cd6cbdbb1f144b2836e22e3833211a570eb463` |
| Installed-header manifest SHA256 | `4a9f8ec187515b105d646e7bacd8af57eab3b1bf13dab25c9b5fe91aeda1f78e` |
| Selected result SHA256 | `b5f8ab74d2e3460da769a04ba3e441d131db5d130fc4af7502710fc4ffbfa96b` |

The [official ARM package page](https://archlinuxarm.org/packages/aarch64/hyprland)
identifies this package and build. Local package-payload comparison establishes
that the executed ELF and all installed plugin headers came from the same cached
package bytes; the probe did not reverify the package signature. The package's
recorded compiler version matches the guest compiler. ELF `.comment` also records
GCC `16.1.1 20260430`, including a crosstool-NG compiler variant.

The [Arch packaging tag](https://gitlab.archlinux.org/archlinux/packaging/packages/hyprland/-/blob/abb207b0391249daf8a01171f08567dc0209a826/PKGBUILD)
selects the same official upstream archive, and contains build-file substitutions
for RPATH and the glaze version constraint. Its retrieved PKGBUILD SHA256 is
`2403e124b50afb22174e55125efcd57337bf9d25f96394f9d9c7865e4224bd29`,
which differs from the installed ARM package's `.BUILDINFO` recipe hash. The ARM
Source Files and View Changes endpoints returned HTTP500 during inspection.
The exact ARM recipe and complete distribution C++ patch set have therefore not
been recovered. The 400 source-header matches do not establish that every C++
implementation file is unpatched. The working build selection is the installed
package's own headers and ELF, rather than an assumed upstream rebuild.

The native guest compile command was:

```sh
g++ -std=c++23 -fPIC -shared -O2 -g \
  $(pkg-config --cflags hyprland) \
  /tmp/native-session-probe-build/probe.cpp \
  -o /tmp/native-session-probe-build/probe.so
```

The installed generated protocol headers are included by `hyprland.pc`. The
plugin checks `__hyprland_api_get_hash()` against the header-derived client hash;
its load and unload both returned `ok`. The runtime ABI hash is
`efb50993780079460b0cbed1363e2166a2de1d9f_aq_0.15_hu_0.14_hg_0.5_hc_0.1_hlg_0.6`.
That API hash omits some artifact detail and is not a substitute for the ELF,
package, compiler and source pins. Hyprland's
[plugin API](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/plugins/PluginAPI.hpp)
passes C++ objects and does not promise ABI compatibility across builds. This
execution qualifies the recorded package and exercised API paths only.

## Public native APIs exercised

The probe retains `CHyprSignalListener` handles for the central
`g_pSeatManager->m_events.keyboardFocusChange` signal. Its snapshot reads
`g_pSeatManager->m_state.keyboardFocus.lock()`, obtains the focused resource's
actual `client()`, then calls `wl_client_get_credentials` and `wl_client_get_fd`.
The resource's `m_hlSurface` and `view()` identify its desktop role. Only a
`VIEW_TYPE_WINDOW` converted by `CWindow::fromView`, with `m_isMapped`,
`m_xdgSurface` and no `m_isX11`, can be a candidate. The live peer pidfs identity
check below is also required. Layers, null focus and other view roles produce no
eligible window. Native popup ownership remains unqualified.

Lock observation uses public `PROTO::sessionLock->isLocked()`,
`g_pSessionLockManager->clientLocked()`, protocol `newLock`, manager `lock` and
`unlock`, and each observed lock's `unlockAndDestroy` and `destroyed` signals.
A compositor event-loop timer samples the completion flag separately. There is
no public native completion signal in `LockNotify.hpp`; Wayland lock notification
and initial current-state reconciliation are separate interfaces.

The dynamic-symbol inventory records the central setter, native resource client
accessor, window/view accessors, lock accessors, manager operations, globals and
plugin registration entry points. Their native load and exercised calls provide
concrete ABI evidence; symbol presence alone does not qualify an unused method.
The protocol macro `PROTO` names `NProtocols` in exported symbols. No private
member casting or `private`-visibility override was used.

## Peer process identity

Numeric Wayland connection credentials alone cannot identify the current actor's
app generation if an old connection survives delegation and the numeric PID is
later reused. The probe gets a pidfd from the **actual focused Wayland socket**:

```cpp
getsockopt(wl_client_get_fd(client), SOL_SOCKET, SO_PEERPIDFD, &peerFd, &length);
fstat(peerFd, &identity);
fstatfs(peerFd, &filesystem);
poll(&peerPoll, 1, 0);
```

It requires a 64-bit `ino_t`, pidfs filesystem magic `0x50494446`, a nonzero inode
and no poll readiness/error. It closes the returned descriptor after obtaining
the snapshot. Unsupported or failed socket identity, a wrong filesystem or dead
peer makes the candidate ineligible. There is no `pidfd_open(observed_pid)`
fallback in the plugin.

In run4, the native foot connection reported PID413, UID/GID1000, pidfs device6,
inode8291 and a live peer. The independent root fixture opened the current live
PID413 with `os.pidfd_open`; its device6/inode8291 matched exactly, and its poll
also reported no exit readiness. That independent PID lookup is a test comparison,
not the plugin's identity source. The layer connection had its own live identity,
inode8298, while its view type still made it ineligible.

[Linux7.1.6 socket code](https://github.com/gregkh/linux/blob/v7.1.6/net/core/sock.c#L1817)
gets a retained `sk_peer_pid` reference and prepares that pidfd, using
`PIDFD_STALE` for AF_UNIX. It does not look up the observed numeric PID.
[AF_UNIX connection setup](https://github.com/gregkh/linux/blob/v7.1.6/net/unix/af_unix.c#L742)
registers and retains the connecting task's process reference.
[Pidfs](https://github.com/gregkh/linux/blob/v7.1.6/fs/pidfs.c#L73)
documents 64-bit inode comparison, consistent with the
[systemd inode-ID contract](https://man7.org/linux/man-pages/man3/sd_pidfd_get_inode_id.3.html)
for an identifier within the current boot.

A future actor must compare this opaque device/inode identity with its already
owned current-generation pidfd, check that descriptor's liveness, and retain
source incarnation and boot binding. Numeric PID, title, class, user name or
Wayland FD possession cannot replace that comparison. Numeric PID reuse was not
forced in this probe. Dead delegated Wayland connections, descriptor inheritance,
socketpair-created clients, identity mismatch, unsupported kernels and inode
exhaustion have not been executed here.

## Eleven executed groups

| Group | Actual observation |
| --- | --- |
| Matching installed headers compile native AArch64 plugin | Guest GCC compiled the selected C++ source against the installed headers; probe ELF and undefined-symbol manifest were preserved |
| Exact installed Hyprland plugin load and initial protocol snapshot | Original distribution compositor loaded the plugin; initial lock and completion flags were false, with no focused principal |
| Central seat native window credentials match actual kernel principal | Native foot focus had PID413 and UID/GID1000; `/proc` credentials and public window query matched |
| Actual Wayland SO_PEERPIDFD identity matches independent current-task pidfd | Socket-derived and independently opened pidfds had the same pidfs device/inode and live poll state |
| Central seat null-focus signal observed | Fixture called the public seat setter with null; the signal fired and candidate credentials were absent |
| Actual layer focus differs from remembered active-window PID | An authored Quickshell exclusive layer owned keyboard focus as PID444/view type3 while `activewindow` still returned foot PID413; layer eligibility was false |
| Real ext-session-lock early admission and dead locker remain locked | Native Wayland client requested a lock without surfaces; immediate lock was true before completion, and stayed true after the client died |
| Restored lock and five-second completion fallback are separate | A new native lock client restored the stranded lock; no lock surfaces were supplied; the five-second fallback changed the completion flag to true |
| Real protocol unlock signal observed after protocol-state clear | The fixture's restored lock owner sent actual `unlock_and_destroy`; protocol/manager signals and unlocked snapshot were observed |
| Manager force-unlock signal observed | Fixture-only plugin command called the public manager operation; its unlock signal and false protocol state were observed |
| First lock signal precedes protocol locked-flag assignment | First manager-lock and protocol-new-lock callback snapshots both contained false protocol state; a later snapshot contained true state |

The fixture enables `allow_session_lock_restore` and intentionally allows its
synthetic locker to request restoration and unlock. These observations establish
original protocol behavior, not trusted locker authorization or containment.
The public-control fixture commands for clearing focus and force-unlocking are
private test operations and must not be exposed by a production observer.

## Original ordering observation and presentation limits

In the final event log, first-acquisition manager-lock event28 reported
`locked=false` while the native foot candidate was still eligible. Protocol
new-lock event30 also reported `locked=false`, after focus had cleared. The
[pinned protocol implementation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/SessionLock.cpp#L185)
emits `newLock` before assigning `m_locked=true`; the manager emits its own lock
signal inside that callback. This ordering was executed in the installed binary.
A naive observer reading only the flag in those callbacks would temporarily
export the old unlocked value. This is an original observation that the passing
fixture expects, not an implemented correction or a claim about a deployed
permission vulnerability.

A production observer must preserve admission denial during these callbacks and
reconcile after `onLock` returns; intervening null-focus snapshots must not clear
that denial. Rejected/inert lock requests, restoration, force-lock and observer
restart need their own bounded conservative handling. Protocol unlock sets the
flag false before its signal; the manager's unlock callback can still see the old
completion flag before the manager clears its lock object. Completion is therefore
an independent observation and cannot authorize reads.

The no-surface locker test proves early and stranded denial state. The later
five-second completion flag proves fallback behavior, not that every output
presented a secure locker frame. Secure presentation, no-output paths, trusted
unlock, transport backpressure/source failure, lock-intent coordination and
publication ordering remain unqualified. There is no claim of zero publication
after an unobserved external transition.

The public `CSessionLock` object has no client accessor; its actual protocol
resource is private. Early acquisition cannot obtain trusted locker credentials
from a lock surface that does not yet exist. Trusted admission/restore restriction
needs a matching source patch at the protocol boundary. The FactsObserver build
path uses public members only and does not supply that restriction.

## Isolation, cleanup and historical runs

The runner created its own sparse/reflink clone of `sim/rootfs.img`, its own lock,
result directories and container `pokelinux-session-build-sol-20261012`.
The repository and clone were mounted read-only; guest disk changes used QEMU's
snapshot overlay. Docker used `--network none`, dropped capabilities and did not
expose host GPU/input devices. QEMU used `-nic none`; the guest enumerated only
`lo`. Xvfb/SDL supplied an isolated virtual display. The shared Messages VM,
host desktop and physical phone were not accessed.

The guest attempts bounded termination and waits for each owned fixture process,
then powers off. Individual child-reap outcomes are not separately recorded.
The selected runner exited0, and a subsequent Docker inspection confirmed that
its owned container was absent. Its cleanup trap removes only that named
container, terminating the contained QEMU/Xvfb processes. This does not qualify
the production actor's cleanup contract.

All runs are preserved in ignored `artifacts/native-session-build-probe/`:

| Run | Result and interpretation |
| --- | --- |
| run1 | Four groups passed; fixture then used obsolete `focuswindow` dispatch syntax and failed. This was test setup failure |
| run2 | Four groups passed; authored layer QML had an invalid semicolon, so the layer-focus wait expired. This was test setup failure |
| run3 | Corrected Lua/QML fixture passed the original ten groups |
| run4 | Added socket-derived pidfs identity and independent comparison; all eleven groups passed; selected evidence |

The five current executable/source recipes match run4's preserved source files
and its recorded SHA256 selection exactly. The final event log has55 records;
`result.json` records53 before the final unload/query/exit records. The published
provenance lists source, ELF, original-reference, result and log hashes. Official
source archives, package bytes, original-reference inputs, binaries and raw logs
remain in ignored private cache/artifact storage. At the probe handoff, no service
or actor had changed and no commit had been made. Subsequent publication links
this observation from the README, integration map and handoff; those links do not
qualify a production session bridge.
