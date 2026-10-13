# Pokélinux working rules

- At every work turn, check that README.md and affected documentation reflect the current implementation and evidence. Update them before committing or pushing changed behavior or new test results. Do not add meaningless edits when nothing has changed.
- Separate original-code execution, synthetic emulator integration, ARM64 user-mode emulation and physical-device evidence. Document unresolved behavior explicitly.
- The target is native Rust phone services, shared across hardware through separate vendor backends. Java Android peers are test infrastructure. Follow docs/smartphone-goal.md and docs/device-portability.md.
- Do not access or modify a physical phone. Use Harbour only as temporary build and validation capacity.
- Do not add backward compatibility for earlier project iterations. Do not publish proprietary binaries, decompiled source, credentials or raw private audit artifacts.
- Future UI runs and previews use the Pixel 10 display profile: portrait1080 ×2424 native pixels,422PPI and the matched image's420 logical density. Qualify actual output/window metadata and native capture dimensions before presenting screenshots; never resize a capture to satisfy this rule. See docs/native-phone-display-profile.md. Preserve older evidence at its recorded geometry. A deliberately wrong-density negative measurement is private test evidence, not a UI preview.
