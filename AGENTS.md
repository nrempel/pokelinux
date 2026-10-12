# Pokélinux working rules

- At every work turn, check that README.md and affected documentation reflect the current implementation and evidence. Update them before committing or pushing changed behavior or new test results. Do not add meaningless edits when nothing has changed.
- Separate original-code execution, synthetic emulator integration, ARM64 user-mode emulation and physical-device evidence. Document unresolved behavior explicitly.
- The target is native Rust phone services, shared across hardware through separate vendor backends. Java Android peers are test infrastructure. Follow docs/smartphone-goal.md and docs/device-portability.md.
- Do not access or modify a physical phone. Use Harbour only as temporary build and validation capacity.
- Do not add backward compatibility for earlier project iterations. Do not publish proprietary binaries, decompiled source, credentials or raw private audit artifacts.
