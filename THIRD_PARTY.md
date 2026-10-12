# Third-party materials

The root MIT license applies to original project code and documentation. It does
not replace upstream licenses or grant trademark rights. Patches contain upstream
context; that context retains its original license. Dependencies and any resulting
binary distribution must satisfy their respective license obligations.

- libhybris: Linux-on-droid fork, pinned in sources.lock.json; retain its file-level
  license notices (including Apache, BSD and LGPL components).
- Android headers: Droidian android-headers-30, pinned in sources.lock.json;
  preserve Android and upstream file-level notices.
- Omarchy: selected sources pinned in dependencies/omarchy-files.lock.json;
  upstream MIT license is fetched as part of the source inventory. The shell
  patch retains that license. Omarchy names and logos are not licensed by MIT.
- rsbinder: version 0.12.0, Apache-2.0, used by the native Linux Binder
  qualification client; exact transitive versions are in native-radio/Cargo.lock.
- Rust crates: versions and source checksums in each Cargo.lock; each crate retains
  its own license. Qt, Linux, Mesa, Hyprland and other system dependencies retain
  their upstream licenses.
- Theme screenshots show actual VM test runs, not hardware support. The theme
  names identify the upstream themes; artwork remains attributable to its owners.
- Generated banner artwork is conceptual project artwork, not evidence of a
  working device and not an official upstream logo.

Proprietary factory images, APKs, vendor libraries and firmware are not included.
Google and other vendors retain their rights. Protocol observations and test
fixtures do not grant rights to redistribute the original inputs.
