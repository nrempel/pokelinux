# Installed native radio operator harness

This independent harness targets the real installed Rust Main, supervised radio
worker and finite intent writer under generic ARM64 Linux Binder. It uses the
existing owned VM envelope and a separately built public Binder peer. The installed
operator now passes all six fresh generic ARM cases, with actual native/PID1
closure and unchanged-marker refusal. See the [operator report](../../docs/native-radio-operator.md)
and [bounded provenance](../../docs/evidence/native-radio-operator-provenance.json).
The original missing-command baseline RED, failed unit/fixture epochs and shared
parent/worker channel checks remain separate evidence.

The maintained files are the original five-case consumer/typed assertions, separate
manager-stop consumer, concrete GREEN adapter,
credentialed socket client, public config/vector, independent Rust fixture and
`envelope/` setup/selection/runner. See [INTEGRATION.md](INTEGRATION.md) for their
boundaries. Per-run review manifests, candidate source snapshots, actual Cargo
JSON/build proofs, selected ELFs and runtime results belong in separately reviewed
immutable execution inputs; they are not maintained source or checked-in outcomes.

Prepare one reviewed freeze of these sources with `REVIEW-MANIFEST.json`. Its
`files` map binds each relative source path to its `sha256`, byte count and line
count. Preserve the exact consumer and fixture bytes used by the reviewed oracle.
Before planning, obtain a successful closed strict offline build of the actual
candidate and six binaries: manager, peer, controller, radio-service, radio-worker
and radio-status. Native component success alone does not qualify this integration.

The actual artifact qualification must bind `harness_manifest_sha256` to this
reviewed freeze and `production_source_pins_sha256` to the immutable candidate
source-pins JSON. It must retain the successful `fixture-arm-cargo.jsonl` and
`production-arm-cargo.jsonl` beside it, with actual artifact paths and hashes.
The planner reads those Cargo streams and actual ELF bytes itself. Candidate source
pins use `source_pins` (relative path to SHA256) and
`whole_HEAD_equality_required:false`; exact selected bytes, not unrelated HEAD
changes, define the candidate.

After reviewing the actual build, select one case using its real artifact values:

```sh
python3 "$freeze/envelope/plan.py" \
  --output /tmp/pokelinux-radio-operator-green-arm-accepted-1 \
  --case accepted --review-manifest-sha256 "$harness_manifest_sha256" \
  --candidate-source "$candidate_source" \
  --candidate-pins "$candidate_pins" \
  --candidate-pins-sha256 "$candidate_pins_sha256" \
  --artifact-qualification "$artifact_qualification" \
  --artifact-qualification-sha256 "$artifact_qualification_sha256" \
  --bin manager "$manager_elf" "$manager_sha256" \
  --bin peer "$peer_elf" "$peer_sha256" \
  --bin operator-controller "$controller_elf" "$controller_sha256" \
  --bin radio-service "$service_elf" "$service_sha256" \
  --bin radio-worker "$worker_elf" "$worker_sha256" \
  --bin radio-status "$status_elf" "$status_sha256" \
  --build-proof "$build_execution" "$build_execution_sha256"
```

Review the generated selection and command before separately authorizing runtime:

```sh
python3 "$freeze/envelope/run-selected.py" \
  --selection-dir "$selected_epoch" \
  --selection-sha256 "$selection_sha256" --command-sha256 "$command_sha256"
```

Run each of `accepted`, `expired-before-admission`, `permit-expires`,
`stop-before-consumption`, `writer-held` and `manager-stop-held-writer` in a fresh
selected epoch with a unique
output label. Keep the original deadlines/resources. Never clear permanent
markers, reset capture, reuse an epoch or retry an operation to obtain a pass.
These scripts retain the managed workspace's existing read-only boot inputs and
shared VM input lock; they do not build or provision replacement inputs.

The selected unit must have no ExecStop and use KillMode=mixed. Observe ExecStop
through the existing system bus: Manager.GetUnit returns its typed object path;
Service.ExecStop must be exactly the empty a(sasbttttuii) array. These types follow
[systemd's primary D-Bus interface](https://raw.githubusercontent.com/systemd/systemd/main/man/org.freedesktop.systemd1.xml)
and [busctl's typed output](https://raw.githubusercontent.com/systemd/systemd/main/man/busctl.xml).
Both actual busctl child waits share the original show deadline and use regular
stdout files, with no PIPE EOF claim. Missing busctl/bus/property or malformed
output is setup failure; a missing systemctl display field never means empty.
No package or daemon is added. The sixth case
holds the actual admitted writer, requires three responsive status visits and
zero105, then invokes actual systemctl stop under the same5s caller budget. It
uses an ordinary pre-Stop diagnostic snapshot and claims no socket StopAck. Both
native children must reach actual signal9 waits and PID1 must record Main0/success.

Qualify actual setup, UID separation, root-only admission, exact writer sync plus
normal wait0 before permit, independent Binder105/response observations, typed
Stop snapshot and both native waits. Also require separate PID1 Main0/restart78,
unchanged marker inode/bytes, no successor dispatch, empty/gone unit cgroup, every
actual direct-child wait and selected PIPE EOF before its original cutoff, and
exact owned-CID absence with unchanged inputs. Derive wait/EOF counts from results.
A CLI exit0 or copied summary alone is insufficient. No phone, vendor cleanup,
carrier/APN/IMS authority or response crash persistence follows from this fixture.
