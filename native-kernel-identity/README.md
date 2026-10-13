# Exact socket peer pidfd

This small Linux ABI boundary exists because pinned rustix1.1.5 has no
`SO_PEERPIDFD` getter. The only public operation accepts a borrowed socket and
returns an owned CLOEXEC pidfd for the peer reference retained by that socket.
It never opens a numeric PID, accepts a generic socket-option number or falls
back on an older platform. Callers may keep unsafe code forbidden.

The audited unsafe boundary is one fixed `libc::getsockopt` and conversion of
its newly transferred descriptor to `OwnedFd`. Output shape, status, descriptor
and atomic CLOEXEC are checked. Rejected shapes drop any transferred descriptor.
The Linux implementation retains `sk_peer_pid`, calls `pidfd_prepare`, copies
both fixed outputs and installs the descriptor only after successful copies:
[pinned public Linux v6.8 source](https://github.com/torvalds/linux/blob/e8f897f4afef0031fe618a8e94127a0934896aba/net/core/sock.c).
Public API source hashes are recorded in `public-api-inputs.json`; source bytes
stay in the ignored public-input cache. Dependency versions and checksums remain
pinned in Cargo.lock.

Tests distinguish actual Unix kernel credentials/pidfd readiness/peer exit and
actual ENOTSOCK from injected malformed result shape with real descriptor
ownership cleanup. The injected shape case does not claim a malformed kernel
return was observed. A pidfd exit is local process liveness evidence; it does not
qualify a healthy policy seal, remote source cleanup or modem quiescence.
