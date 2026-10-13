# Public repository publication audit

A read-only independent audit pinned public main at
`ac57dd6e038c36e9278e07ba1b90b0084a4a5c24`: one branch, no tags,62 reachable
commits,1276 unique blobs and910 historical paths. GitHub release, issue/PR,
comment, wiki, discussion and Actions attachment surfaces were checked; none had
published assets. Mirror object integrity/reachability also passed.

No validated credentials, proprietary input binaries, raw decompilation dumps or
private audit logs were found. The original source/fixture/provenance review and
Gitleaks8.30.1 history/current-tree scans found11 candidates, all validated as nine
public Git blob IDs and two hashes of ignored test logs. Three documented PNG
assets have no personal UI content; no APK, ELF, firmware, module, VM, database,
packet capture, archive, LFS input or embedded key/binary was found in history.
This is an exposure audit of the pinned state, not a claim about future commits
or private ignored material, and not a general code-vulnerability assessment.

The current-tree cleanup removes15 tracked AppleDouble sidecars and ignores
future `._*` files, normalizes four provenance artifact labels to repository-relative
paths without changing test/executable hashes, and removes three private ChatGPT
conversation URLs while preserving advisory review status and public evidence.
All untracked/private copies and exact prior reports remain private and untouched.

Historical commits retain the low-severity metadata, generic workspace labels and
private conversation identifiers. The shared163-byte AppleDouble payload contains
FinderInfo, an empty resource fork and an opaque provenance attribute, with no
path, private-key or provider-token pattern. No credential exposure was found.
The cleanup preserves published commit/evidence history. A new immutable audit
must be performed when published inputs or artifacts change.
