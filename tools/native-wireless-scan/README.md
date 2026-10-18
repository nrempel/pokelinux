# Wireless discovery test sources

`consumer.py` is the unchanged desired-result consumer used for the original
native Rust refusing RED and its completing successor. It performs no I/O or
clock operations: the actual process owner supplies the original deadline,
canonical wait/EOF receipt and independently established AP/interface facts.
The caller persists those observations, finishes owned cleanup, then qualifies
and asserts desired behavior once. An unconfirmed outcome is not a RED.

`refusing/` is the exact original Rust baseline source and generated lock used
for the normal ARM Release worker. It always reports explicit unsupported
without opening a socket. It is test infrastructure, not a runtime fallback or
compatibility path. No executable is published here.

The [report](../../docs/native-wireless-scan.md) keeps original tools, component
QA, generic ARM execution and phone gates separate. Private VM disks, package
archives, runtime receipts and credentials are not test-source inputs in Git.

`arm/` preserves the exact native-epoch guest/bootstrap snapshots. The same guest
and frozen consumer execute in RED and GREEN; only the host/boot expectations
and selected worker/build proof change. They expect the private sealed selection,
trusted prepared image chain and staged public package inputs described in the
report. They are execution snapshots, not a phone installer or a standalone VM
builder. The shared `owned.py` comes from the existing installed-service tool.
No private selection, runtime receipt, package archive, VM disk or executable is
published here.

The `unavailable-*` snapshots select an independently observed non-wireless
loopback in a fresh guest and execute the exact same completing binary. No
AP, radio creation or package installation occurs. Module inventory changes are
recorded, not interpreted as zero kernel effects. This control has its own
original deadline, response validation and complete process custody.
