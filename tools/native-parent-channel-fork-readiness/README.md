# Parent readiness after fork

This standalone native x86_64 Linux control passes against the shared process
owner. A real silent worker reaches the normal Supervisor/BoundParent bootstrap.
The parent's readiness hint is false; one fork child then calls the inherited
hint, requires `InvalidInput`, and exits through `_exit` without dropping the
inherited Supervisor. The original parent observes its pidfd and actual exit 0
wait/ECHILD before obtaining one fresh bounded reply from the same worker.
Normal Stop, worker signal 9 wait/PIPE EOF, and durable successor refusal follow.

The [curated evidence](evidence.json) binds the exact formatted source, six
existing release rlibs, direct-rustc ELF, and separate compiler/runtime receipts.
Formatting and compilation with warnings denied pass; actual fork execution is
a later, separately admitted pass. Each outer owner closes all 14 Docker CLI
waits/EOF, actually waits its container with exit 0, removes the exact CID/name and verifies
absence, unchanged inputs and ECHILD. The fork child has no PIPE EOF claim.
This qualifies host fork ownership; Main idle persistence, GUI, ARM and physical
hardware remain separate controls. No behavioral fork baseline RED is claimed.

The published Rust files are the exact formatted compiler inputs. `common.rs`
is copied unchanged from the existing parent-channel contract. The Cargo
manifest declares dependencies for source reuse; this qualification ran direct
`rustc`, with no Cargo execution or consumer Cargo-artifact claim. The prior
unexecuted source-only preparation is recorded separately in the evidence.

For the recorded build, mount these two source files at `/out/formatted/` and
the six pinned rlibs at `/inputs/deps/` inside the evidence's compiler image.
Use its exact `compiler.rustc_argv`; no dependency resolution is needed. Keep
sources/libraries readonly and retain compiler wait/EOF before qualifying the
ELF. Execute only that pinned ELF, as UID/GID 1000 with capabilities dropped,
no-new-privileges and no network, passing one fresh nonexistent private runtime
path. Retain external ownership through actual consumer/container wait and EOF.
The fixture fixes work/cleanup at 4 s/6 s and clips its single reply to 1 s; those
cutoffs do not renew. Unsafe fork/_exit is confined to the documented fixture
function, and the rest of this fixture remains under the unsafe-code deny lint.
