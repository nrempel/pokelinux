# Native x86 session runtime

The native CPU execution environment on Harbour now passes one actual offline
KVM boot. The production Rust payload also compiles for x86_64 from the current
public source. A later native Qt build and current component checks also pass;
the full composition fails during original compositor startup before Sounds/PCM.
No Source availability or phone hardware qualification follows.
[Hash-only evidence](evidence/native-session-x86-runtime-provenance.json) binds the
inputs, builds, execution and qualifications. Private logs and VM artifacts stay
private.

## Runtime and authenticated guest

The existing graphics runtime is actually Debian13 amd64; its historical image
name does not establish an Arch or Hyprland installation. A derived image adds
matching-family QEMU x86, SeaBIOS, ipxe-qemu and genisoimage. Actual package
inventories and tool reports bind QEMU `1:10.0.13+ds-0+deb13u1`; the original
runtime and its renderer dependencies remain intact. An earlier Dockerfile
setup failure remains failed and is not a production RED.

The official `Arch-Linux-x86_64-cloudimg-20261015.611680.qcow2` is576,680,960 bytes.
Both its image signature and signed checksum were verified with the Arch CI key
published in the [pinned arch-boxes README](https://github.com/archlinux/arch-boxes/blob/7f733af26fe9a0c93fdabba13e7803cbe803374a/README.md).
The original is read-only, has no external backing/data file, and retains its2GiB
virtual disk. Only a fresh disposable child overlay receives writes. This is an
x86 Arch kernel, not Google's matched Pixel kernel.

## Actual boot and closure

A separate isolated API query first returns KVM API12. The actual VM then uses
explicit `-accel kvm -cpu host`, with no TCG fallback, no network and only the
host `/dev/kvm` device admitted to its container. Stock BIOS/GRUB, kernel,
initramfs and systemd discover a local NoCloud seed. No kernel extraction,
custom PID1, login/password or package installation runs.

The private bootstrap runtime-masks the offline time-wait service. This is an
explicit fixture change, not an unchanged boot-policy claim or installed phone
policy. The stock no-NIC network-online wait still consumes about120 seconds.
Actual cloud-init26.2 reaches the root bootstrap, which observes Linux
`7.2.9-arch1-1`, x86_64 and UID0. Its final receipt is flushed before it returns;
normal cloud-init power_state then powers off. The guest does not claim a
separately owned shutdown-child wait.

The same authenticated QMP socket identifies the owned QEMU process, confirms
KVM enabled/present, and later records `SHUTDOWN` with guest true and reason
`guest-shutdown`, followed by socket EOF and actual QEMU wait0. Serial identity
and the guest-result hash match the retained file. Seven host CLI waits and14
PIPE EOFs, four boot children, five guest commands and one early command all
close within their original cutoffs. The two CLI exit1 receipts establish exact
name/CID absence; they are expected. QEMU and Docker CLI return0, the same daemon
reports the exact container absent, and selected input hashes remain unchanged
under the shared lock. The complete host envelope takes132.46 seconds against
its240-second work/270-second closure limits; QEMU retains180/210 seconds.

## Matching native Rust payload

All83 selected production Cargo/source/lock files match public commit
`387fe72bc375def6dacd79fae8df2758f537e58a`; the four Sounds exec-fixture inputs
are separately bound. Strict formatting, selected-bin Clippy and native builds
pass for appearance, audio and sounds-exec. Packaging renames
`pokelinux-appearance` to `native-appearance` without changing bytes. The existing
host session owner and fixed Sounds client are separately hash-verified and
bound to that matching source. All five selected binaries are ELF machine62.
Ten CLI waits0 and20 PIPE EOFs close within their original bounds, with exact
container absence and no shared writable target/cache.

This compilation reruns no behavioral tests or Qt checks. `audio-client-test`
is a Qt/CMake target and was not treated as a Rust binary. The earlier32 Qt
checks are not current native-guest evidence. No production behavior changes,
new behavioral RED/GREEN result or screenshot follows from this checkpoint.

## Offline installed guest

A later fresh native KVM child now qualifies the offline installed software
cohort. Native pacman resolves against frozen repository databases;338 original
and29 additional archives have actual trusted detached-signature checks. The
lossless367-package cache preserves the original bundle. Package download/setup
failures remain preserved; they are not production behavioral REDs.

The first guest install misses `pw-cat`: Arch supplies it and `pw-record` in
[pipewire-audio](https://archlinux.org/packages/extra/x86_64/pipewire-audio/files/).
The corrected bundle adds29 missing archives for it and its dependencies. A second fresh
guest reaches all seven tool and four SPA-file hashes, but its version-only
Hyprland query aborts without XDG_RUNTIME_DIR. A private root0700 directory for
that same-child version exec corrects the fixture environment. Both failed
layers remain failed and are not reused.

The third fresh child grows the Btrfs root to8,273,244,160 bytes. Native pacman
projects the complete actual stock upgrade before mutation and finds no missing
archive; its strict signed offline transaction applies187 archives. The actual
installed inventory contains367 packages. Installed Hyprland and SDK bytes match
the signed archive, and the actual version query plus version.h bind commit
`efb50993780079460b0cbed1363e2166a2de1d9f`. QtGui6.12.0, Wayland1.26.0 and
libdrm2.4.134 are observed. This executes a version query, not a compositor session.

All18 guest commands, one early command, five boot children and seven host CLI
waits meet their original cutoffs, with38 actual PIPE EOFs. The same authenticated
QMP stream confirms KVM and normal guest shutdown, socket EOF and QEMU0; Docker
CLI0, exact container absence and750 external input hashes close. The envelope
takes36.11 seconds. Guest work/closure remains420/480 seconds, QEMU600/630 and
host660/690. These are setup limits, not changes to the100ms Source lease.

Only the fresh child receives disk writes. Its closed8GiB virtual QCOW2 is
hashed after QEMU wait0 and sealed0444; the authenticated original remains
read-only. The guest results export excludes the active disk and host selection.
[Hash-only provisioning evidence](evidence/native-session-x86-provision-provenance.json)
binds the archive/source/layer/SDK/inventory results and preserved failures. No Qt
build, component cases, normal app, final display or PCM runs in this checkpoint.

## Actual native Qt and composition attempt

One fresh native KVM child now builds and installs the current normal Qt app and
two test tools with `-Wall -Wextra -Werror`, using eight build jobs. All three
binaries are x86 ELF machine62. Its actual32 audio checks include setup/cleanup
and30 behaviors; all29 appearance cases also pass. These checks execute in this
epoch against Qt6.12.0, rather than inheriting the earlier ARM receipts. All17
selected Qt/desktop source files remain unchanged before/after. The component
scope includes offscreen appearance checks, without a normal app or display claim.

The matching SDK builds all three compositor plugins and the non-presenting
discovery helper. The Rust owner starts the exact installed Hyprland binary as
UID1000 with no supplementary groups. Its retained compositor pidfd becomes dead
during cold startup, before normal Sounds launch or final profile qualification.
The before-Stop authenticated root snapshot retains sequence1 sampled at
54,297,771,031ns and first provider `InvalidData` observed at54,409,790,650ns.
That112,019,619ns difference is neither a rejected-sample age nor a proven
producer gap. `InvalidData` covers receive EOF/truncation and multiple parsed
frame checks; no particular rejection or scheduling cause is known. The
cooperative startup witness retains13 committed landmarks with zero flags,
without proving callback return, in-flight work or an empty queue.

The functional selection stays failed, with zero boundary groups, PCM windows
or new captures. Its78 guest, five bootstrap, two early, five boot and seven
host direct waits meet their original cutoffs, with104 actual PIPE EOFs. The
finite tone child returns its declared-allowed1; the two host absence checks
return their expected1. Original Source/compositor child custody is not relabeled
as a numerical native-parent wait: the external pidfd is dead and the owner
itself waits0. The authenticated QMP stream confirms KVM, normal guest shutdown,
socket EOF and actual QEMU0; Docker CLI0 and exact CID absence on the same daemon
qualify separately. All78 selected source and27 external input hashes remain
unchanged under the shared lock. The complete envelope takes60.82 seconds.
The raw driver also labels the CID unresolved in its functional-assertion handler;
that field remains preserved alongside the independently qualified exact absence.

[Hash-only attempt evidence](evidence/native-session-x86-composition-provenance.json)
keeps the current Qt passes separate from full session failure. This changes no
production behavior and supplies no new behavioral RED/GREEN result.

## Next integration

The [bounded typed first-rejection record](native-source-observation.md#first-observer-rejection-evidence)
now passes12 admin cases after eight actual missing-record REDs, plus57 Observer
process and four lifecycle cases. Strict formatting/Clippy and default-feature
native owner/client release builds pass. Its receive/sample decisions remain
historical observations, with no raw frames or peer identities retained. The
next replay selects release opt3 owner/client instead of the prior debug opt0
artifacts; neither a changed result nor timing would establish a cause. Preserve
the existing ErrorKind,100ms lease, sequence checks, terminal refusal and cleanup.
Then run the unchanged four-step session composition once: denied quiet launch,
current grant and real focused touch with PCM, null-focus background continuation with disabled
touch, and terminal Source loss without app/audio revival. Preserve per-app
Wayland, independent grants,1080×2424/422PPI/420 logical density, capture guards,
all graph/PCM assertions and the100ms lease.

Native x86 success would qualify that environment only. The earlier ARM Source
expiry remains failed and does not establish a TCG cause. ARM availability,
matched Google kernel integration, complete Omarchy shell/startup and every
physical Pixel/carrier gate remain open. No physical phone has been accessed.
