# Common native observations to Rust planning

The public [Rust planner](../../../native-render/src/format.rs) implements the
[frozen V2 contract](CONTRACT.md). Six actual C++/Rust groups pass with all64 prior
Rust tests and the five previous C++ programs; see the
[report](../../../docs/native-render-format-policy.md). Inputs in those tests are
authored CPU facts.

The [incremental native patch](common-native-collector.patch) applies only to the
exact two-file before cohort recorded in [source-cohort.json](source-cohort.json).
It requires the preceding open-source compositor patch stack, the matching Rust
archive/four production headers and exact AQ15 library/SDK. A shared bounded
collector now invokes Rust after real backend startup and after CHANGE. Both old
C++ first-tuple choices and the duplicate C++ corroboration policy are removed.
Native output/mode/device objects and the copied Rust request mode stay retained
on the native worker. Existing mode corroboration remains unchanged.

Collectors preserve raw native order, empty first modifier lists, actual query
returns/errno/comparison counts and each output's own device context. Complete
blob decoding precedes pair publication; malformed data cannot expose partial
pairs. Excess observations return CAPACITY before Rust; zero spans use null.
Neither collector queries nor a positive plan authorize display mutation.

The matching full NORMAL compositor/SDK build passes:458 configured C++/PCH
entries, all31 Rust exports,537 regular SDK files/two links and all original
regressions/strict interface checks. The first build compiled/linked but failed
SDK packaging because the fixture omitted the format header; four staging lines
repair it without changing production code or assertions. That aggregate stays
failed. Actual normal bootstrap/CHANGE remains pending. Native prepare/execute, real client frames, scanout
replacement/retirement and authorized Stop remain open. Process disposal does not
establish native cleanup. Source, ARM and all Pixel hardware gates stay separate.
No physical phone was accessed.
