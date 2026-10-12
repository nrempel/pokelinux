# Recovery review checkpoint

BRIDGE VERIFIED: Chat selected, Pro 5 of 5 verified, terminal response actions present with Stop absent, marker PRO-BRIDGE-20261012-0318-b29c matched. Zero context rounds. Owned tab 211965966 closed after extraction. [Conversation]((private review transcript)).

The independent review recommends durable per-record acknowledgment, separate from immutable command outcome, plus daemon-enforced session admission fencing. Pending work cannot be acknowledged. Terminal uncertainty may be explicitly acknowledged without becoming a known result, deleting replay protection, or triggering another command.

The daemon must check all blockers, not just the first 64 returned rows. Filter acknowledged rows from the attention list but preserve historical lookup. Once blockers clear, require a new observation request started after clearance; unchanged Android revision is acceptable. A later separate user gesture is required for another call command. No saved intent may auto-resubmit.

Every new command must have current session authority at final admission. An old already-admitted operation can complete once. An obsolete helper must never obtain fresh authority for its old intent. A pure duplicate-result path may return existing results without dispatch. Terminalizing a pending local worker as unknown must not leave a continuation that can later initiate first dispatch. Already-transmitted Android requests remain uncertain even after acknowledgment.

Suggested integration tests cover both sides of the admission/takeover race, pending visibility, lost acknowledgment response and persistence, replay/conflicting payloads, stale observations, more than 64 rows, later explicit controls, daemon epoch changes and missing session fields.

Implementation status when user redirected work to hardware-boundary research: internal Dispatcher session tests pass; local socket session API test is red because phone_local.begin_session is absent. Harbour session-red.log confirms AttributeError, five tests run, one error. Server/CLI/QML session integration and acknowledgment remain unfinished. Released packages have not been rebuilt to include the internal session changes. This review is advice, not proof of correct implementation or hardware behavior.
