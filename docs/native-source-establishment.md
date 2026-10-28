# Bounded Source establishment

The Rust Source observer now requires two consecutive fresh samples with
advancing timestamps before exporting its first snapshot. One timer dispatch
proves that the compositor started dispatching; it does not establish a
continuous observation chain. This addresses premature establishment without
claiming to identify the callback responsible for earlier cold-start failures.

During establishment, the observer returns no snapshot. A timing gap or expiry
retires only the provisional timing candidate. Credentials, source identity,
sequence, timestamp ordering, peer handles and lock/unlock protocol facts still
undergo validation. Raw protocol history survives discarded timing candidates;
malformed or inconsistent input remains terminal.

Commitment occurs at first snapshot export, after draining and checking
freshness. An unlock occurrence from before or at that export is never exposed
as authentication evidence, including on subsequent heartbeats. Its raw facts
remain available for strict protocol validation. A later genuine lock/unlock
cycle can expose its new occurrence.

The original Source startup deadline is never renewed. Until first export,
SourceOwner remains uninitialized and the lifecycle actor receives no Source
authority. After export, every observed expiry or sampling gap retains its
existing terminal behavior. The 100 ms sample budget and original sample cutoff
are unchanged. No compositor restart, authority reset or registration retry is
introduced.

## Executed boundary

The initial host regression uses an actual authored child, credentialed Unix
seqpacket traffic, transferred pidfds and actual child wait. The original
implementation fails because its first sample escapes as sequence 1. With only
the production observer changed, the exact frozen consumer passes: first sample
private, actual provisional expiry nonterminal, a later fresh pair establishing
the source, then permanent refusal after established expiry.

The frozen test SHA-256 is
`62ff5595ced2303cec196997d1849ac779feab2a52cfc81d4533b44dff0ef8ef`.
The original observer is
`20d804cac76dc35657a320eeb68c3a09145bcef18b191ad9c62d8932f86e012f`;
the passing observer is
`af9b65e48c06c0c369c0f191e73e071fccf4a6b7f486af8e7cea062a0a665399`.
Both runs close their actual peer and container custody. A preceding missing
build dependency is setup failure, not behavioral RED.

The adapted host validation records 96 case passes: 69 observer/protocol cases,
7 SourceOwner cases, 3 cleanup cases, 12 Source-status cases and 5 startup cases.
Strict formatting/Clippy and an optimized normal x86 owner build pass. The
original five-second startup deadline is exercised without any publication.

These passes span preserved epochs. An earlier child-death test omitted the
existing typed zero-byte receive refusal; its assertion now checks that precise
alternative rather than allowing arbitrary errors. A later long-window test
exceeded its diagnostic trace capacity; only that case receives a larger fixed
record budget. Both failed aggregates remain failed, including the aborted
case’s unqualified child cleanup. [Curated evidence](evidence/native-source-establishment-provenance.json)
records these distinctions.

The adapted C++/Wayland timer cases compile as Rust tests but have not been
re-executed with their native producer. Normal cold owner→shell→app execution
remains open.
This host test does not qualify upstream compositor startup, ARM graphics,
authentication, a screenshot or phone hardware. See the
[separate failed cold diagnostic](native-session-x86-runtime.md#bounded-backend-dispatch-diagnostic).

## Cold shell integration after establishment

The fresh x86 installed-profile run reaches genuine Omarchy shell IPC `ping` and
its background/bar compositor surfaces with the normal optimized Rust owner.
Source is observed and ready at sequence 14, then remains healthy at sequence 76
before cleanup. The original compositor and matching adapter remain unchanged.

The full run still fails: the normal appearance snapshot is unavailable before
app launch. The selected deployment omits genuine theme assets and initial theme
selection; this is deployment-source evidence, not a captured collector errno.
Separate shell journal errors concern Qt Color name resolution and missing
compositor instance environment. These require normal deployment integration.

No app boundary steps or new PNG were produced. Final Pixel display qualification
is null and the positive manager Stop-after-Ready case is false. Guest cleanup,
QEMU exit and container closure qualify independently of the failed scenario.
The execution, boot and result SHA-256 values are respectively:

- `e35303b1fef3f292f9a2436bc4725ba99026f93342300474c0e882d66a1bfc9a`
- `c27ce1ee6a06ab4d370f049c816bf4e2ff3b8985233d0c7852bdc35ae8674125`
- `6bbaaf999f60e19019a761cbdf821c47c9a4dad7604131a7804d2bf36a2c80ef`

This is partial cold shell progress on x86, not completed app/session integration,
ARM graphical qualification or a bootable Pixel image.

The same shell journal also reports unavailable PipeWire, network and UPower
integration, missing helper commands, and no login session for its polkit
listener. Background/bar surfaces and IPC ping therefore do not establish working
shell status controls, authentication or notifications. Native service bindings
and the session/locker integration remain product gates; theme provisioning alone
cannot close them. The selected audio service uses its own private runtime, so
the shell's default PipeWire connection failure does not invalidate the separate
qualified native audio paths.
