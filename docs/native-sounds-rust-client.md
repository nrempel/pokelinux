# Sounds request and model engine in Rust

The app-local Rust engine now owns direct and queued requests, outgoing packets,
original deadlines, reply validation, pagination and the accepted UI model.
`AudioClient` retains QObject/QML presentation and the credentialed nonblocking
SEQPACKET edge. The lifecycle owner continues to own permission and provider
policy. No provider client, lease, worker, retry budget or new authority is added.

The13-function client ABI replaces the obsolete queue ABI. Safe modules reuse the
existing internal Queue and BootClock; the narrow FFI copies bounded inputs and
outputs without retaining caller pointers. One event thread owns each handle.
Connection epochs and send tokens reject stale callbacks. EAGAIN/EINTR preserve
an unsent packet and its deadline; a complete actual-send report consumes its
intent once. A queued route's floor-rounded remaining TTL is never rounded up.
Initial connect and every status page share one deadline. A timely reconnect-ok
starts a separate read-only status request; late replies cannot do so.

Reply validation preserves required nullable keys, exact normalized object shapes,
last-value duplicate keys and exact integral numeric offsets such as8,8.0,8e0.
Labels retain their text and UTF8 byte limit. One explicit authored policy rejects
unpaired surrogate escapes; current Qt accepts those as a question mark, while
valid Rust actor strings cannot contain them. This is a deliberate valid-Unicode
policy, not a claimed baseline bug. A finite26-case actual current-Qt socket probe
records the selected edges; exhaustive Qt parser equivalence is unclaimed.

The visible catalog and permission/state facts change only on the final valid
page. Denied/unavailable clear applied and permissions while retaining the last
requested/catalog. Malformed reply/disconnect clears the model and pending work.
There is no automatic reconnect/replay. All send/reply/prepare paths check real
BOOTTIME independently of timer delivery. Pure copied views do not dispatch,
expire a request or renew a deadline.

## Actual component evidence

The genuine current QObject header control compiles with zero diagnostics before
implementation. Its independent missing-API target then fails only on the absent
client type/create function. After implementation both original translation units
compile cleanly. This is a structural RED→GREEN, without an old semantic-bug claim.

The independently frozen six-group C++/Rust consumer passes direct model behavior,
atomic pagination/wire semantics, copied send fences, final fresh queued-intent
validation, connection/token cancellation and real expiry without timer delivery.
Its source and proposed header remain byte-identical from the preimplementation
freeze. Every assertion in the historical queue consumer is mapped to the new
boundary; no old production ABI remains for compatibility.

The unchanged30 Qt behaviors plus generated setup/cleanup32 pass against the real
Linux socket/credential fixtures. The existing29 appearance cases also pass in a
separate offline test namespace with only SETUID/SETGID added for actual UID/GID
negative peers. The source fixtures, original time limits and assertions stay
unchanged. Rust formatting, strict all-target Clippy and Release pass. The Rust
library's unit-test target contains zero cases; the independent C ABI consumer is
the functional logic test rather than a mirrored implementation test.

Default offline Cargo/CMake builds and links the full Release Sounds application,
and CTest executes the same permanent six-group consumer with assertions active.
Its archive and full app are byte-identical to the explicit-provider build. Actual
symbols show13 strong client definitions and no obsolete queue functions; the
actual link command names one selected archive and `ldd -r` resolves it. The
header is a compiled source input; CMake does not claim to install an FFI header.

A final source review also removes redundant Qt notifications on successful
25ms watch ticks. Only the affected Qt/full-app and default CMake builds rerun;
the unchanged Rust archive, frozen C ABI, appearance and CTest proofs retain
their actual earlier source-bound executions.

All executed phases retain actual bounded parent waits/merged PIPE EOFs, exact
container absence on the same daemon and unchanged selected source inputs.
The read-only build-copy setup failure and first strict-Clippy diagnostic remain
preserved; subsequent epochs use separately selected corrected inputs.

These are host Linux CPU/C ABI, real socket and full-link results. The
[historical prepared GUI/PCM pass](native-sounds-rust-queue.md#actual-prepared-app-and-audio)
selects the earlier queue client. A fresh native prepared app/PCM epoch with this
whole engine is still required. No new ARM build, installed cold startup, native
display cleanup or physical phone result is inferred.

[Hash-only provenance](evidence/native-sounds-rust-client-provenance.json) records
the actual source, archive, frozen consumer and closed host evidence.
