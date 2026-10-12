# Current native package candidate

The current offline candidate is **omarchy-phone 0.1.4-1 aarch64** at
`artifacts/client/omarchy-phone-0.1.4-1-aarch64.pkg.tar`. It contains release-mode
Rust daemon/CLI binaries, the release-mode native Qt app, desktop entry,
fixed-path D-Bus activation descriptor, systemd user unit and reproducible gzip
`.MTREE` file-integrity metadata. It contains no Python runtime, credentials,
Android APK, legacy adapter or journal migration.

## Current Arch validation

`tools/arch-package-suite.sh` installs the archive with pacman in an isolated
Arch Linux ARM container, with networking disabled. Dependencies resolve against
the image's installed Qt 6.12.0 packages. Pacman reports six installed files and
zero altered files. Installed executable and activation-descriptor hashes match
the archive. All seven native CTest executables pass: **55 behavior cases plus
14 Qt setup/cleanup entries**, running as an unprivileged test user. These cover
the real Rust service/journal, authenticated synthetic Answer/Dial, recovery,
themes and D-Bus startup of the packaged app.

The Arch base rootfs matches the project's pinned SHA-256 and detached signature
fingerprint. `sim/Dockerfile.arch-local` creates the validation image with a full
package upgrade; the exact image IDs and installed versions are recorded. The
Docker LinuxKit kernel lacks Landlock, so only pacman's filesystem downloader
sandbox is disabled inside this disposable environment. Download UID, syscall
sandbox and repository signature checks remain active. The suite creates its
test UID in the disposable passwd database because Arch D-Bus requires it.
Child-app diagnostic tests explicitly request Qt console logging so stderr
assertions cannot silently inspect an empty platform log stream.

Evidence: `artifacts/rust-native-local/arch-package-result.json`,
`arch-package-tests.log`, `arch-package-check.log` and
`arch-package-versions.txt`. The earlier 0.1.3 archive remains unchanged; its
Debian extraction tests are recorded separately in `package-013-result.json`.

This is not a finished Omarchy smartphone release. Actual Hyprland/Omarchy
session operation, systemd startup, current Android framework integration,
licensing inventory and native setup remain open. The presentation owner is
not yet enabled in the daemon; host session/lock/power integration is incomplete.
There is no physical phone, GPU, modem, IMS, audio, camera or suspend/wake proof
in this checkpoint. The historical Python paths below are not installation or
upgrade instructions for this native candidate. Obsolete journal schemas are
refused, with no compatibility migration.

# Historical package checkpoints

Historical candidate: client **0.6.0** with native plugin **0.4.0**. The client installs the native Phone launcher, eleven Python runtime modules, enrollment tools and systemd user service. Historical GTK remains an explicit optional legacy interface. The historical checkpoints below describe older releases.

## Coordinated upgrades

The mandatory controller-session protocol requires the current daemon and native plugin together. End any call and finish recovery before a routine upgrade. Stop the Omarchy shell through its session lifecycle, then stop the user daemon. Install both packages in one pacman transaction. Reload the user service definitions and start the daemon before starting the shell again. Preserve the pairing configuration and state directory; deleting the operation ledger is not an upgrade step.

```sh
systemctl --user stop omarchy-phone.service
sudo pacman -U omarchy-phone-0.6.0-1-any.pkg.tar omarchy-phone-plugin-0.4.0-1-any.pkg.tar
systemctl --user daemon-reload
systemctl --user start omarchy-phone.service
```

These commands assume an already enrolled and intentionally running user service. Installation alone still does not activate or enroll anything. Use the existing Omarchy session manager to stop/start the shell; no unverified generic shell restart command is prescribed here.

`artifacts/native-session-upgrade/` verifies pacman upgrade from client 0.2.0/plugin 0.3.0 to 0.5.0/0.4.0 in ARM64 Hyprland/Omarchy. The test stops both processes, preserves a seeded historical command record across the daemon restart/schema upgrade, starts the new shell, invokes the desktop entry twice with one panel, and exercises touch Answer/End. It then disables, unregisters, removes, reinstalls and registers the plugin; after another shell restart the disabled setting remains and the launcher rejects opening. No extra call commands occur. Package hashes and all eight installed plugin runtime source hashes match the current candidate.

This upgrade fixture starts the installed daemon classes directly with a synthetic Android adapter. It is not an upgrade-through-systemd or Android-backed upgrade test. Actual systemd service operation and normal enrolled Android calls are covered separately by `artifacts/native-session-android/`; do not combine those into a stronger upgrade claim.

## Historical package baseline


`artifacts/client/omarchy-phone-0.2.0-1-any.pkg.tar` contains the existing GTK interface, enrollment tools, the background daemon, its durable ledger and command dispatcher, the one-shot native client, and a systemd user unit. No pairing credentials, Android APK, tests or activation hooks are included. The 0.1.0 artifact remains the historical GTK integration baseline.

Installation and service controls:

```sh
sudo pacman -U omarchy-phone-0.2.0-1-any.pkg.tar
systemctl --user start omarchy-phone.service
systemctl --user status omarchy-phone.service
systemctl --user stop omarchy-phone.service
```

Installation does not start or enable the service. To start it with the user session, use `systemctl --user enable omarchy-phone.service`; disable that with `systemctl --user disable omarchy-phone.service`. Explicit stop does not trigger automatic restart. An unexpected crash does trigger restart, with a new service epoch and existing durable outcomes retained.

The unit uses the user's XDG config, runtime and state paths through systemd's `%E`, `%t` and `%S` specifiers. Pairing is read from `omarchy-phone/client.json` beneath the config directory. The existing GTK file picker can select another path; that setting does not yet reconfigure the background unit. A final native setup flow must reconcile this before replacing GTK.

The daemon publishes a same-user private Unix socket. Both the socket and ledger are mode 0600; their directories are private. A missing pairing file leaves command authority unavailable. The unit sets NoNewPrivileges and a private umask. It starts independently of the visible shell and is not stopped when a phone panel closes.

## Verified installation checkpoint

The actual package installed with pacman in a disposable ARM64 Arch VM. The actual systemd user manager ran it as desktop UID 1000. The unit passed systemd-analyze verification. Tests verified disabled-after-install, private ownership/modes, idempotent start, SIGKILL recovery, a new epoch on restart, clean explicit stop and no command authority without pairing. Installed package, ten Python modules and unit hashes match current sources. Evidence: `artifacts/service-package/`. All 99 Linux Python tests pass.

The subsequent real Android integration passed through the installed package and actual systemd user service as desktop UID 1000. Its installed CLI submitted Answer, observed active state, recovered the receipt, returned the saved receipt for the original duplicate command, submitted End and observed no current call. It used the enrolled mutual-TLS connection to the owned emulator, with Android SELinux Enforcing. All installed module and current Android source hashes match. Evidence: `artifacts/service-package/android/`.

The top-level enrollment provenance still references the historical 0.1.0 GTK package; the new `packaged_user_service` result and `source-verification.json` explicitly identify the exercised 0.2.0 package. Do not confuse those scopes.

This does not prove a QML plugin, audio, suspend/wake or Pixel hardware. The earlier intermittent emulator call loss is still unexplained; this passing run does not close it. No physical device was accessed.

Client 0.6.0 adds native CLI dialing; its installed systemd/Android path passed in `artifacts/native-dial-android/`. The recorded package-upgrade matrix above is specifically 0.5.0 and is not silently extended to 0.6.0. See `native-outgoing.md`.
