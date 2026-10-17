# Shared appearance client policy

This app-local Rust core owns the public appearance request, schema3 validation,
normalized cache, owner/revision checks, socket generations, readiness admission,
request/lease deadlines, retry and change notification decisions. It introduces
no worker or thread. The existing Qt event loop executes copied native plans and
projects RGBA16 colors and style tokens into QObject/QML values.

The public11-operation C ABI is [pokelinux_appearance_client.h](include/pokelinux_appearance_client.h).
Handles retain no caller pointers, file descriptors or provider authority. Calls
and free must not overlap. Qt owns the actual nonblocking SEQPACKET socket,
notifiers, root peer/packet credentials and closure of received descriptors.
Native resources retire before handle free; CLOSED means local retirement only.

Each refresh has one500ms CLOCK_BOOTTIME deadline. EAGAIN/EINTR await a new actual
notifier delivery without renewing the request. Captured connection generations
are checked in Rust. Cached expiry is independent; available is a pure fresh
clock check. The periodic250ms Qt timer starts once and does not reset on replies.
The constructor needs no successful clock read. Production time is never injected.

Schema3 accepts only the exact published palette/style shape or explicit
unavailable appearance:null. All nine RGBA16 roles, eight font tokens and six
spacing tokens are copied and compared without an RGBA8 conversion. Same-owner
revision rollback and changed content at the same revision refuse publication.
Invalidation retains the last accepted snapshot for presentation, without granting
freshness. Canonical32-lowercase-hex owners and Unicode scalar JSON are explicit
wire rules; this is no exhaustive Qt parser-equivalence claim.

Native CMake uses locked offline Cargo by default. Cross builds select a matching
archive and its SHA256 together:

```sh
cmake -S native-app/appearance-client-tests -B build/appearance \
  -DPOKELINUX_APPEARANCE_CLIENT_ARCHIVE=/absolute/libpokelinux_appearance_client.a \
  -DPOKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256=ACTUAL_SHA256
cmake --build build/appearance
ctest --test-dir build/appearance --output-on-failure
```

The independent C++ consumer tests eight policy groups with real BOOTTIME and
absolute timerfd delivery, plus one test-only wrapped libc EIO. Its authored native
result reports are CPU inputs. The separate unchanged29-case socket probe tests
actual forked peers, credentials, SCM_RIGHTS closure, delayed recvmsg and QColor/
QML projection. Run that probe only in its disposable root test environment.
The internal counter regression uses private test-only state, never a production
setter. Full app, installed QML, ARM and visible-session evidence must select
this exact header/archive/adapter cohort separately.

Closed qualification now passes formatting/strict Clippy, the counter unit, eight
frozen C ABI groups and all29 native Qt cases. Fresh joined normal Sounds/Messages
host builds, default Cargo and installed-QML controls pass separately. See the
[report](../../docs/native-appearance-rust-client.md). A fresh [current ARM
CMake/install epoch](../../docs/native-current-arm-apps.md) now links this matching
archive into both normal apps, checks native loaders and installation, and runs
their frozen nine/six C ABI groups. It does not repeat appearance8/Qt29 or run
either normal app main/window. ARM default Cargo, the full running ARM cohort
and physical-device gates remain open.
