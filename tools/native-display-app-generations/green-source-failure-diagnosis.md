The cancellation successor epoch remains failed. Its retained bytes establish Source InvalidData followed by actor withdrawal of the successor context. They do not identify the first rejected source packet or establish a particular timing/shape cause. No core, lease or frozen driver/result changed during this read-only diagnosis.

| Retained BOOTTIME observation | Meaning |
| --- | --- |
| 40.154362856s, sequence1/revision1 | Actual accepted focus373; sample until40.254362856s. |
| 40.215833590s, sequence3/revision2 | Actual accepted focus362; sample until40.315833590s. |
| 40.347375347s, sequence9/revision3 | Actual accepted focus373; sample until40.447375347s. |
| 40.477823971s, sequence15/revision4 | Actual accepted focus362; sample until40.577823971s. This is the last **logged semantic change**, not the last accepted heartbeat. Intermediate unchanged heartbeat samples were not printed. |
| 40.963405181s / 40.963671939s | Owning client EOF / actual SDK391 SIGCONT. |
| 40.969775223s | Canceled SDK391 terminal occupied=false; old path is later observed absent. |
| 41.134936992s | Authenticated actor Snapshot still ready/unlocked with principals0; original native-focus probe reports live owned focus362; same writer388 lives. The probe sequence269 is its own query counter, not the Source wire sequence. |
| 41.149687034s / 41.196725950s | NewSDK392 Preparing / Ready. |
| 42.883624843s | First retained Source error, InvalidData. No rejected frame fields or last accepted heartbeat are printed. |
| 42.887910842s / 42.915950687s | Successor listener withdrawal / SDK392 terminal occupied=false. |

The actual result contains zero `baseline_frames`: the guest transferred the authenticated Source socket to the Observer and closed its local receiver. The inherited `pump()` therefore had no mirror for this selected composition. Service traces print a Snapshot only when its semantic revision changes. Hyprland's retained stdout does not print adapter packets; its full runtime log path names a guest-private file which was not retained before shutdown. There is no independent app-death timestamp or successor Qt stderr log establishing whether Qt died before the error; the later empty `/proc/.../cmdline` and mapped-window query cannot reconstruct that order.

The frozen Source610aa `Observer::poll` reports InvalidData for receive/ancillary errors, more than16 drained packets, JSON shape errors, wrong source/sequence/lock/focus fields, backwards or discontinuous intermediate sample chains, an individually expired/future sample, or focus/descriptor shape/type mismatch. A cached sample expiring after an empty drain has a distinct TimedOut branch. The outer fixture printed only `ErrorKind`, so it cannot distinguish the listed InvalidData branches. A greater-than100ms interval between two **semantic revision** samples is not evidence of a publisher sampling gap.

A dead correctly typed focus pidfd alone is accepted as ancillary shape: the selected `ProcessView::is_pidfd` explicitly checks kernel descriptor type independently of liveness. Failed live focus equivalence makes a semantic revision rather than directly failing Source. This rules out the simple interpretation that ordinary Qt death automatically makes a typed pidfd malformed; it does not establish the actual rejected descriptor or rule out a separate shape/error race.

The evidence supports a bounded availability failure during the first cold normalQt successor in this cancellation composition. It does not support a precise claim of sampling gap, stale/dead focus descriptor, malformed publisher input or a production repair. No Source reload, replay, deadline change or search for a lucky aggregate pass was performed. Further diagnosis would require a fresh explicitly scoped instrumentation epoch retaining every accepted heartbeat and the actual queued input/error predicates; its different instrumentation bytes and timing would remain separate from this failed epoch.
