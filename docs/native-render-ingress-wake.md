# Retained ingress retry notification

The experimental Rust render executor could return `Busy` while the worker held
its ingress mutex, then release that mutex without notifying control. If the
queue was empty, no later completion or timer event was guaranteed: accepted
startup disarms its timer. A retained frame could miss its unchanged cutoff.

The repair notifies the existing completion FD immediately after releasing that
mutex, including an empty pop. It adds no API, FD, credit, timer or retry queue.
Control may observe Empty when draining the completion API and then reoffer its
retained frame with the same absolute cutoff. Notification failure uses the
existing terminal/revocation path; a popped job retains its normal cleanup path.

The frozen test holds the actual worker mutex after Ready, drains old
notifications, calls the real public Rust admission method and observes `Busy`
with the same frame and cutoff. RED compiled and exited101: no notification
arrived before the original1s bound. Actual Stop, worker join and owner destruction
completed before the original8s close bound. After the five-line repair, the exact
same test passed before formatting or assertion changes, observing one new
notification and exactly one admission. The scheduling hook is test-only.

A separate regression releases the actual lock after the original cutoff and
observes `Expired`, retaining the same frame with zero admissions/credits. It is
additional passing coverage, not part of the original RED. Full Harbour-native
Linux x86 QA passes66 active tests (the original64 plus these two), two ignored
child modes, formatting, strict all-target Clippy and release compilation.

The first full-QA result finalizer rejected two authentic Cargo hardlinks after
all commands had exited0. That failed result remains preserved. A separate
read-only review authenticated those exact internal link targets, source hashes,
all test names and closed process receipts without rerunning tests. The exact-test,
formatting and full-QA phases retain16 direct waits/merged PIPE EOFs; five embedded
child waits/EOFs belong separately to existing tests.

This is real Rust thread/eventfd/admission behavior with an authored CPU native
owner. It proves no Wayland frame, GPU operation, vendor cleanup, ARM or phone
behavior. The new release archive has not been selected or C++-qualified into the
older normal compositor epochs. The [graphics critical-path decision](runtime-language-decision.md#graphics-critical-path)
keeps this executor as component research while the next app tracer uses normal
upstream Hyprland under the Rust session owner. [Hash-only evidence](evidence/native-render-ingress-wake-provenance.json)
retains the chronological scopes without raw private logs.
