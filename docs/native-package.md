# Native Phone package checkpoint

2026-10-12. `artifacts/client/omarchy-phone-0.1.2-1-aarch64.pkg.tar`
contains the Rust daemon, Rust CLI, standalone Qt/QML app, user service unit
and desktop launcher. The QML and palette code are embedded in the app binary.
There is no Python/GTK runtime, old-format reader, fixture identity or test
executable in this archive. Enrollment is separate and is not activated by
building or extracting the package.

The package builder now requires all three native ELF64 binaries with the
selected architecture. Tests reject an absent or wrong-architecture app and
verify archive contents, modes, launcher, service settings and reproducibility.
Two builds with identical inputs produced identical archive bytes.

Dependencies are glibc, libgcc, libstdc++, systemd, qt6-base >= 6.8,
qt6-declarative >= 6.8 and qt6-wayland >= 6.8. The current
[Arch Linux ARM Qt base package](https://archlinuxarm.org/packages/aarch64/qt6-base)
uses separate libgcc and libstdc++ packages. The broad gcc-libs metapackage is
not needed for these binaries. Qt declarative provides the QML/Quick runtime;
Wayland support is required for the intended compositor environment.

## Verified

- Four package-builder tests pass after two intended red failures for the
  missing app and incomplete archive.
- ELF inspection identifies all three packaged binaries as AArch64. Dynamic
  libraries resolve in the native ARM64 Linux validation image.
- The archive was extracted to a fresh temporary directory. All three binary
  payloads match their build inputs. The app also matches a new build from
  current sources byte for byte.
- All 54 native behavior cases / 68 Qt entries pass with archive-extracted
  binaries. This includes installed-path process startup, single-instance
  activation, complete-window session establishment with the Rust daemon,
  real journal recovery and the separate touch Answer/Dial synthetic-TLS test.

Evidence is in `artifacts/rust-native-local/app-package-*`,
`qt-tested-binaries.sha256`, `qt-sources.sha256`, `qt-tests.log` and
`qt-run.json`. `app-package-result.json` records the archive hash and explicit
verification limits.

## Still required

This archive uses binaries built and tested with Debian's ARM64 Qt 6.8.2
environment. Extraction and execution there are not proof of pacman dependency
resolution or operation against the target Arch ARM Qt 6.12 environment.
Pacman installation, actual systemd user-service activation, Omarchy launcher
and Wayland focus behavior, package/source licensing completion, full font
integration, background ringing and the current Android framework suite remain
unverified. The package is an offline integration artifact, not a release claim.

The existing Harbour Arch/Android guests remain the intended next deployment
test environment. The latest SSH check still required Tailscale reauthentication;
the waiting connection was stopped. No physical phone was accessed.
