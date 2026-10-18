# Actual Settings QML connectivity boundary

The normal CMake target `settings-connectivity-click-test` compiles
[`connectivity-click.cpp`](../../native-settings-ui/tests/connectivity-click.cpp)
with assertions, Qt Test and four real-call linker wrappers. It is an unshown
component. Its actual mouse press/release activates the normal QML Enable button;
it does not invoke the setter directly. [`case.py`](case.py) is the frozen
root-authored Unix peer and post-custody assertion driver.

Build the normal `native-settings-ui` CMake project with three matching pinned
Rust archives as documented in its README. Build `pokelinux-settings`,
`settings-client-component-test`, `settings-network-card-unavailable-test` and
`settings-connectivity-click-test`. The last target needs Qt Test development
files; the normal application does not link Qt Test. No standalone CTest entry
starts a privileged peer or changes a host network.

Run only in a disposable, network-disabled Linux environment. The prepared
caller provides the existing `owned.py` Scope, `closure.py`, and the capability
helper from `tools/native-connectivity-control/peer.py` on its module path. It
calls `run_case(scope, output, runtime, executable, theme, work_until_ns,
close_until_ns, expect_red)` with original absolute BOOTTIME cutoffs. Runtime
contains a protected0755 executable/theme directory and a separate sticky user
parent. Its `/runner/inside.py qt-child` wrapper first selects actual UID/GID11050,
empty groups/all capability sets and NoNewPrivs, creates its own0700 HOME/XDG
children, then execs the component in the same PID. The caller/root peer retains
only SETUID/SETGID/KILL after setup. Keep runtime execution enabled and source/
archives read-only. These are authored test facts, not an OS NetworkControl grant.

The peer serves brightness unavailable on its separate connection and fresh
connectivity facts on one retained socket. It records the exact original click
deadline and rejects additional mutation packets. Connectivity socket reset handling retains
the same FD through actual zero-byte EOF; late readonly reply failure is recorded
separately, while a failed Set reply cannot qualify success. `expect_red` selects
the deliberately missing-effect baseline; the current implementation uses false.
It changes no packet or confirmation predicate.

The component prints observations and exits normally after actual asynchronous
Stop/Join/Free. Only after the parent obtains actual child wait/PIPE EOF and
closes preceding handles does the same compiled consumer run `--assert-result`
against the proof. The final assertion distinguishes a healthy click with no Set
from exact Set/deadline plus fresh Ready state in the actual normal QML. Wrappers
call the real Rust functions; readonly configured facts with Uncertain do not
satisfy confirmation. All original clocks, output caps and custody gates remain.

The qualified cohort is native Harbour x86_64 Qt6.8.2 with a normal Release Rust
archive. It supplies no normal app-main runtime/window, screenshot, native output
profile, fresh theme-provider, actor/kernel packets, installed deployment, ARM or
phone result. See the [report](../../docs/native-settings-connectivity-qt.md) and
[hash-only provenance](../../docs/evidence/native-settings-connectivity-qt-provenance.json).
