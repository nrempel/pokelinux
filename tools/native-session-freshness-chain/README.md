These tools freeze and execute the native-session actual process test suite on isolated Root host Linux and actual ARM Linux. The source peer and observer helper are authored fixtures. This lane does not qualify a compositor, installed app, vendor backend or physical phone.

Use a dedicated existing RAM directory as the mutable Cargo target. Cargo dependencies are mounted read-only from the existing main `sim/rust-cargo`; builds are offline and locked. The pinned local Rust image receives eight CPUs and 12 GiB. The build freezes three crates, Cargo artifact metadata and static host/ARM test, peer and observer ELFs. It never executes the tests. Cargo's exact compiled `/cache/...` paths are preserved in the runtime, including the process fixture's embedded `CARGO_BIN_EXE` helper paths.

```
mkdir -p /dev/shm/pokelinux-session-freshness-cargo-cache-20261013
python3 -B tools/native-session-freshness-chain/build.py --phase red --label freshness-chain-red-build-2 --cache-path /dev/shm/pokelinux-session-freshness-cargo-cache-20261013
python3 -B tools/native-session-freshness-chain/run.py --phase red --arch host --build freshness-chain-red-build-2 --label freshness-chain-red-host-3
python3 -B tools/native-session-freshness-chain/run.py --phase red --arch arm --build freshness-chain-red-build-2 --label freshness-chain-red-arm-1
```

Each output label is new and immutable after completion. A selected RED exits one only after a completed libtest run returns 101 with exactly the four desired failures. Compile, bootstrap and missing-helper failures cannot qualify it. The full suite remains selected; `--test` exists for explicitly separate, exact-name diagnostics.

ARM execution reuses the existing immutable production guest backing under its shared input lock, snapshot mode and read-only mount. Its preparers require an exclusive lock. The no-NIC VM has four guest CPUs, within a six-CPU/eight-GiB Docker cap. The host runtime uses the pinned graphics-simulator image, which provides Python, as a separate Root container with no network or capabilities. Both runtimes validate the selected helper hashes before executing tests. The narrow runner acts as a subreaper only inside its own runtime, records actual waits for adopted descendants, and cleans its test session. `/proc` sampling establishes final absence for sampled helpers; it does not independently trace each helper's original Rust-parent wait.

After Root corrects the implementation, repeat build/run with fresh green labels and `--phase green`. `proof.py` validates full 34-test results, exact RED failure names and green success, rehashes every selected source/artifact, and requires that core differences are `native-session/src/lib.rs` plus the single existing test binding gaining `mut` for `snapshot(&mut self)`; it verifies that no test assertion/timing condition or peer input changed. It preserves unselected build evidence separately. Earlier milestones are not relabeled as these new tests.

The initial static host build without an explicit host target failed before test execution because Rust attempted to build a static proc-macro. `freshness-chain-red-build-1` preserves that unselected infrastructure failure. The corrected driver explicitly targets `x86_64-unknown-linux-gnu` for host compilation. Selected RED runtime results are frozen in `red-evidence.json`; the separately frozen correction passes all 34 tests on both host and actual ARM Linux.

`freshness-chain-red-host-1` preserves a pre-execution bootstrap failure: the pinned Rust build image has no Python executable. No native test ran there. Host epoch 2 uses the already pinned graphics image for its Python runner; static test/helper ELFs remain those selected by red build 2. ARM epoch 1 already used the same graphics image.

`freshness-chain-red-host-2` preserves a second pre-test setup failure: capability-free container Root could not write the host-user-owned results directory. Host epoch 3 keeps UID 0 and uses the output owner GID with an explicit group-writable owned output directory. It adds no capability and changes no native binary.

Selected RED: `freshness-chain-red-build-2`, host epoch 3 and ARM epoch 1 each complete all 34 tests, with 30 passing and exactly four expected failures. Timely actual heartbeat/cycle and the explicitly authored contiguous boundary return `TimedOut`; the backwards intermediate sample is coalesced away. All four new terminal-state negative controls pass. The frozen baseline library hash is `3f2e9d07b3163dcc057b85bef6d1f353f10b9f207558d5d41386532970da7bb2`, byte-identical to `10ce2f1`. Host kernel is `6.8.0-136-generic`; ARM kernel is `7.1.6-1-aarch64-ARCH`. Both runners observe final absence of sampled helpers and zero adopted orphan waits. The source/helper cleanup assertions inside Rust remain the original parent ownership checks.

Selected GREEN: `freshness-chain-green-build-1`, `freshness-chain-green-host-1` and `freshness-chain-green-arm-1` each execute the full process suite with 34 passing, no failures, no ignored/filtered tests and exit 0. Format, Clippy with warnings denied and static ELF checks pass. The corrected library hash is `610aa41e9c290ee1184ef994b74039485d4b1d485e553124e829312b12c5a220`. Only Observer implementation and the existing unknown-source test binding's required `mut` differ from the selected RED; peer source, all assertions and timing conditions are byte unchanged. These positives demonstrate timely queued source samples being examined before cached expiry. They do not renew a deadline captured by old work. An observed expiry, real sample gap, expired intermediate, backward intermediate or returned timeout stays terminal.

| Actual process epoch | Passed | Failed | Ignored/filtered |
| --- | ---: | ---: | ---: |
| Host RED, kernel 6.8.0 | 30 | 4 | 0 |
| ARM RED, kernel 7.1.6 | 30 | 4 | 0 |
| Host GREEN, kernel 6.8.0 | 34 | 0 | 0 |
| ARM GREEN, kernel 7.1.6 | 34 | 0 | 0 |

```
python3 -B tools/native-session-freshness-chain/build.py --phase green --label freshness-chain-green-build-1 --cache-path /dev/shm/pokelinux-session-freshness-cargo-cache-20261013
python3 -B tools/native-session-freshness-chain/run.py --phase green --arch host --build freshness-chain-green-build-1 --label freshness-chain-green-host-1
python3 -B tools/native-session-freshness-chain/run.py --phase green --arch arm --build freshness-chain-green-build-1 --label freshness-chain-green-arm-1
python3 -B tools/native-session-freshness-chain/proof.py --red-build freshness-chain-red-build-2 --red-host freshness-chain-red-host-3 --red-arm freshness-chain-red-arm-1 --green-build freshness-chain-green-build-1 --green-host freshness-chain-green-host-1 --green-arm freshness-chain-green-arm-1 --unselected-output freshness-chain-red-build-1 --unselected-output freshness-chain-red-host-1 --unselected-output freshness-chain-red-host-2 --output tools/native-session-freshness-chain/evidence.json
```

The commands above record the historical selections. A reproduction requires fresh output labels and the matching archived source for each phase; the current corrected core cannot reproduce the old behavioral RED without selecting its archived baseline. `red-evidence.json` remains unchanged. The final `evidence.json` binds both phases plus current authored infrastructure separately. Earlier compositor/Qt milestones remain bound to their historical binaries and were not replayed or relabeled here. The ARM runs are system emulation on an actual guest kernel, distinct from ARM user-mode or phone evidence. The authored exact-boundary and malformed-time inputs travel through real process sockets and actual credentials/pidfds; only the timely heartbeat/cycle positives use the sender's unmodified real BOOTTIME.
