# Native action and recovery review

2026-10-12. Independent ChatGPT Pro review via the chatgpt-pro-bridge skill.
Conversation: (private review transcript)
Marker: PRO-BRIDGE-20261012-6a71.

BRIDGE VERIFIED: Chat selected, Pro shown in composer, one marked user prompt,
terminal marker-matched FINAL_RESPONSE with Copy/Rate/Share controls and no Stop,
zero context rounds. Owned tab closed after extraction. This is a contract review,
not a review of source code. The following is a summary, not a verbatim transcript.

## Confirmed admission gap

Read-only recovery can report empty while a complete old request is buffered but
not reserved. The daemon process session and Android context can remain valid.
That request can later be admitted. Deduplication and one-unresolved admission
are not violated, but reopening the app cannot treat the empty read as proof
that forgotten intent is excluded. Repeated null lookup has the same limitation.

Codex independently reproduced this with the production coordinator and real
Unix sockets in `empty_recovery_is_only_a_snapshot_not_an_admission_barrier`.
The caller disconnects with a complete buffered command, recovery returns empty,
then admission returns pending. No physical device was involved.

## Selected next correction

Add an explicit recovery/admission barrier with a separate admission token.
Commands carry the token captured with their immutable intent. The barrier and
final token check must use the same admission serialization as reservation.
Before the barrier, a request either has durable journal ownership or is excluded
from later new admission. Already admitted workers continue and receipts remain
available. Keep the daemon incarnation session stable; never silently refresh a
command's admission token or automatically resubmit it.

A global token is the smallest model for the current single command owner.
Another surface's reconciliation may refuse a still-unadmitted action; this is
an explicit stale refusal, not cancellation of an admitted action. A per-surface
fence would need a stable surface identity across app restart, which the current
product does not define. Do not add that extra model without a concrete need.
The barrier is not an exclusive reservation for the next action: concurrent
current clients still compete through the normal admission rule.

After transport uncertainty, preserve the original identity, establish a barrier,
then query that identity. An absent receipt is conclusive only in combination
with the completed barrier and retained journal history. Ordinary null lookup
keeps its existing read-only meaning.

## Other review requirements

- Derive action availability from session/recovery, fresh action observations,
  unresolved local intent and acknowledgement state together.
- Capture action context by value and claim the local action guard synchronously.
- Preserve unresolved intent across presentation invalidation and resume.
- Distinguish job IDs from command IDs. Route every completion, including errors
  without a JSON body, to the originating job. Complete each job once.
- Do not let old pending/null reads overwrite an accepted terminal receipt.
- Invalidate observations across suspend/resume and check elapsed age in the
  action path with a suspend-aware clock. A delayed timer is not freshness proof.
- Do not interpret a known command receipt as the current phone/call state.

Codex added per-invocation caller tokens to the Qt adapter and QML session, with
real-process and unrelated-completion tests. Separately, an active-adapter
teardown test reproduced SIGBUS because QProcess destruction could emit callbacks
while containing members were being destroyed. The explicit destructor now
disconnects callbacks and stops the child before member teardown. Eight behavior
cases pass (ten Qt entries including setup/cleanup).

## Remaining acceptance evidence

Test barrier-before-reservation and reservation-before-barrier orderings, final
validation under the admission lock, no-record transport uncertainty, daemon
restart, concurrent surfaces, output/timeout/reentrancy, reordered callbacks,
double press, suspend with queued observations, and lost acknowledgement reply.
Exercise controls and dial independently. Existing tests cover only part of this
matrix. The barrier is now implemented and tested through the packaged Rust daemon/CLI
and QML session. Coordinator tests cover both reservation/barrier orderings,
old-token rejection without Android dispatch, and preservation of matching
receipts for already admitted work. The action controller now implements observation capture, synchronous intent
guards, bounded suspend-aware observation age, immutable command context,
receipt lookup and uncertainty retention. QML answer/dial passed through the
real Rust service and an authenticated synthetic peer. Live uncertainty recovery now uses hello/fence/lookup, with actual packaged
daemon and journal tests for an absent entry and an explicitly acknowledged
unknown entry. Fault cases retain evidence of admission and do not replay lost
acknowledgements. Sleep notifications now cross a private D-Bus instance through the native
observer and QML lifecycle into actual packaged Rust recovery. This does not
prove host suspend or Pixel wake behavior. Read-only service health and automatic reconnection now pass against a real
restarted daemon with retained journal state. Real Android app integration and
remaining failure combinations are unfinished.
