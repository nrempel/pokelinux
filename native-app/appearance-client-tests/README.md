# Current native appearance client boundary

`appearance-socket-probe CASE ASSET_DIRECTORY` uses QCoreApplication and actual
forked Linux seqpacket peers/credentials. Run it as root only inside a disposable
owned test container; selected peer controls use UID1000. The asset directory
contains the matching `PhoneAppearance.qml`. No window or GUI preview is created.

The recorded finite29-case selection is publication, expired, cached-expiry,
malformed, unavailable, peer-uid, restart, disconnect, until-syntax,
revision-float, revision-rollback, owner, rights, held, oversized, sender-pid,
sender-uid, delayed-consumption, late-real-receive, qml-full-style, schema1,
schema2, rgba16-exact, rgba16-alpha-only-new-revision,
rgba16-alpha-only-same-revision, rgba16-one-unit-new-revision,
rgba16-final-liveness, style-same-revision and style-new-revision.

The unchanged29-case fixture now exercises the thin Qt native adapter backed by
the app-local Rust policy engine. Qt keeps socket/credential/descriptor/notifier
work and QColor/QML projection; Rust owns schema/cache/generation/readiness and
deadline decisions. Assertions and cutoffs remain unchanged. The selected cases
reject old schemas; there is no fallback.
Retain each actual probe/peer wait and applicable EOF, enforce the original
cutoffs, and preserve failures. The test target alone does not qualify the real
Rust owner, a rendered app, an ARM runtime or hardware. The separate installed-QML
probe loads only the two-file install tree, without the source tree or JavaScript.

See the [current Rust-client qualification](../../docs/native-appearance-rust-client.md)
and the [earlier reuse checkpoint](../../docs/native-audio-appearance-client.md).
CMake selects one appearance engine. Explicit/cross builds supply both
`POKELINUX_APPEARANCE_CLIENT_ARCHIVE` and its `_SHA256`; native default builds use
locked/offline Cargo. The separate frozen eight-group C ABI consumer uses authored
native-result inputs and does not replace these actual credentialed peers.
