# Proposed durable incoming receipt and acknowledgement owner

This is a design for the next native messaging milestone. It grants no modem
acknowledgement permission and adds no receive deduplication implementation.
The recovered [transport contract](native-sms-transport.md) remains the evidence
for its assumptions and unresolved gates.

The frozen standard `newSms` and `newSmsStatusReport` indications contain a type
and complete PDU, with no durable receive identity. Their public ordering contract
holds subsequent SMS/status-report indications until acknowledgement of the last
incoming GSM item. The serial passed to `acknowledgeLastIncomingGsmSms` identifies
the acknowledgement command, not the arrival. Factory ordering, reset and replay
semantics have not been executed. Shannon's TPID/token fields supply no independently
qualified durable identity or reconnect idempotence either.

## Local occurrence storage

Each admitted indication receives a fresh local receipt UUID. Equal bytes, a body
hash, a concatenation reference, recipient/timestamps or a TP-MR do not reuse a
receipt UUID. A source-generation/endpoint/occurrence ordinal records where it was
observed; it supplies no cross-session or network identity. Duplicate-looking
arrivals remain distinct records. Text assembly and display policy may link
records only as explicitly untrusted content associations; they must not erase
transport occurrences or reuse their acknowledgement authority.

A bounded serialized owner should persist the complete bounded raw PDU, indication
kind, local receipt UUID and source provenance in its exclusive private SQLite
store before granting any positive receipt acknowledgement. Decode outcomes belong
to separate fields: valid deliver text, valid status-report metadata, unsupported
or malformed. A status report must not enter the incoming-text path or finalize an
outbox part solely because it parsed. Delivery correlation needs its own qualified
per-slot/per-attempt mapping; current TP-MR zero and later reference reuse collide.

Only the backend-qualified live incoming window can become acknowledgement eligible.
Stored receipts reopened after restart cannot recover that authority. One current
window must cover both SMS and status-report indications for a GSM endpoint; a
second unacknowledged indication violates the assumed ordering and revokes local
eligibility rather than selecting whichever receipt was stored last. Callback
queue overflow, replaced callbacks, endpoint death and source revocation also
revoke it. Existing receipts stay durable. An adapter must independently qualify
factory registration, ordering and quiescence before it can open such a window.

## One-shot acknowledgement ownership

An acknowledgement permit would bind a local receipt UUID, opaque endpoint/source
generation, opaque live incoming-window identity and a fresh command attempt ID.
Its backend-specific target stays outside the shared owner. Standard last-incoming
acknowledgement has no message target token; a provider token, where available,
remains scoped to its qualified live session until evidence establishes more.

Claiming a permit must durably record acknowledgement uncertainty before exposing
an IPC command. The permit is non-cloneable and cannot be refunded after it is
claimed, dropped, timed out or invalidated. The adapter owns command serial/method
validation and maps an exact final acknowledgement result to the attempt. One-way
return and `acknowledgeRequest` are progress only. A final response that qualifies
an acknowledgement transport command still does not establish unique network
receipt identity, carrier delivery or safety of replaying that command.

Positive versus negative receipt policy and the GSM failure-cause mapping require
separate qualification. Durable storage alone must not silently choose a success
flag or an error cause. `responseAcknowledgement()` for ACK_EXP callbacks is another
protocol obligation; it must not consume or replace this last-incoming GSM permit.
The proposed API should therefore expose no generic “acknowledge everything” path.

Cancellation of local ownership, source replacement, callback consumption and
service death must be serialized with acknowledgement claims/finalization. A held
or late callback must pass the current source/attempt fence when applied. It may
finish only its original attempt; it cannot choose a newer last-incoming receipt,
reactivate an ineligible receipt or authorize a retry. Store failures emit no
successful receipt result.

| Crash or uncertainty point | Required durable/local behavior |
|---|---|
| Before arrival storage commits | No positive acknowledgement authority is released. Remote retry behavior remains unqualified. |
| Receipt stored, permit not claimed | Receipt remains; ephemeral incoming-window eligibility is lost on restart. |
| Permit claimed before IPC, or command outcome unknown | Uncertainty remains; no automatic replay after restart/reconnect. |
| Equal-looking arrival after reconnect | Store a new local occurrence; do not deduplicate or transfer an earlier permit. |
| Qualified final acknowledgement for an old attempt | Record only that attempt's result; preserve newer receipt ownership and all uncertain history. |

## Smallest next executable qualification

Use the existing frozen messaging V3 generated NDK/Rust Binder peer to emit synthetic
`newSms` and `newSmsStatusReport` into a proposed owner backed by real SQLite. First
observe a runtime red proving that no acknowledgement command can escape before
durable admission. Exercise bounded queues/storage, both indication kinds sharing
one window, a second premature arrival, callback-generation replacement, held
callbacks after cancellation/death, malformed PDUs, store failures, one-way and
intermediate progress, exact final command qualification and restart with uncertainty.
Assert separately that equal PDU bytes produce two receipt UUIDs and that neither
creates a retry permit. No production modem acknowledgement dispatch is part of
that synthetic test milestone.

Factory incoming-window ordering/registration, backend success/failure mapping,
producer lifetime across resets, redelivery identity and acknowledgement idempotence
remain gates. A future native implementation can store local occurrences without
resolving those gates, but it cannot connect a production acknowledgement adapter
or promise exactly-once receive semantics from this design.
