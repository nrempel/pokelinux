# Bounded source freshness across delayed observer turns

This report preserves the earlier queued-successor correction and its exact
execution evidence. Its terminal-timing rules below describe that frozen
checkpoint. The current [timing-recovery contract](native-session-timing-recovery.md)
instead records loss before replacement export and requires fresh advancing
samples after actual discovery; this report does not qualify that recovery.

The Rust observer now drains authenticated queued source facts before deciding
that its cached sample has expired. Every successor must belong to the same
monotonic, continuous 100 ms sample chain and remain fresh at its own examination.
The drain stays bounded to 16 packets. An expired intermediate, sampling gap,
malformed packet, dead source or overflow remains terminal. No returned or
previously observed failure can be restored by a later packet.

The original Source sample budget remains 100 ms BOOTTIME. A successor satisfies
`previous.sampled_at <= next.sampled_at <= previous.sampled_at +100ms`, with checked
arithmetic. Equality gives contiguous half-open leases. Each packet also retains
its own absolute deadline, current source credentials, exact sequence, source
identity, descriptor shape and live source watch. Semantic revisions inspect every
packet before coalescing, including lock/unlock cycles. Final source liveness and
latest-sample freshness are checked after the bounded drain.

`snapshot` now requires mutable access and permanently latches an observed
expired established sample or dead source. This covers an expired `None`
observation before a later poll. Initial unknown source still yields `None`.
Provider conversion propagates a final observed failure as an error, so a resource
owner stops its context. A small cloned frame contains bounded metadata; focused
pidfds retain their separate ownership and actor validation.

## Actual process boundaries

The unchanged `10ce2f1` implementation first executes 34 real process/credential/
descriptor cases on Harbour Linux 6.8 and an actual ARM 7.1 guest. Each suite passes
30 and fails exactly four desired boundaries: a timely queued heartbeat, a timely
queued lock cycle, an authored exact lease boundary, and a backwards intermediate
sample hidden by a later fresh frame. Both selected runs finish with exit 101 and
zero ignored/filtered cases; setup failures remain separately unselected.

Only after those reds does Root change the observer. Fresh frozen builds pass
34/34 on each kernel, with zero ignored/filtered cases. Genuine sampling gaps,
expired snapshot observations, expired intermediate packets, previously returned
timeouts, dead-source buffered packets and bounded overflow retain their negative
results. The only additional test adjustment is mutable access in the existing
unknown-source case, required by the changed API. No assertions or timing margins
are relaxed.

Timely heartbeat/cycle positives use the sender's actual BOOTTIME sampling and
real SCM credential/focus-fd transfers. Exact-boundary, backwards and expired
fields are explicitly authored wire cases. Source deadline observations are
separate from an actor-held-read execution. Existing actor read stamps retain
their captured deadlines and unchanged validation code; these component cases do
not establish a new held-reader actor result.

The [process evidence](../tools/native-session-freshness-chain/evidence.json) binds
selected red/green sources, static executables, logs and unchanged cases. Sampled
helper absence is separate from the Rust harness's direct `Child` wait assertions;
the runner adopts no orphan waits. Both runtimes use readonly source inputs, private
mutable RAM targets and no network. No old immutable target or physical phone is
modified.

## Original compositor and incoming application

Separate diagnostic epochs instrument the original 25 ms publisher and observer
with fixed-size nonblocking timing records authenticated by actual SCM credentials.
The legacy `518954b` cold run fails while a warm control passes; a subsequent exact
published `10ce2f1` cold baseline independently reproduces cached expiry. Those
original failures and their instrumentation remain separately frozen.

In the current baseline, the cached sample is 59.702361286 s. The next observer turn
at 59.820465363 s rejects it before consuming actual full sends sampled at
59.727800405s and59.819378181s. Both queued samples remain within their original
100 ms leases, and the sample gaps 25.439119 ms and 91.577776 ms form a continuous chain.
This particular poll gap happens about 997 ms after launch acknowledgement, rather
than inside its synchronous spawn path. Shared graphics/scheduling remains a
qualified cause; tracing can perturb scheduling. Timestamp continuity proves
sampled facts in the shared BOOTTIME domain, without proving kernel send-time
continuity or eliminating unobserved-transition publication races.

The corrected replay selects immutable `10ce2f1` inputs plus the exact reviewed
observer patch, leaving the other 65 selected core files unchanged. It passes nine
full-matrix groups through real ARM Binder arrivals, original Hyprland/current
focus, independent grants and unchanged normal Qt. Cold reopening admits the new
owned app and history. A later intended source unload denies reads and reaps the
held readonly child while the same incoming writer commits callback4. Actual peer
death then fences receiving and reaps that writer. No source is revived.

The replay exercises the changed boundary: at 59.749421923 s the previous cached
sample is 100.161518 ms old, while a queued successor sampled 59.748437544 s is fresh.
Its actual full send completes before the old lease ends; sample spacing is
99.177139 ms. The fixed poll accepts that timely queued successor. All 58 cleanup
records pass and 377 publisher/2460 observer records have zero diagnostic drops.
This is a narrow observed availability margin, with no guarantee against a future
genuine publisher gap.

Root independently verifies 1,600 retained proof files and 1,575 nested bindings,
checks the 66 immutable base files, and reapplies the exact single-file patch with
zero fuzz or offset. The [publication provenance](evidence/native-session-freshness-provenance.json)
binds this review, the process indices and the separately frozen
[baseline diagnostic](../tools/native-incoming-cold-availability/current-core-evidence.json)
and [patched compositor replay](../tools/native-incoming-cold-availability/patched-core-evidence.json).
Root also views four selected normal frames independently: initial and reopened
history show the conversation, grant revocation shows access denied, and Source
loss shows Messages unavailable. Conversation visibility, database body evidence and literal normal
body visualization retain different scope. The receiver keeps its same explicit
Binder generation and one durable attempt; this change permits no reattachment,
registration retry, production ACK/send or vendor cleanup.

The display-owner 25-group checkpoint remains separately frozen before this
observer change. Per-app display context generations, production trusted source/
configuration and namespaces, PAM/secure presentation, broad scheduling and
graphics availability, incoming plus restricted display, notifications/wake and
matched Pixel kernel/modem/display/input remain open. The full smartphone goal is
active; this harness is not production IMS or hardware completion.
