# Fixed helper-profile containment control

The actual uncontained RED proves that root-owned cgroup controls do not contain
an original-helper privilege profile: the authored probe moves only itself into
an explicitly owned sibling, then survives killing the empty workload. The
separately selected contained ARM control passes after a private mount view.
See [report](../../docs/native-auth-containment.md) and
[bounded evidence](../../docs/evidence/native-auth-containment-provenance.json).
Neither run authenticates or executes original PAM/unix_chkpwd. No phone runs.

`fixture/` contains authored Rust, with no PAM implementation linked. Default
build is the uncontained probe. `--features contained` compiles the launcher
against the exact `native-pam/src/containment.rs` module. This is an isolated
fixed control, with no selectable program/account/root/policy. The unchanged
`entry-copy.rs` pins our existing native PAM credential transition, SHA256
`ec8cfae7f0231e9e7d6b44702eef9794c5261adae3a697bf89fb8a4d531d9962`.
The copied `preparation.json` describes the historical initial compile selection;
its source hashes and UNRUN label do not describe the later changed fixture.
Current source/build/runtime bindings are in the bounded evidence above.

The outside driver retains the exact gated child/pidfd before admission to a
private workload, then independently observes its actual executable inode, IDs,
groups, capability sets, NNP, CORE limit, pipe and membership. The probe executes
a fixed root-owned06755 file, gaining UID/GID `[1000,0,0,0]`, audit-only E/P/Bnd,
I/A0 and NNP0. It writes only `0\n` to the owned sibling's fixed cgroup.procs;
this names itself. In the contained build that open returns ENOENT. The contained
probe uses direct kernel readbacks; it cannot read full proc status/membership.
Those observations remain the outside driver's responsibility.

The one nonblocking16-byte packet is `PKHCPR01`, outcome:u8, stage:u8,
errno-class:u8, reserved0:u8, flags:u32le. The uncontained complete-move packet is
4/10/0/0/255. The contained direct-open-denied packet is1/10/5/0/135 (class5 is
ENOENT); membership/scope/write flags are absent. Thus changed preconditions are
explicit, not a false claim that the complete original probe ran unchanged.
Any partial packet, failed profile or fallback cleanup fails the contained case.

`red/` and `contained/` preserve the exact authored guest/owner/runner sources for
the two selections. These scripts are qualification infrastructure for the
explicitly owned disposable Arch ARM7.1.6 guest, never host/device installers.
They require the selected immutable manifest, read-only public ARM code inputs
and backing image, fixed absent label, original15s BOOTTIME attempt and bounded
outer process/container ownership. No credentials or account changes are used.
Do not invoke the guest/delegated scripts on Harbour itself.

Build from the repository root with the existing offline registry and ARM linker:

```sh
cargo fmt --manifest-path tools/native-auth-helper-profile/fixture/Cargo.toml --check
cargo clippy --locked --offline --manifest-path tools/native-auth-helper-profile/fixture/Cargo.toml --all-targets -- -D warnings
cargo clippy --locked --offline --manifest-path tools/native-auth-helper-profile/fixture/Cargo.toml --all-targets --features contained -- -D warnings
cargo build --locked --offline --manifest-path tools/native-auth-helper-profile/fixture/Cargo.toml --features contained --release --bins --target aarch64-unknown-linux-gnu
```

The helper fixture root has only three required public runtime libraries, fixed
NSS files and selected devices. It is not the original PAM dependency closure.
All alias/direct-process routes, original PAM/tally behavior, native scope
composition, installed broker/actor restoration, matched Google and hardware
qualification remain separate gates. This is not a general hostile-code sandbox.
