# Native recovery review

BRIDGE VERIFIED: Chat selected; model menu showed Pro, 5 of 5; one user turn with marker PRO-BRIDGE-20261012-0353-c81d; terminal FINAL_RESPONSE with Copy/Rate/Share actions and no Stop. Zero context rounds. Owned tab closed after extraction.

Conversation: (private review transcript)

The review found no confirmed implementation defect from the supplied protocol description. It identified decision-changing test gaps: overlapping UUID preparation; stale async completions; delayed remote authorization after local timeout; session rotation versus durable reservation; worker retirement; reserve/outcome/ack commit failures; recovery beyond 64 records; abrupt shell death during admitted work. These are review suggestions, not claims that the reviewer inspected or executed the project.

Independent source checks:

- PhoneService includes identity.active in busy before starting another UUID job. Completion checks captured session/version/context. JsonCommand owns callbacks per job and signals SIGKILL on timeout; marking a job inactive is not proof of OS process exit. Session fencing remains the dispatch barrier.
- Dispatcher holds its admission mutex across authority checks, durable reservation and pending publication. begin_session takes the same mutex. Acknowledgment rejects while pending remains true. Ledger has an exclusive owner flock and uses journal_mode=DELETE plus synchronous=FULL.
- GatewayService creates an Admission gate, posts a FutureTask to Android main, waits boundedly, then cancels the gate and removes callbacks. ControlPolicy checks state/revision and commitAllowed before dispatch, reserves the ID and consumes revision before invoking Telecom. These mechanisms address remote late-admission risk; combined recovery/fault-injection proof is still a separate test requirement.
- Existing 122-test Linux suite includes multiple relevant socket, session, persistence, pagination and ordering cases. Do not infer complete fault coverage from the count.

The full installed shell recovery run passed with lost Answer reply followed by shell restart and explicit acknowledgment. The current packages also passed enrolled Android Telecom ACTIVE/idle verification. A stronger SIGKILL-during-admitted-worker test was added after this review; its result is recorded separately.
