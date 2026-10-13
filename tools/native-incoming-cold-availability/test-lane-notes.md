# Actual cold incoming Qt availability diagnostic

The new diagnostic reproduced a terminal cache-expiry failure while fresh authentic source packets were already queued for the actor. The separate warm-focus control passed. No availability fix, source revival, native API/UI change, longer lease or production deployment claim is included.

| Exact epoch | Actual scope | Result | Actual source-epoch poll gap | Publisher sample gap | Trace drops | Reaps |
| --- | --- | --- | --- | --- | --- | --- |
| cold-reopen-timing-arm-v1 | Six prerequisite groups; held readonly close/reap preserves writer; new normal Qt launch | Cold current-source admission failed | 102.027265 ms | 93.797267 ms | 0 / 0 | 48 / 48 |
| warm-focus-timing-arm-v1 | Six groups; existing normal Qt current focus and incoming history retained | Passed control | 22.880573 ms | 31.911124 ms | 0 / 0 | 43 / 43 |

Both use the exact frozen v2 core head 518954b, original Hyprland 0.56.2-4 ELF ae9a9e..., unchanged installed normal production Qt ELF4aa..., and the actual ARM64 7.1.6-1 kernel. Original receiving writer and readonly child remain distinct. The new diagnostic fixture ELF is f288ac3c1c47321315aadb1eadd87b8a1c9e145f4089b11767acf1629ed570d8; its formatter/Clippy checks passed and independent readelf metadata confirms static AArch64. Build-1 is a preserved Clippy diagnostic, not behavioral RED. Build-2 freezes the corrected instrumentation sources before compiling. The unchanged lifecycle launcher is d7d05cea88594e7315cbe104f22249b1246b08926030cd2ba94442bc159b7433.

The original uninstrumented full-matrix v1/v2 remain failed. Their cache ages at failure were 111.463790 ms and 110.721490 ms. The first v1 reopened group lacked an accepted focus event for its new PID and does not qualify history restoration. v2 explicitly failed current-source admission for the new Qt PID. Their successful accepted-snapshot maximum gaps of 63.551576/64.103446 ms did not measure every compositor send. Exact historical selectors/results are indexed in baseline-evidence.json.

In the fresh cold trace, compositor PID349/UID1000/GID1000 sent source sequence313 sampled102.003506862 s. Observer service PID396/UID0/GID0 last polled at102.021401991..102.021442257 s and returned sequence313. Its next poll began102.123429256 s and returned TimedOut by102.123778088 s. The observer's cached sample was119.922394 ms old at that poll begin.

The exclusive real source channel had meanwhile received full successful original sendmsg attempts:

| Sequence | Sample time (BOOTTIME seconds) | Actual send begin / end | Return | Sample age at failed poll |
| --- | --- | --- | --- | --- |
| 314 | 102.028788913 | 102.028876818 / 102.028931420 | 169 bytes, errno0 | 94.640343 ms |
| 315 | 102.122586180 | 102.122772238 / 102.122886261 | 169 bytes, errno0 | 0.843076 ms |

The313→314 interval was25.282051 ms and314→315 was93.797267 ms, both within the unchanged100 ms lease. There is no second source receiver: the real Observer exclusively owns that channel. Its frozen implementation tests cached expiry before recvmsg, so it rejects this expired cache before draining the two actual live queued packets. A stale decoded packet would instead produce InvalidData. This identifies the immediate failure mechanism in this instrumented epoch.

Relaunch was requested at100.851216475 s and acknowledged at101.132157132 s,991.272124 ms before the failed poll. The frozen actor sends desktop Ok only after trusted-executable validation, spawn and owned pidfd registration. Thus this particular terminal polling gap is after that synchronous initial launch path returned. It does not establish which later actor operation or scheduler/graphics event caused the pause.

The actual original publisher had a concurrent93.797267 ms gap; the observer poll body itself took only0.348832 ms on its failed call. Exact public release sources show rendering/backend callbacks and the25 ms source timer share the original Wayland event loop. This supports a possible shared graphics/scheduling pause, but one instrumented failure and a warm control do not establish its ultimate cause. Trace packets themselves perturb scheduling. All timing is observed guest BOOTTIME under QEMU/Harbour, not a hard realtime or physical-device guarantee.

Private timing channels are separate from the source channel. Guest reception checks kernel SCM_CREDENTIALS exactly against the owned publisher and actor PID/UID/GID, packet sizes64/72 bytes, no truncation or descriptor rights, and bounded record retention. Producer trace sends are MSG_DONTWAIT|MSG_NOSIGNAL; failed trace writes increment only their drop counter and never affect source state. All2281 cold and1991 warm trace records passed those checks, with zero drops. The original source frame shape, socket-derived pidfd transfer, source ownership,25 ms timer,100 ms budget and permanent failure semantics are unchanged.

The diagnostic adapter ELF874f09a612b5c17def76e4e3ad4e079dcca9d07275ab462fe8ed22857cbc46ac was independently compiled against the installed exact original headers in both guests before source load. Cold v1 recorded its hash but its original guest did not export the bytes; the later warm label retains diagnostic-adapter.so and matches that exact hash. This is byte recovery by matching independent compilation, not a changed cold selector. Normal captured frames were inspected from actual original-compositor screencopy: initial grant/restoration show the +15550100 conversation and revocation shows Messages access denied. The captures qualify conversation visibility, not literal body display; durable synthetic body hello is independently present in raw/inbox rows. No new-app restoration frame is qualified after terminal cold source expiry.

The desired actual cold regression was authored before any availability fix: after initial actual source/grant/data positives, close/reap the held readonly/app while retaining the same receiving generation, launch a new normal app, and require accepted current source focus for that new PID and real history presentation under the unchanged lease. Root owns any future fix and further process/ARM reds for revised queue handling. These diagnostic observations are not a replay of such a fix.

Each VM used the existing immutable receiver backing read-only, snapshot=on, shared input lock and exclusive label lock; an exact owned no-NIC container capped at6 CPU/8 GiB with4 guest vCPUs. Cold graphics epoch ran after the other observed VM completed; warm ran sequentially after cold. No new VM clone, unrelated workload mutation, phone or private/proprietary publication occurred. README/main handoff remain Root-owned; their existing cold-availability gate remains appropriate.
