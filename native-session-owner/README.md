# Native session owner

This Rust entry owns one compositor Child and composes the existing Source
Observer, lifecycle actor, per-app Wayland supervisor and audio client. The fixed
Sounds desktop request still supplies no permission, route or executable.
This is the Sounds session assembly; the full smartphone system remains open.

The new service starts the compositor itself. Its private credentialed socket
passes through the fixed internal bootstrap on stdin, then only the selected
fd>=3 crosses the final UID1000 compositor exec. No inherited raw-FD launch CLI
is installed. A fresh incarnation and original startup deadline are fixed before
spawn; no automatic Source replacement or route replay occurs. First Source
readiness may use at most120 seconds. The separate bootstrap receive cap is
five seconds; subsequent samples retain the existing100ms lease.

SourceOwner retains Child custody before every post-spawn fallible operation.
Poll is nonblocking and terminal errors still drive actual try_wait. Stop fences
local facts and signals KILL immediately; occupancy clears only after actual
wait. The parent lifecycle fences permissions before its normal provider Stop.
Every returning outer setup path drains the same owner with one original cleanup
cutoff. Existing storage/actor fail-stop aborts are not graceful-return cleanup.
No production poll writes to a potentially blocked logging stream.

The existing root-admin Snapshot reports bounded [Source observation history](../docs/native-source-observation.md):
state, last actually accepted sample metadata and first categorized actor-observed
failure, including a typed first Observer rejection when available. Its bounded
facts distinguish receive, drain, frame, sample-chain and freshness decisions
without retaining packet bytes or peer identity. This is historical operational state; `ready` remains policy/storage
readiness. Collect a Snapshot before Stop if needed, since the actor may exit.
The first failure survives cleanup; observation does not change authority or retries.

The display Factory accepts a configured Wayland pathname before it exists.
An admitted launch connects nonblocking and checks the actual peer against the
retained compositor pidfd/PID/UID/GID before the owned SDK setup. Sounds gets
only its fresh per-app listener; the original desktop runtime stays private.
Listener withdrawal precedes SDK termination, including withdrawal failure.
These checks do not claim containment of every compositor descendant.

## Build and stage

```sh
cargo build --manifest-path native-session-owner/Cargo.toml --locked --release --bin pokelinux-session-owner
cargo build --manifest-path native-lifecycle/Cargo.toml --locked --release --bin sounds-launch
make -C native-session-owner install \
  SESSION_OWNER_BIN="$PWD/native-session-owner/target/release/pokelinux-session-owner" \
  SOUNDS_LAUNCH_BIN="$PWD/native-lifecycle/target/release/sounds-launch" \
  DESTDIR="$PWD/session-stage"
```

The install target stages the two native binaries, desktop file, unit and examples.
It does not activate a host session or install examples as operational policy.
Install the normal Sounds Qt/QML target through its existing CMake rules.
Supply a matching original Hyprland/Source-adapter build and a root-installed
compositor configuration that loads that adapter. Full Omarchy shell setup and
device graphics remain separate from a minimal Source composition.

The deployment must create root-owned nonwritable runtime, display and policy
ancestors, UID1000 compositor HOME/runtime0700 and UID11042 app HOME/runtime0700.
The `policy` field names an existing root-owned directory with mode0700, such as
`/var/lib/pokelinux/session`. It is the lifecycle storage directory, which owns
its policy and durable owner files; a policy JSON filename is invalid. Prepare
the selected directory before startup, for example as root:

```sh
install -d -o0 -g0 -m0700 -- /var/lib/pokelinux/session
```

Configuration validation does not create the directory or change storage data.
The examples show the schema; choose actual trusted paths/accounts for the
installed profile, keeping the unit Stop runtime equal to the configured
runtime. The sole audio service at11040 and appearance feed must be
ready independently before startup. This unit does not start or replace them.
An independent Root policy grant is required; configuration cannot grant access,
clear durable locks or restore a failed Source.

The normal entry takes one root-installed JSON path. `--stop ABSOLUTE_RUNTIME`
uses the fixed bounded typed Stop client without reading installation/profile
files. A positive Stop reply acknowledges the actor's reply; actual owner waits
and independent effects remain distinct evidence. The fixed internal app-entry,
storage-worker, display-context-worker and compositor-bootstrap modes reuse
existing ownership boundaries, with no arbitrary service-command API.

The unit uses KillMode=control-group for manager-owned descendant termination
and Restart=no. Its metadata alone proves neither executed unit containment nor
native descendant reaping. SourceOwner's host tests qualify its direct leader
Child only. Do not treat fallback manager killing as successful native cleanup.
Root needs the filesystem/UID/GID/signal capabilities used by the existing
listeners and workers, including chmod after chown (FOWNER).

## Validation scope

Host component tests use actual UID1000 Source processes, credentials, pidfds,
filesystem roles and owned children. Display tests use a real unresponsive
Wayland peer and actual SDK registry requests, with Preparing rather than an
invented Ready state. They qualify no original compositor, ARM execution, GUI,
phone or complete installed Source/audio path. The later combined run must bind
its own exact source/artifacts and original compositor/normal Qt/graph/samples.

Run the explicitly ignored root integration tests only in an isolated prepared
container with built feature-gated helpers. The `isolated-fixture` helpers are
not installed by the Makefile. See [session report](../docs/native-session-owner.md)
and [current integration map](../docs/native-integration-map.md) for the executed
selection and remaining gates. No physical phone is accessed.
