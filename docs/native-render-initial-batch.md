# Initial broker batch capture

The control broker now exposes one const `peekInitialRenderEvents` operation.
It copies a complete INIT → DEVICE_ADD* → ENUMERATION_DONE prefix without
advancing the queue, duplicating or closing descriptors, invoking callbacks or
doing native work. Rust can capture its own descriptor duplicates before control
marks those exact queue entries admitted. This fills one startup handoff gap;
main-to-worker backend startup remains unfinished.

Owner/role and terminal checks precede queue access. The prefix is bounded to
32 devices/34 events, with contiguous initial sequences, matching generation,
bounded metadata and unique device leases/rdevs. Every refusal preserves caller
storage/count bytes, queue state and descriptors. Success copies values and
borrowed descriptor integers only. No old single-event fallback is supplied.

## Actual structural RED and source build

The frozen [consumer](../tools/native-render-owner/initial-batch-consumer.cpp)
first fails compilation against unchanged SDK ec484015 after a successful strict
existing-API compile/link/loader/run control. Both missing enum/member diagnostics
are present; all31 diagnostics concern those absent types/API and their cascades.
The [executed classifier](../tools/native-render-owner/initial-batch-compile-oracle.py)
was frozen before implementation. Three prior aggregates remain failed: SDK link
creation order, unsupported compiler diagnostic format, then an incomplete
missing-type cascade recognizer. The consumer and strict warning flags remain
unchanged. The last fixture recognizes only exact source-bound cascades; it
suppresses no compiler warning.

Only after that retained RED was the three-file
[incremental implementation](../tools/native-render-owner/aquamarine-initial-batch.patch)
authored. It applies after the existing AQ15 source patch; its
[cohort](../tools/native-render-owner/aquamarine-initial-batch-cohort.json) preserves
all historical inputs separately. A native x86 KVM build compiles all six affected
C++ units and reruns the ten existing CPU source-component groups. Library
323e1cb3 and SDK5a89a24c match the successor headers/source. The SDK contains22
regular files and two exact library links. These CPU groups do not exercise the
new real broker API.

The RED retains33 direct waits/40 PIPE EOFs; the source build retains46/46.
Original cutoffs, readonly input locks, KVM/QMP shutdown, same-socket EOF,
QEMU/foreground CLI0 and exact container absence remain qualified.
[Hash-only evidence](evidence/native-render-initial-batch-provenance.json) records
these separate scopes; no proprietary or raw private fixture input is supplied.

## Actual real-broker boundary

The unchanged [runtime consumer and contract](../tools/native-render-owner/initial-batch-runtime-contract.md)
now pass in a separate native x86 KVM guest using real seatd/libseat, udev/libinput
and virtual virtio primary/render descriptors. One actual graphics lease produces
three initial events. Repeated peek and wrong-owner/capacity poison-output checks
pass; exact fieldwise Rust capture succeeds before any queue mark. Both pending
descriptor duplicates then close in order while the original service descriptors
remain open. Initial-prefix, drained-queue and terminal refusals pass.

A deliberate public transport-failure request after the positive capture/mark
boundary checks the truthful preworker UNAVAILABLE/terminal latch. It is not an
actual seat-disable test. No executor, render mirror, output registry or graphics
retirement aggregate is created. Broker destruction is forced service cleanup,
without a service-retirement claim. The
[executed runtime classifier](../tools/native-render-owner/initial-batch-runtime-oracle.py)
requires all four actual consumer records and owned child/EOF closure.

C++ builds force-include the declaration-only
[libseat C-linkage header](../tools/native-render-owner/initial-batch-c-headers.hpp)
with `-include`: libseat.h itself lacks the needed C++ linkage guard. The first
runtime attempt failed at that link boundary before executing the consumer; it
remains failed. No function wrapper or interposer is supplied. The C++ consumer,
production implementation, assertions and5/8/20-second work/close/outer bounds
remain unchanged. Baseline and consumer compile/link diagnostics are empty;
actual RUNPATH and `ldd -r` select the matching private AQ15 library, and each link
contains one current Rust246 archive.
The runtime retains39 direct waits/46 PIPE EOFs within the original cutoffs,
with the same KVM/QMP/container/input closure requirements.

Main worker startup, real output discovery/retirement, matched executor
GL/Stop/finalize, Source7, normal app/PCM, ARM, Google-kernel and physical-phone
qualification remain open. Process reaping does not establish native device
cleanup. No phone was accessed.
