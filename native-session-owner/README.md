# Native session owner

This Rust entry owns one compositor Child and composes the existing Source
Observer, lifecycle actor, per-app Wayland supervisor, audio client and explicitly
selected local Messages history. The fixed Sounds and Messages desktop requests
supply no permission, route or executable. The full smartphone system remains open;
see the [composition report](../docs/native-messages-session-composition.md) for
current component evidence and the remaining normal-session gate.

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
In history mode the Projector is retained before subsequent fallible setup. Every
returning outer setup path fences Source and history before draining both against
one shared original two-second cutoff; retained occupancy aborts. Existing
storage/actor fail-stop aborts are not graceful-return cleanup.
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
retained compositor pidfd/PID/UID/GID before the owned SDK setup. Each admitted
fixed app gets only its fresh per-app listener; the original desktop runtime stays private.
Listener withdrawal precedes SDK termination, including withdrawal failure.
These checks do not claim containment of every compositor descendant.

## Build and stage

```sh
cargo build --manifest-path native-session-owner/Cargo.toml --locked --release --no-default-features --bin pokelinux-session-owner
cargo build --manifest-path native-lifecycle/Cargo.toml --locked --release --no-default-features --bin sounds-launch
cargo build --manifest-path native-messages/Cargo.toml --locked --release --no-default-features --bin messages-launch
make -C native-session-owner install \
  SESSION_OWNER_BIN="$PWD/native-session-owner/target/release/pokelinux-session-owner" \
  SOUNDS_LAUNCH_BIN="$PWD/native-lifecycle/target/release/sounds-launch" \
  MESSAGES_LAUNCH_BIN="$PWD/native-messages/target/release/messages-launch" \
  DESTDIR="$PWD/session-stage"
```

The install target defines eight staged files: three native binaries, two desktop
files, the unit and two examples. It does not activate a host session or install
examples as operational policy. Install the normal Sounds and
[Messages Qt/QML targets](../native-messages-ui/README.md) through their CMake rules.
Supply a matching original Hyprland/Source-adapter build and a root-installed
compositor configuration that loads that adapter. Full Omarchy shell setup and
device graphics remain separate from a minimal Source composition.

### Explicit normal-session deployment bundle

`install-normal-session` stages a separately selected operational bundle. The
existing eight-file `install` target keeps its examples-only scope. Supply an
absolute source root, an absolute JSON manifest and a private absolute DESTDIR:

```sh
make -C native-session-owner install-normal-session \
  SESSION_DEPLOYMENT_ROOT="$PWD/selected-deployment" \
  SESSION_DEPLOYMENT_MANIFEST="$PWD/selected-deployment.json" \
  DESTDIR="$PWD/session-stage"
```

The manifest is `{"schema":1,"files":{"etc/pokelinux/session.json":
{"bytes":123,"sha256":"…64 lowercase hex digits…","mode":420}}}`. File keys are
canonical relative installation paths; modes are integers (420 is0644). Every
source must be a single-link regular file with the declared bytes, SHA256 and
mode. The installer rejects symlink components, traversal, duplicate JSON keys,
path collisions and unsafe modes, and checks the complete selected source and
destination metadata before its first write. Bounds are4096 files,8MiB per file
and64MiB total. Existing destination parents keep their modes; newly created
parents are0755. Run it only against an installer-controlled staging tree.
Files are replaced individually with fsync; an I/O failure can leave a partially
staged tree and must remain a failed installation.

This build-only target fetches nothing and chooses no UID, HOME, device, policy
or service defaults. It does not create accounts, grant permissions, enable or
start units, or retry startup. The operator supplies and reviews those deployment
inputs. A Root-selected virtual Pixel10 bundle now passes the frozen host
GNUmake4.3 install consumer after an actual missing-target RED: eight core files
plus205 selected files, including all196 pinned Omarchy blobs and modes. Eight
filesystem cases also pass, including source corruption and path/symlink
refusals before copying. This qualifies staged selection; unit ownership,
dependency readiness, cold Source, full shell, Sounds and PCM startup still need
the actual installed VM trace. See the [normal tracer report](../docs/native-normal-upstream-tracer.md#explicit-installed-session-selection).

The deployment must create root-owned nonwritable runtime, display and policy
ancestors, UID1000 compositor HOME/runtime0700 and UID11042 Sounds
HOME/runtime0700. History mode also needs UID11010 Messages HOME/runtime0700
and three existing, separate root0700 directories for history runtime/raw/inbox.
The required `appearance_socket` has a protected parent. Both fixed profiles have
exactly six absolute arguments and bind the configured Wayland path, appearance
socket and session `app.sock`; configuration grants no app access.
The `policy` field names an existing root-owned directory with mode0700, such as
`/var/lib/pokelinux/session`. It is the lifecycle storage directory, which owns
its policy and durable owner files; a policy JSON filename is invalid. Prepare
the selected directory before startup, for example as root:

```sh
install -d -o0 -g0 -m0700 -- /var/lib/pokelinux/session
```

The required `messages` field accepts only `{"mode":"disabled"}` (Sounds only)
or `{"mode":"history","runtime":"…","raw":"…","inbox":"…"}` (both apps).
There is no incoming/backend selector or implicit default. Configuration validation
does not create directories or change storage data.
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
storage-worker, Messages writer/read-worker, display-context-worker and
compositor-bootstrap modes reuse
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
phone or complete installed Source/app/audio path. Current host36 components and
12 Source-status cases pass; that earlier aggregate later fails on a missing
fixture input and loses its tmpfs binaries. A separate completed556-input run
retains release opt3, feature-empty owner/launch clients, passes strict Clippy for
all three and three exact retained-owner history cases. The earlier36/12 cases
are reused, not rerun or combined into a new aggregate. Separate GNU Make4.4.1
staging passes all eight current files with exact hashes, root:root ownership,
0755 binaries and0644 metadata, without activation or test reruns. The normal
GUI/session runtime remains open. The later combined run
must bind its own exact source/artifacts and original compositor/normal Qt/graph/samples.

Run the explicitly ignored root integration tests only in an isolated prepared
container with built feature-gated helpers. The `isolated-fixture` helpers are
not installed by the Makefile. See [session report](../docs/native-session-owner.md)
and [current integration map](../docs/native-integration-map.md) for the executed
selection and remaining gates. No physical phone is accessed.
