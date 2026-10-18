# Installed connectivity selection

The normal Rust session-owner configuration now requires an explicit connectivity
selection. `{"mode":"disabled"}` creates no provider;
`{"mode":"owner","socket":"/absolute/private-connectivity/control.sock"}`
selects the existing lifecycle connectivity provider. There is no implicit default
or earlier-schema fallback. The packaged example explicitly disables it.

Exactly one fixed `org.omarchy.Settings` profile at UID11050 is required when
brightness, connectivity or both select an owner. Both disabled refuses Settings.
Sounds and optional Messages retain their existing rules. Settings still takes
six absolute arguments and uses the lifecycle `app.sock`; the private owner
endpoint is not an app argument. No grant, backend, device or automatic mode is
accepted. Durable NetworkControl permission remains an independent decision.

The connectivity endpoint needs an existing root:root0700 parent with protected
ancestors, an absolute path without parent traversal, and a valid AF_UNIX length.
It must differ from audio, appearance, Wayland, lifecycle and selected brightness
endpoints. Validation neither connects nor creates deployment directories. Normal
runtime wiring constructs the existing provider once and passes it to the
existing connectivity-aware actor entry. The separate service must already be
prepared; this configuration does not start or replace it.

## Executed boundary

A disposable native Harbour x86_64 container exercises real `Config::load` and
root-protected filesystem metadata. The compiled RED runs two exact tests: all
five explicit selection combinations are rejected as an unknown field, while
omission actually passes validation and fails the desired refusal assertion.
Both tests fail behaviorally after successful compilation. Later negative checks
inside that second test are not reached in RED.

After the implementation, all31 selected filesystem tests pass:

| Target | Passed | Scope |
| --- | ---: | --- |
| connectivity_config |4| Five selection combinations; omission/undeclared fields; exact Settings role/UID/arguments; private, distinct endpoints |
| config |15| Existing installed roles, bindings, protected files, environment and policy-directory controls |
| messages_config |8| Existing Messages selection, identities and storage-directory controls |
| brightness_config |4| Existing brightness selection, role and endpoint controls |

The Messages Source/projector runtime test is explicitly filtered. Fixture files
that resemble executables are metadata inputs and are never executed. The root
fixture is empty before and after each suite. Test processes retain only CHOWN,
DAC_OVERRIDE, FOWNER and KILL; all five compiler/QA processes have zero capability
sets. NoNewPrivs is1 throughout.

Formatting, strict all-target Clippy, locked test compilation and the normal
feature-empty, opt3 x86_64 session-owner Release build all finish before the first
test process starts. The normal executable is exported, never launched. The first
GREEN attempt stops at two test-only Clippy warnings, before tests or Release.
Removing one needless borrow and one unused `mut` preserves every assertion; the
compiled RED consumer and corrected GREEN consumer have separate recorded hashes.
The earlier rustfmt transformation is also retained rather than described as
byte-identical source.

The completed GREEN retains22 actual owned waits and stdout EOFs, plus nine
separately closed regular stderr sinks. All original cutoffs hold, no adopted
child waits occur, ECHILD is confirmed, the exact container is removed and the
same Docker daemon is verified. All223 compiled source files match the published
implementation; all456 pinned inputs remain unchanged. See
[hash-only provenance](evidence/native-connectivity-installed-selection-provenance.json).

## Remaining boundary

This is filesystem admission and compilation evidence. No provider connection,
normal session-owner Main, Source, compositor, app, kernel-network operation,
installed boot, ARM execution or phone runs in this cohort. Earlier actor/packet,
safe SDK, C ABI and unshown Qt results remain separate.

Next join the normal Settings window, existing per-app display ownership and
upstream compositor to the actor/Main packet oracle, using explicitly authored
session/admin authority. Enable must produce packets; normal app close must keep
them flowing; a fresh launch must explicitly Disable and prove removal and quiet.
Full Source/authentication/session deployment remains a separate integration gate.
Wi-Fi/Bluetooth/Internet, suspend, matched Pixel hardware and all phone gates stay
open. No physical phone was accessed; zero models are hardware-qualified.
