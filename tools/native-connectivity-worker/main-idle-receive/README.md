# Actual Main empty-idle receive fixture

The historical exact `consumer.py`, [controller.py](https://github.com/nrempel/pokelinux/blob/256c553c1eb7c8434711772fa815d2d5ab21cc00/tools/native-connectivity-worker/main-idle-receive/controller.py), `main_trace.py`, `inside.py` and
`peer.py` are unchanged across the qualified native host
[RED/GREEN](../../../docs/native-connectivity-main-idle-receive.md).
This captured peer differs from the older shared peer; use this copy. Stage
these files at `/runner`, plus exact shared
[`owned.py`](../../native-installed-services/owned.py),
[`closure.py`](../../native-audio-preview/closure.py) and
[`worker_trace.py`](../idle-receive/worker_trace.py). Their hashes are in the
[evidence](../../../docs/evidence/native-connectivity-main-idle-receive.json).

The current four-path controller now passes all three healthy Main cases with
the new CLI providers and complete closure; the
[CLI report](../../../docs/native-connectivity-installed-startup.md) records this
separate execution. Original idle RED/GREEN results retain their recorded
controllers/providers. `current-candidate/main-source-proof.json` and its eight
snapshots bind the qualified new compiler outputs and current controller. Its
pre-execution preparation metadata remains frozen; the CLI evidence supplies
the actual subsequent result. No new-CLI GUI runtime is qualified.
The owning runner must regenerate every selected helper/runner hash from the
actual current files; no old manifest hash is accepted as a current binding.

Historical replay instructions follow; their `baseline/` and `candidate/` proof
examples belong only to the historical controller/provider cohorts.

Select normal x86_64 Release Main/worker from actual Cargo compiler-artifact JSON
and verify their exact hashes. Choose the matching `baseline/main-source-proof.json`
or `candidate/main-source-proof.json` as `/runner/main-source-proof.json`.
Beside it, stage `pinned-source/` with the seven exact compiled Rust source files
named by that proof's `files` map. Verify each declared hash; mutable HEAD is not
a substitute for the selected compiled snapshot. The proof files retain their
original preparation-time metadata; actual execution is qualified separately in
the report. Provider/source-proof inputs differ across RED/GREEN; executable
fixture bytes do not.

Reuse the existing owning container supervisor and actual closure contract:
three sequential fresh cases, network none, root read-only, no host devices,
2 CPUs/2 GiB and only NET_ADMIN/SYS_ADMIN/SETPCAP/SYS_PTRACE for trusted setup and
exact owned-process tracing. Main drops capabilities; only its normal worker
keeps NET_ADMIN. After actual loaders qualify, an owning runner invokes:

```sh
python3 -B /runner/controller.py --case idle \
  --main /providers/pokelinux-connectivity-main \
  --worker /providers/pokelinux-connectivity-service \
  --source-proof /runner/main-source-proof.json --output /results/case \
  --work-until-ns ORIGINAL_WORK_BOOTTIME_NS \
  --close-until-ns ORIGINAL_CLOSE_BOOTTIME_NS
```

Run `consumed` and `parent-death` in their own fresh namespaces/outputs. Keep
6-second cases with 3-second startup limits, 30/40-second inner and
180/200-second outer bounds, 80 ms holds, 50 ms receive duration and a 250 ms lease. No traced case submits Apply. Do not poll/wait Main
while the tracer owns it; failed nonterminal detach remains unqualified under
the retained outer container's custody.

Only after all three actual case/namespace/container closures may the existing
owner call the pure consumer's case qualifiers, `qualify_all`, then exactly one
`assert_desired_after_cleanup`. RED is its caught in-process desired AssertionError,
not an invented child exit or setup failure. The candidate must preserve a
fresh same-worker fact and both strict negative controls. Preserve private
runtime outputs; publish only the curated metadata/hashes. No Main idle result
attributes graphical root4 or qualifies Settings/phone behavior.
