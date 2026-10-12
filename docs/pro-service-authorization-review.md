# Service authorization review

BRIDGE VERIFIED: Chat selected, Pro selected, marker matched, response completed;
zero context rounds. Owned tab closed after extraction.

- Conversation: (private review transcript)
- Marker: PRO-BRIDGE-20261012-auth-73d9
- No secrets or source files transmitted; architecture summary only.

Pro supports the shared process session for the explicit same-UID desktop trust
model. The UID is the principal; the session fences service instances, not apps.
The following findings remain implementation and test requirements:

- Check busy/pending and unknown outcomes under the same admission authority;
  do not queue a second action for automatic execution after the first finishes.
- Preserve each action's captured session and Android preconditions. Reconnecting
  must not attach a fresh session to an old action and retry it.
- An absent receipt is not proof that a delayed complete request cannot arrive.
  Window close does not promise cancellation. Recovery must query the original ID.
- Hold exclusive service ownership before recovery and socket cleanup, through
  the lifetime of all workers. Existing journal locking is only part of this.
- Acknowledgement must name the exact epoch and operation, preserve its unknown
  receipt, and never release a gate while a worker can still send.
- Peer credentials are a connection-creation snapshot. Same-UID processes can
  delegate descriptors; this is not app sandboxing or protection against root.
- Bind immutable TLS enrollment to admitted work. Do not retarget commands through
  a live credential/configuration replacement.
- Keep the journal in persistent storage, separate from the runtime socket.

One concrete defect was confirmed independently against SQLite documentation:
DELETE journaling with FULL can lose the last transaction on power loss. EXTRA
also syncs the containing directory after unlinking the rollback journal.
Source: https://www.sqlite.org/pragma.html#pragma_synchronous
A test on the actual ledger connection failed (2 instead of 3) before the fix.
Physical power-loss behavior remains unverified; this test checks the required
SQLite configuration, not the storage device's guarantees.

Authorization tests now cover shared sessions, stale restart sessions, and a
real cross-UID connection in an isolated container. The broader service lifetime,
concurrent admission, delayed-helper, and installed UI recovery matrix remains
unfinished. Pro's suggestions are review input, not completion evidence.
