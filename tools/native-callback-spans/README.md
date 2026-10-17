# Bounded upstream callback diagnostic

This temporary diagnostic measures three existing main-thread Hyprland callback
boundaries while investigating cold installed startup. It changes no Source
lease, readiness rule, timer schedule, callback order or native ownership.
The production phone-service and app-core target remains Rust. The probe lives
at the retained C++ compositor boundary; it is not new phone policy.

Apply `callback-span-producer.patch` only to upstream Hyprland
`efb50993780079460b0cbed1363e2166a2de1d9f`. Its EventLoopManager.cpp baseline
SHA256 is `104ff43a4c02fcc88e8711cade68becd3a1fe099e970e4b1306774c85dd76f7c`;
the resulting file is `fa6453581abdff5d164fd6fca0cf2c34f121df3254352eae0a66c9884cb40988`.
The separate logical-output allowlist patch is still required by the selected
normal compositor build. Read [evidence and limits](../../docs/native-callback-spans.md)
before interpreting an output file.

The writer arms once in the existing event-loop constructor. It requires a fresh
zero regular file at `/run/user/1000/pokelinux-callback-spans.bin`, exactly82,048
bytes,0600,UID/GID1000,one link,no symlink. The runtime must preallocate it on
tmpfs, retain its descriptor/device/inode and the actual compositor main pidfd,
and prevent another writer. Missing or refused tracing leaves callbacks running.
Do not create the file while a selected compositor is already running.

Records bracket only Aquamarine `onSignal`, selected event-loop timer `call`,
and existing deferred `f.second` callbacks. Tokens copy identities without
extending object lifetimes. A return record means actual normal return; an
unmatched entry does not identify an internal cause. BOOTTIME measures elapsed
time; THREAD_CPUTIME measures CPU time of that thread. A temporal overlap alone
is not causation. Raw Wayland timer dispatch and other callbacks are outside
this probe's coverage.

The little-endian file contains a128-byte header and1024 rolling80-byte records.
Highest reservation precedes slot modification and final commit follows payload.
The reader tolerates only one incomplete final reservation. Overflow, missing
entries and failed clocks remain explicit. This is a single-writer protocol with
post-exit reading, not a concurrent snapshot API. Read the retained inode only
after exact writer death and source retirement; require expected PID=TID=the
actual compositor main. `callback_span_consumer.py` validates the format and
always reports `runtime_qualified:false`; external runtime custody and Source
stamp authentication must be established separately.

`producer-component.cpp` is the frozen source-extracted native consumer;
`extract_producer.py` extracts only the marked writer from the selected CPP.
Compile it as C++20 or later against that extracted `producer-under-test.inc`,
with warnings and an actual bounded foreground runner. Its output directory
must be fresh and writable by UID/GID1000. It creates real mmap/files/clocks and
checks three component groups. It neither compiles all Hyprland nor executes its
native callbacks. `format_check.py READER_PATH` supplies ten independently
crafted binary-format controls. Neither test qualifies cold startup or hardware.


`callback_observer.py` supplies the exact original-code runtime custody helper and
finite `FDIdentityObserver`. The latter takes at most two samples through an
existing caller watch loop, borrows its authenticated main pidfd, and reads only
bounded proc metadata. Its first binding consumes custody; a second sample can
occur only at least50ms later under the original caller WORK deadline. No target
FD is opened/read/duplicated. Maximum64 rows per sample and512KiB total; errors
consume attempts. The caller must establish native identity before binding and
interpret Source ordering only from authenticated original records after exit.
Repeated metadata equality never proves descriptor continuity or object identity.

`real_proc_consumer.py --control` exercises independent Linux pidfd/pipe/eventfd/
epoll metadata. Its `--helper` option loads the actual observer and exercises four
real-proc groups, including a genuinely closed/reused fd and wrong starttime.
Use an actually owned external runner with the original5s work/8s closure;
the consumer's cooperative work check is not a kernel-interruption guarantee.
These are finite host metadata tests, without a compositor, Source or phone.
