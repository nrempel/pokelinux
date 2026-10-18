# Fixed IPv4 virtual-backend packet fixture

These are the original authored controller and independent Python peer used by
[the host Rust packet proof](../../docs/native-connectivity-packets.md). They are
byte-for-byte the measured RED/GREEN runtime inputs. Their “private” comments
refer to disposable namespace/test preparation; no proprietary input, raw vendor
decompilation, binary, credential or captured packet is included here.

The Rust desired-behavior test is
`native-connectivity-control/tests/packet_lifecycle.rs`. Compile it with the
selected locked offline toolchain and obtain its exact `compiler-artifact`
executable from Cargo JSON; do not guess a wildcard target or substitute an old
binary. The measured run used Debug test ELFs, not normal Release services.

The controller requires a trusted fresh exclusively administered current network
namespace with loopback only, UID0, initial NET_ADMIN/SYS_ADMIN/SETPCAP and a
retained clean container/process supervisor. Set
`POKELINUX_CONNECTIVITY_ISOLATED=1`. Never run this against a phone or shared host
namespace. Loopback-only inspection is a sanity check, not proof of exclusivity.
The controller creates and retains the peer namespace itself.

In the prepared container, mount these files as `/runner/controller.py` and
`/runner/peer.py`, the existing `tools/native-installed-services/owned.py` as
`/runner/owned.py`, and `tools/native-audio-preview/closure.py` as
`/runner/closure.py`. Invoke the controller with a fresh absolute output directory:

```sh
python3 -B /runner/controller.py --test-elf /absolute/selected/test-elf \
  --output /absolute/fresh/output --work-until-ns ORIGINAL_WORK_BOOTTIME_NS \
  --close-until-ns ORIGINAL_CLOSE_BOOTTIME_NS
```

Those original absolute clocks come from the retaining outer supervisor. A
six-second functional case starts once; each original action is at most one
second, and the quiet phase has an original800ms cutoff including gate delivery.
Its actual armed-to-final-EAGAIN interval is recorded separately. The outer supervisor
must retain its exact container/CLI custody through actual wait, PIPE EOF and
removal. This directory is fixture source, not a standalone Docker build or an
installed connectivity service.

The controller reaps and closes the short-lived peer provisioner before starting
the capability-free UDP peer. It then drops all its own capabilities, leaving
Rust as the sole NET_ADMIN writer. Both namespace handles remain retained until
all owner/peer/keeper waits and output closure complete. A separate Python
GETADDR parser and exact random nonce/ACK exchanges check actual traffic. The
quiet oracle drains to real EAGAIN before publishing and checks for unexpected
datagrams after peer shutdown.

The initial Unsupported assertion is an expected behavioral RED only after
healthy setup and complete custody; it is never a passing Rust test. Positive
results qualify fixed local IPv4 and literal trusted owner-principal withdrawal,
not Settings windows, OS permission enforcement, Internet, Wi-Fi, ARM or phones.
