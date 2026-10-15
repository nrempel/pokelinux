# Current lifecycle ARM execution and failed queued setup

The production Rust source from `90ced3a` now executes all 40 lifecycle library
tests on an actual generic Arch ARM kernel7.1.6-1-aarch64-ARCH as UID0. All 40
names match the frozen host inventory; ignored and filtered counts are zero.
The exact same-build daemon supplies the existing root-owned storage helper
test. Both ordinary planning-time daemon/socket cases also pass, including actual
UID1000 clients with groups[]/CapEff0 and normal actor waits/storage death.

The combined ARM run remains **failed**: its later queued-reply case does not
qualify its setup. The only fixture change adds native ARM ppoll73 qualification
alongside host poll7/ppoll271; production code and original deadlines are
unchanged. The generalized fixture also passes separately on the host. No
retry, silent exclusion, weakened guard or replacement ARM success is used.
[Bounded provenance](evidence/native-lock-queued-storage-arm-provenance.json)
keeps the successful executed subsets and failed aggregate distinct.

After observing a poll sleep location, the fixture stops the actor but reads
`-1 SP PC` from `/proc/PID/syscall`, without a syscall argument or fd-count
witness. It therefore refuses to qualify the main poll. The failure precedes
both the valid queued lock packet and the grant packet; it supplies no ARM
queued-send behavioral result. The recorded denied handshake is not that result.

Upstream ARM64 signal handling can clear the saved syscall number before a
group stop, so a negative syscall field is consistent with interrupted poll.
The earlier sleep sample cannot exclude a scheduling gap or establish which
poll was interrupted. This is an inference from versioned upstream v6.12 source,
not a source match for the exact guest7.1.6 build. The guard remains unchanged.
See [ARM64 signal handling](https://github.com/torvalds/linux/blob/v6.12/arch/arm64/kernel/signal.c#L1308-L1344)
and [the syscall collector](https://github.com/torvalds/linux/blob/v6.12/lib/syscall.c#L7-L36).
A future ordinary syscall-entry/exit witness would need to prove the exact
poll arguments and queued packet boundary; none is implemented here.

The failed fixture kills and waits its actor and peer with status-9. It observes
storage pidfd death but no storage proc absence before guest shutdown. This
does not establish native actor-owned storage cleanup. QEMU naturally waits0
after31.319s within360s; attached CLI and container wait0, with fresh exact-CID
absence on the same daemon. Those prove outer closure, not the missing storage
parent wait. The four earlier listing/library/planning processes wait0 with
stdout EOF/proc absence; the queued fixture itself waits1 with EOF/proc absence.
Root rehashes all 120 reviewed input bindings after the run.

The host queued RED→GREEN remains qualified separately. This ARM report adds no
send-EAGAIN, completed-intervening-write, Source/UI, original lock action, PAM,
auth scope/broker, armed receipt, unlock/clear or phone evidence. Library closure
stamps remain test models. Source100ms, matched Google/kernel separation and all
hardware gates remain unchanged. The next coherent implementation is private
atomic arming→exact unlock→versioned synced clear through the actual actor/storage
path, with test-only authentication and production ingress absent until genuine
whole-owner completion provenance is independently qualified.
