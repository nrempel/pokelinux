# Native display context ownership

`pokelinux-display` uses the pinned open-source Rust Wayland client and
security-context-v1 bindings. It preserves direct application socket identity
while the original compositor filters privileged protocol access.

Use `supervisor::Supervisor` for an owned context process. The supervising caller
passes a prepared listener, established compositor connection, already-owned
source pidfd, canonical app/instance metadata and an original absolute BOOTTIME
initialization deadline. The caller protects executable/listener provisioning,
procfs and common namespaces, and retains sole child-reaping ownership.

`Context` is the protocol driver inside the limited child. Its SDK read/dispatch
can monopolize a turn; call it only inside that process boundary. The parent
never dispatches Wayland. Stop denies admission before signalling. Continue
polling terminal instances while `occupied()` is true; only actual `try_wait`
reaping clears that state. Dropping an unreaped instance aborts the owner to
prevent detachment. No retry or restart is implicit.

The two binaries require `isolated-fixture` and exercise separate test entrypoints.
They are not installed production services. Metadata is not permission, closing
the listener does not disconnect accepted clients, and local reap does not
acknowledge remote cleanup. See the [containment report](../docs/native-display-containment.md).

`listener::Listener` owns a fresh Root-protected per-instance pathname. It checks
parent/directory/socket identity, prepares ACLs before exposure and withdraws only
its own socket before the SDK is signalled. Existing accepted clients are separate.
The lifecycle display Factory returns an owned Context to each app slot; app and
SDK reaping precede slot reuse. [Per-app evidence](../docs/native-display-app-generations.md)
qualifies lifetime and held-init Stop; cancellation successor availability remains
failed after unexplained Source InvalidData. These APIs are not an installed
privileged-runtime/PAM provisioner or a physical display qualification.
