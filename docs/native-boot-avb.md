# Native boot candidate and the retained AVB contract

2026-10-14UTC: unchanged original AOSP AVB tools qualify a partial stock metadata
graph and refuse the current unsigned native `init_boot` candidate. These are
offline original-code results on authenticated selected files, without bootloader
or phone execution. See [bounded provenance](evidence/native-boot-avb-provenance.json).

The original `avbtool.py` is pinned to commit
`761178607206f4cb2af79ed9eec52d8cbd814adb`,207,502bytes, SHA256
`e5a664a38db623da00f080219bc0ee60a640a9dc4a872803616fae4938ac749b`.
Twenty-six original CLI invocations and one authored wrapper calling unchanged
original parsing/signature/descriptor APIs inspect ten authenticated factory
images and the exact current candidate. Python3.12.3, OpenSSL3.0.13 and source/input
identities are retained privately. No key, footer or image is modified.

## Qualified stock metadata

Five SHA256_RSA4096 metadata blocks are internally signature-consistent:
`vbmeta`, `boot`, `init_boot`, `vbmeta_system`, `vbmeta_vendor`. The five selected
local metadata blocks in `vendor_boot`, `vendor_kernel_boot`, `dtbo`, `system_dlkm`
and `vendor_dlkm` use algorithmNONE. Their local hash/tree verification is not a
signature. The original signature helper returnsTrue for NONE; the proof keeps
these rows explicitly unsigned.

The root metadata declares these four child keys and rollback locations. Actual
embedded child keys match byte-for-byte, and separate original expected-key/slot
checks pass:

| Parent | Child | Declared rollback location | Child key |
| --- | --- | ---: | --- |
| vbmeta | boot | 2 | K1 |
| vbmeta | init_boot | 4 | K1 |
| vbmeta | vbmeta_system | 1 | K2 |
| vbmeta | vbmeta_vendor | 3 | K1 |

K1 is SHA256`757c626a2a91fe852536546048d7ca3f50df6c745c026db9ff89cc4703c59481`;
K2 is`c3cdf0a539c33c0555ab4c261aabdeb9d648dd90cb51c4c237018b3846e372bf`.
These fingerprints cover1,032-byte AVB-format public-key blobs. They do not
establish Google's external device trust anchor. All five signed headers have
rollback index1791158400, header rollback-location0 and flags0; the parent-declared
locations above are separate metadata. No stored rollback state was read.

The original APIs pass16 available descriptor checks:4 chain,8 hash,4 hashtree.
Some targets have both parent and local coverage, so this is not16 independently
qualified partitions. Another27 payload checks remain unavailable:22 under root,
4 under system and1 under vendor. Original hashtree checks do not independently
qualify FEC, module signatures or KMI. No extra unrelated firmware was extracted.
Complete transitive stock verification remains unproven.

The original follow-chain mode skips expected-key/slot comparisons when no
expected data is supplied. This investigation therefore separately executes
the actual declared child-key/slot comparisons. Parent CLI failures caused by
missing expected data or unselected payloads stay failed; signature consistency
does not turn those aggregates green.

## Actual candidate refusal

The compact native candidate is1,462,272bytes, SHA256
`b2ba46ec4eaf8c4398ae7eb6033fed179c1cc030f4bdbcb753f30717738359a8`.
Both original `info_image` and `verify_image` return1 because it has no parseable
VBMeta. This is an absent valid metadata/footer result, not a failed RSA comparison.

Stock `init_boot` is8,388,608bytes. Its child VBMeta starts at2,691,072 and is
2,432bytes; its salted SHA256 descriptor covers the first2,691,072bytes. That
original descriptor verifies the stock file and refuses the exact candidate in
a separate read-only view. The candidate has changed bytes and is shorter than
the declared protected range. No full-size partition-write, erase/padding or
residual-storage behavior is established.

An original root CLI using actual expected child keys and chain traversal
accepts the stock root/boot and init_boot key/slot, then refuses the candidate
because its child VBMeta cannot be parsed. No stock footer was copied onto it.
Factory archive SHA256 authentication, salted payload coverage, metadata trust
and physical partition geometry remain distinct facts.

## Supervision and admission limits

All27 outer processes were actually waited. Retained process traces contain27
matching Python terminal exits and42 nested OpenSSL executions that exit0 and
are reaped by actual positive parent wait4 results. Eighteen invocation exits
are0 and nine are1; nonzero refusals and incomplete-chain results stay nonzero.
Root independently rehashes26 exact input identities, compares the original API
report with the bounded metadata and checks every retained process trace/wait.
This is review of actual original execution, without a new crypto implementation,
independent pidfd proof or postwait `/proc` absence claim.

The concrete gate is that this unsigned compact artifact cannot satisfy the
retained stock signed init_boot contract at delegated rollback location4.
Embedded-key self-consistency establishes no external trusted key, device rollback
acceptance or locked/unlocked error policy. A future authorized experiment needs
an independently established complete-image or temporary-load contract and actual
loader selection. This review makes no signing or verification-policy changes
and supplies no flash/unlock commands.

The [Google-kernel/QEMU RAM execution](native-boot-google-kernel.md) establishes
none of those loader properties. Early recoverable diagnostics, reset, vendor
fragment/DTBO selection, module admission and all physical phone gates remain
open. Full smartphone deployment stays refused. Raw originals, keys, decompilation,
logs, traces and private audit records remain private.

References: [exact original avbtool](https://android.googlesource.com/platform/external/avb/+/761178607206f4cb2af79ed9eec52d8cbd814adb/avbtool.py),
[AVB trust and rollback contract](https://android.googlesource.com/platform/external/avb/+/761178607206f4cb2af79ed9eec52d8cbd814adb/README.md).
