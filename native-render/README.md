# Immutable SHM payload custody

This original Rust library owns bounded immutable CPU pixel snapshots for the
[connected Hyprland import](../tools/native-render-import/README.md). It copies
exactly `stride * height` bytes before returning success, retains format/shape and
import identities, and charges distinct live pixel allocations once across
explicitly cloned handles. Releasing the control store does not invalidate
snapshots. Each C handle has one owner; a caller must clone before transferring
another independently owned handle. Pixel views borrow that owned handle and
must end before its release.

The store admits at most512MiB; one snapshot admits at most64MiB and each extent
at most16384. The compositor wrapper selects a128MiB store. Invalid identity,
reserved fields, layout or input length publish no snapshot. Capacity refusal
leaves the input with its caller and consumes no additional budget. Rust owns no
Wayland, client-buffer, C++ scene, EGL or output object.

Bulk pixel allocation uses `try_reserve_exact`. Handle and `Arc` metadata use
infallible allocation: allocator exhaustion can terminate the process rather
than return `PR_ALLOCATION`. The capacity bound is a live pixel-byte budget,
not a bound on allocator overhead or a complete out-of-memory recovery policy.

This prerequisite does not implement the rendering executor or move GL/context,
GPU buffers, fences, output commits or resource deletion to another owner.
A successful snapshot does not establish upload, GPU completion, presentation,
secure lock display, Source freshness or phone support. The prepared rendering
[authority baseline remains RED](../docs/native-render-authority.md).

## Build and actual boundary test

The crate is std-only and has no downloaded dependencies. For a native Linux
x86 build, run from this directory:

```sh
cargo fmt --check
cargo clippy --offline --locked --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
cargo rustc --offline --locked --release --target x86_64-unknown-linux-gnu -- --print=native-static-libs
```

Use the archive path Cargo actually reports, including any explicit
`CARGO_TARGET_DIR`. Compile `tests/ffi.cpp` against that exact archive and
`include/pokelinux_render.h`; link the native libraries reported by that Rust
build. The current x86 compiler reports `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`.
The C++ flags are `-std=c++20 -O2 -Wall -Wextra -Werror -pthread`.

The test uses an actual shared memfd and forked producer: capture, mutation,
unmap/close and actual child wait precede the retained-byte check. A separately
owned clone crosses a joined C++ thread after the original handle and store are
released. Remaining groups check ABI/store admission, quota lifetime and invalid
admissions without partial output. Success emits five named groups plus actual
producer and thread closure receipts under original BOOTTIME cutoffs. These are
CPU custody checks; they cannot qualify compositor import or rendering behavior.

Formatting, Clippy and the x86 Release Rust archive compilation have executed
successfully. A separate actual x86 KVM guest compiles and runs the C++ test: all
five groups pass, with producer wait/EOF and actual thread join retained. The
connected full Hyprland build and matching501-file SDK also pass. These results
remain separate from actual compositor runtime and rendering acceptance; see the
[build report](../docs/native-render-import.md).

A later separate [actual compositor no-attach case](../docs/native-render-import.md#actual-no-attach-identity-red)
qualifies RED: a retained unassigned surface is unnecessarily reimported despite
unchanged pixel bytes. A one-file incoming-buffer guard repair is authored;
its matching full rebuild/five FFI groups and unchanged actual no-attach oracle
pass. The repaired import identity remains stable7→7 with the same bytes. This
does not repair the seven-case Source/rendering RED or qualify upload, padded
cursor consumption,
server-resource retirement or complete rendering/output ownership.
