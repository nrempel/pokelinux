# Original SIT context memory application

A fresh original-code run composes the complete144-byte setup-response parser
with `PdpContext::UpdateDataCallInfo` under genuine Bionic ARM64 user-QEMU.
One load control and seven context cases complete14 actual update calls.
This is additional original integration: native Rust behavior is unchanged,
no new RED is claimed, and the earlier22 native codec tests were not rerun.
Root and independent review qualify this bounded original run.

The authored helper uses actual constructors without field patches. It passes
the parser's DataCall directly to an original context whose C1 state is0 and
whose APN, traffic-descriptor and controller pointers are null. Each case calls
the updater twice with the same data, or twice with null in that control.

| First update | Return bit | Complete352-byte fixed prefix |
| --- | --- | --- |
| Matching dual-family CID9, constructor index0 or17 |1 | Copied |
| Matching inactive observation |1 | Copied |
| Matching PDP failure status65535 |1 | Copied |
| Matching Non-IP observation |0 | Copied |
| Mismatched CID |0 | Unchanged |
| Null input |0 | Unchanged |

The bit records the measured address-change result. It is not a general
copy-success or readiness flag: Non-IP copies with bit0, while both refusal
controls leave the prefix unchanged. Copying PDP failure65535 does not accept
that failure as success; the [native decoder](native-sit-codec.md#base144-byte-response-decoder)
continues to refuse nonzero status. This measurement covers the complete fixed
prefix, not every dynamic member of the full original DataCall.

All seven identical second calls return0. Actual full-prefix `memcmp` checks
report equality with the first result and unchanged source bytes; no second
raw prefix array was emitted or separately hashed. Context state0 and all
three null pointers remain unchanged. Index0/17 changes the local interface
name without establishing channel or slot binding.

Original context D1, its guard check and outer free run first; adapter D1 and
packet D1 then precede their guard checks and outer frees. All eight owned QEMU children have actual
wait0 and stdout EOF within their original CLOCK_MONOTONIC cutoffs. Outer CLI
wait0, exact container/fresh absence, same daemon and unchanged inputs close;
no separate outer EOF or suspend qualification is claimed. Selected original
files and mapped identities are checked; ambient failed UNIX/property attempts
match the load control. Raw prefixes, proprietary inputs and dumps stay private.
[Evidence](evidence/native-sit-context-application-provenance.json).

The service handler and its two test-hook gates were not executed. Normal
returned-CID/requested-context selection, populated QoS/TD and state3 behavior,
owned completion, activation, APN ownership, channel/netdev/provider identity,
deactivation/reset, real kernel/modem/carrier and physical hardware remain open.
No OnActivated, NetIf or transport call is made, and no phone was accessed.
