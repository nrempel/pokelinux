# Native lifecycle authority

The actor owns app children, durable policy work, Messages reads and session
facts. Display-enabled callers supply `display::Factory` through
`service::run_with_services`; headless callers use the same app slot algorithm.
A trusted Factory creates one owned Context for a reserved app instance. Setup
uses an original one-second BOOTTIME deadline within the two-second client RPC.
No app starts before Ready and an exact-owning-connection EOF check.

Retirement revokes data authority, withdraws the listener before SDK signals and
retains app/context ownership until actual waits. Snapshot and Stop stay bounded
while initialization is held. Source observation and the history writer remain
independent of app context retirement. Metadata is not permission, pathname
withdrawal does not disconnect accepted clients, and local reap is not vendor
cleanup.

[Per-app ARM evidence](../docs/native-display-app-generations.md) qualifies
lifetime/relaunch and held-init Stop. Pending EOF passes its narrow boundary;
its successor aggregate is failed after Source InvalidData of unknown predicate.
The full installed smartphone, trusted lock/restore, incoming restricted-display
composition, suspend/wake and Pixel hardware gates remain open.
