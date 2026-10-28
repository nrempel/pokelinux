# Fixed address and packet fixture

These are selected runtime snapshots for explicit normal Rust fixed-profile
operations after confirmed open association. They are not a standalone VM installer.
The radio/AP/namespace owner is the enclosing association fixture; the address
worker receives no inferred radio ownership. The stationary UDP peer runs in the
separate retained AP namespace. Both UDP clients and peer have zero capabilities
after exec; only the trusted fixture provisioner configures the peer address.

The pure `address_consumer.py` is unchanged between actual baseline RED and the
new implementation. It checks actual worker completion, independent kernel
addresses/routes, two exact nonce ACKs, explicit removal, quiet and all enclosing
cleanup. It makes no I/O or action requests. Its SHA256 is
`efe8a25a1a030ea945b987ddc2563f21a350090c5bc756aaf671274bfcdb3fe5`.

`red-address_case.py`, `red-CONTRACT.json` and `arm/red-boot.py` preserve the actual
refusing baseline. `address_case.py`, `CONTRACT.json` and `arm/boot.py` select GREEN.
`arm/red-guest-main.py` is shared by the actual baseline and first failed GREEN
publication attempt. The later `arm/guest-main.py` adds actual inherited-limit /
effective-unit sampling and SIGTERM deferral around durable final publication.
It records that pending TERM may prevent natural guest exit 0. It changes no
production or functional-oracle bytes. UDP client and peer remain unchanged.

GREEN also adds a read-only wait for peer publication of the same two nonces
after ACK2, under the same original six-second cutoff. This is a static fixture
race correction identified before execution, not an observed behavior RED.
The whole runtime is therefore not claimed byte-identical.

Staging reuses `../split-namespace/consumer.py` as the association consumer,
`../disconnect_consumer.py`, `../arm/guest.py`, `tools/native-connectivity-control/peer.py`
as the namespace keeper, `tools/native-wireless-scan/arm/{arm_tools,worker_caps}.py`
and `tools/native-installed-services/owned.py`. See the
[report](../../../docs/native-wireless-address-packets.md) and hash-only evidence
for the original source, normal ELF, selection and actual closure bindings.

Keep the original action/case/outer cutoffs. A caller-retained Scope fences L2
Disconnect and radio removal while any address worker or peer lacks actual wait
and stdout EOF. No uncertain outcome admits an automatic retry or renewed deadline.
