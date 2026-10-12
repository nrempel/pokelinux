# Upstream dependencies

Keep our source, tests, small patches and dependency manifests in Git. Fetch upstream source into ignored build directories. Do not commit copied upstream trees, SDKs, factory binaries or VM images.

## Fetch pinned source

```sh
python3 tools/fetch_upstreams.py
# Also fetch the selected Omarchy shell, theme and command sources:
python3 tools/fetch_upstreams.py --omarchy
```

`sources.lock.json` pins libhybris and Droidian Android headers by commit and archive SHA-256. `dependencies/omarchy-files.lock.json` pins the selected Omarchy files by commit, Git blob hash and file mode, including its license. The fetcher verifies existing cached files as well as downloads. A mismatch stops the operation; it does not silently replace a changed source.

The graphics builder applies our patches to the pinned libhybris tree. Preserve upstream license notices in fetched trees and any redistributed builds. This repository does not relicense dependencies. Rust crates use the checked-in Cargo.lock files; builds use `--locked`. OS package repositories remain rolling where the existing runners use them: these locks do not yet make the complete OS image reproducible.

## Why fetch scripts

A manifest and fetch scripts keep dependency identity explicit without copying upstream history into this repository. Use a submodule only if future work requires direct development in a maintained upstream checkout. Subtrees are not the default. Submit reusable fixes upstream where practical, then remove the local patch when the pinned upstream revision contains it.

## Inputs not supplied here

Google factory images, proprietary APKs, PowerVR libraries, modem/camera firmware and extracted vendor files remain local inputs subject to their original terms. Download them from their official source separately; this project does not grant redistribution rights or accept terms on behalf of contributors. The Android SDK and emulator image must also be provisioned separately.

The original-code Shannon oracle requires the exact APK hash recorded in `docs/evidence/codec-provenance.json`. The public Rust fixtures permit codec comparison without shipping that APK. Running fixture comparisons does not rerun the original implementation or test a modem.

Large artifacts and enrollment credentials belong in ignored directories. Never add them with `git add -f`.

## Verification checkpoint

On 12 October 2026, the fetcher downloaded both pinned graphics source archives
into an empty temporary directory and verified their SHA-256 values. It also
verified all 196 locally cached selected Omarchy files against the blob manifest.
Four tests cover accepted downloads, rejected checksums, corrupt cache handling
and Git blob verification. This does not claim a clean-machine full OS build.

Native messaging pins CC0 `sms-pdu` 1.1.0 for GSM alphabet mapping.
`native-messaging/Cargo.lock` pins its transitive dependencies; the crate archive
hash and actual AOSP comparison provenance are in
`evidence/sms-submit-provenance.json`. The private Android framework JAR is not
distributed. Public synthetic fixtures can be compared independently of it.
