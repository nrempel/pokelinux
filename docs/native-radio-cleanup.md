# Vendor radio cleanup contract

REA examined the pinned Shannon APK and native external-radio implementation on
Harbour. This is static shipped-artifact evidence, not vendor service execution
or proof of safe re-registration. Raw responses and snapshots remain private.

The APK's generated `clear` proxy sends a one-way transaction without a reply.
Its detach path clears the saved ID, then drops its proxy only after that call
returns; a caught remote exception does not establish cleanup. Registration is
a synchronous transaction returning a numeric client ID. Death notifications
and demand reconnect in the reference APK do not supply a cleanup acknowledgement.

The focused native clear path removes a client from the manager and releases its
callback references and segmented request storage. Pending modem request records
are managed separately; the recovered clear/reset paths do not drain that list
or demonstrate cancellation at the modem. Completion later removes pending
request information and attempts response delivery. Callback delivery failure
can trigger client removal, but that is not proof of immediate cleanup on death.

Recovered allocation increments a process-local counter; no free-list reuse or
counter reset was observed in the focused paths. Allocation occurs before the
manager lock and external dispatch serialization is unresolved. Process restart
and numeric-ID reuse therefore remain separate gates. A delayed clear must never
use a reused ID without qualified producer identity. Bounded absence searches
do not prove that other libraries or generated Binder code lack death cleanup.

`Supervisor` keeps its durable uncertainty marker and refuses a successor
registration after launch, failure, termination, reaping or restart. This static
investigation supplies no reason to remove that fence. Local process reaping,
client-list removal, pending-request completion and vendor/modem reset are
different events.

[Provenance](evidence/vendor-radio-cleanup-provenance.json) records target hashes
and evidence identifiers. Required next execution covers actual endpoint clear,
pending response after clear, callback death, overlapping registration, ID
allocation/reuse across restart and a qualified reset/cleanup acknowledgement.
Do not retry uncertain registration implicitly. No phone was accessed.
