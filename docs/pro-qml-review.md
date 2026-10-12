# QML service boundary review

BRIDGE VERIFIED: Chat selected; composer Power showed Pro, 5 of 5; one submitted user turn; marker-matched FINAL_RESPONSE; response actions and Response complete observed with Stop absent. Zero context rounds. Owned tab closed.

Marker: `PRO-BRIDGE-20261011-qml-73ab`
Conversation: (private review transcript)
Skill: `chatgpt-pro-bridge skill (local tooling)`

## Review findings and implementation consequences

- A Python user service owns observation, admission and outcomes. QML owns display and explicit user gestures. Panel absence and full shell restart must not stop observation.
- Shell child processes can end on reload. Recreate read-only observation only; never recreate a command from startup state or a process-exit callback.
- Snapshot identity includes daemon epoch, sequence and Android observation freshness. Heartbeats do not refresh Android authority. Discard superseded connection output.
- Reserve operation identity and canonical arguments before dispatch. Duplicate identity returns the stored status; changed arguments reject. Preserve the original Android generation/revision. Never queue commands for reconnect.
- Keep incoming and outgoing pending states separate. Preserve uncertainty across daemon restart. An idle observation or an absent result alone does not prove an earlier transmission cannot execute. Recovery is read-only and must never dispatch a saved request.
- Bound connections, messages, pending operations and result history. A full ledger rejects new work; it must not evict a replay guard while a request remains admissible.
- Use a private runtime directory and socket, verify peer UID, and document that these controls do not isolate a plugin from other processes running as the same user.
- Before replacing GTK, test real shell close/reload/restart, duplicate and stale commands, crash points before/after transmission, delayed Android admission, slow readers and access checks. Repeat actual touch/framework call tests.

Codex checked the pinned shell source: PluginShellApi exposes serviceFor, summon, hide, toggle and isPluginOpen through scoped callbacks. Built-in firstPartyServiceFor calls are not the general third-party interface. keepLoaded is not a full process restart guarantee.

The review proposed length-prefixed JSON. The initial codec uses bounded newline-delimited JSON with escaped internal newlines and exactly one request per connection. This is an implementation choice, not an established streaming protocol; observation streaming and service admission remain to be implemented and tested. The codec rejects duplicate keys, non-finite numbers including exponent overflow, incomplete and oversized frames, and coalesced extra frames. It applies an absolute receive deadline and restores the caller's socket timeout. Callers must set bounded write timeouts and close every connection after the exchange.

## Current evidence and limits

Eight new real Unix-socket tests pass locally. The complete Python suite passes 61 tests locally. The codec is not yet packaged or wired to a daemon/QML plugin, so these are boundary tests, not end-to-end plugin proof. Existing outgoing APK/package evidence remains a historical tested baseline; the new transport file changes the source inventory and must be covered in the next full delivery run.

The existing ARM64 guest package inventory contains Hyprland 0.56.2-4 but no Quickshell or Qt6 entries. Arch Linux ARM currently publishes quickshell 0.3.2-1 for aarch64: https://archlinuxarm.org/packages/aarch64/quickshell . Availability is not installation or shell compatibility proof.
