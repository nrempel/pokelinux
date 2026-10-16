# Actual Source timer boundary fixture

The root-only component tests launch an authored UID1000 C++ producer through
the unchanged native session-owner bootstrap. The producer uses the adapter's
shared `source_sampler.hpp` packet publisher and an actual libwayland event
loop/timer. A separate fixture-only socket selects dispatch. No Source protocol,
clock, receiver deadline or production CLI is extended.

The frozen two-case baseline executes on a generic ARM kernel: early publication
fails with an actual Rust Snapshot, while the activated-source expiry control
passes. Only the shared publisher's first-dispatch gate changes for GREEN; both
unchanged cases then pass. Initial150ms silence precedes fresh sequences1/2;
a later real stall reaches permanent `TimedOut`, including after timer dispatch
resumes. The configured timer is25ms; its observed sample gap is25,690,904ns,
without a timer-accuracy guarantee.

The test startup limit is5s within the production120s maximum. Each case has one
original8s child-custody cutoff; the unchanged receiver lease is100ms. Source
children are killed and actually waited even on assertion failure. Their output
is DEVNULL, without a child PIPE EOF claim. Native adapter SDK compilation is
separate from original Hyprland execution, normal GUI/PCM and hardware.

Build the Rust test with:

```sh
cargo test --offline --manifest-path tools/native-source-timer-boundary/Cargo.toml \
  --features isolated-fixture --test timer --no-run
```

In a prepared isolated root guest with the matching native Wayland SDK, compile
`timer-fixture.cpp` with `-std=c++23 -Wall -Wextra -Werror` and
`pkg-config --cflags --libs wayland-server`, including
`../native-session-compositor`. Install the fixture and unchanged default
`pokelinux-session-owner` in root-owned nonwritable paths. Set
`SOURCE_TIMER_BOUNDARY_FIXTURE`, `SOURCE_TIMER_BOUNDARY_OWNER` and a root0755
`SOURCE_TIMER_BOUNDARY_ROOT`, then execute the test binary with
`--ignored --test-threads=1 --nocapture`. Preserve native compile, child wait,
SDK and outer process closure evidence.

See the [startup report](../../docs/native-source-timer-startup.md) for the exact
evidence and remaining original-compositor integration gate.
