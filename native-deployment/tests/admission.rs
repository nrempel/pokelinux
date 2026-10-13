use pokelinux_deployment::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    profile: Profile,
    inventory: Inventory,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "pokelinux-deployment-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    let profile = Profile {
        schema: 1,
        id: "authored-staging-fixture".into(),
        device: "authored-offline-device".into(),
        architecture: arch.into(),
        component_set: "authored-current-set".into(),
        evidence: "authored-fixture".into(),
        kernel_release: "authored-kernel".into(),
        artifacts: vec![ArtifactPin {
            id: "kernel".into(),
            kind: "kernel".into(),
            sha256: Some(hash(b"authored kernel bytes")),
            source: "authored bytes, not a kernel image".into(),
        }],
        resources: vec!["messages".into(), "permissions".into()],
        service_contracts: vec![
            ServiceContract {
                id: "messages".into(),
                owns: vec!["messages".into()],
                dependencies: vec!["permissions".into()],
            },
            ServiceContract {
                id: "permissions".into(),
                owns: vec!["permissions".into()],
                dependencies: vec![],
            },
        ],
        gates: REQUIRED_GATES
            .iter()
            .map(|id| Gate {
                id: (*id).into(),
                scope: if *id == "physical-device" {
                    "physical"
                } else {
                    "offline"
                }
                .into(),
                state: "qualified".into(),
                evidence: "authored fixture assertion, not original evidence".into(),
            })
            .collect(),
        capabilities: vec![Capability {
            id: "messages".into(),
            available: false,
            qualification: "installed-test".into(),
            detail: "fixture only".into(),
        }],
    };
    let mut inventory = Inventory {
        schema: 1,
        profile: profile.id.clone(),
        device: profile.device.clone(),
        architecture: arch.into(),
        component_set: profile.component_set.clone(),
        artifacts: vec![Artifact {
            id: "kernel".into(),
            path: "factory/kernel".into(),
            component_set: profile.component_set.clone(),
            kernel_release: profile.kernel_release.clone(),
        }],
        packages: vec![],
        files: vec![],
        services: vec![],
        enable_capabilities: vec![],
    };
    write(&root, "factory/kernel", b"authored kernel bytes", 0o644);
    inventory.files.push(Payload {
        path: "factory/kernel".into(),
        sha256: hash(b"authored kernel bytes"),
        mode: 0o644,
        package: None,
        kind: "kernel".into(),
    });
    // This is an actual compiled native executable for this test's architecture.
    let elf_path = std::env::var("DEPLOYMENT_TEST_ELF")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_pokelinux-deployment").into());
    let elf = fs::read(elf_path).unwrap();
    for name in ["messages", "permissions"] {
        let path = format!("usr/bin/{name}");
        write(&root, &path, &elf, 0o755);
        inventory.files.push(Payload {
            path: path.clone(),
            sha256: hash(&elf),
            mode: 0o755,
            package: Some(name.into()),
            kind: "native-executable".into(),
        });
        let meta = format!("usr/share/pokelinux/packages/{name}/.PKGINFO");
        let deps = if name == "messages" {
            vec![Dependency {
                name: "permissions".into(),
                version: "1.0.0-1".into(),
            }]
        } else {
            vec![]
        };
        let text = format!(
            "pkgname = {name}\npkgver = 1.0.0-1\narch = {arch}\n{}",
            if name == "messages" {
                "depend = permissions=1.0.0-1\n"
            } else {
                ""
            }
        );
        write(&root, &meta, text.as_bytes(), 0o644);
        inventory.files.push(Payload {
            path: meta.clone(),
            sha256: hash(text.as_bytes()),
            mode: 0o644,
            package: Some(name.into()),
            kind: "native-resource".into(),
        });
        inventory.packages.push(Package {
            name: name.into(),
            version: "1.0.0-1".into(),
            architecture: arch.into(),
            component_set: profile.component_set.clone(),
            metadata: meta,
            dependencies: deps,
        });
        inventory.services.push(Service {
            id: name.into(),
            runtime: "native-rust".into(),
            executable: path,
            package: name.into(),
            owns: vec![name.into()],
            dependencies: if name == "messages" {
                vec!["permissions".into()]
            } else {
                vec![]
            },
        });
    }
    Fixture {
        root,
        profile,
        inventory,
    }
}
fn write(root: &Path, path: &str, bytes: &[u8], mode: u32) {
    let p = root.join(path);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, bytes).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(mode)).unwrap();
}
fn check(f: &Fixture) -> Report {
    validate(&f.profile, &f.inventory, &f.root)
}
fn rejected(f: &Fixture, code: &str) {
    let r = check(f);
    assert!(!r.deployment_admitted, "incorrectly admitted {code}");
    assert!(
        r.issues.iter().any(|i| i.code == code),
        "missing {code}: {:?}",
        r
    );
}
#[test]
fn actual_compiled_native_elf_and_package_inventory_are_valid_but_fixture_cannot_deploy() {
    let f = fixture();
    let r = check(&f);
    assert!(r.inventory_valid, "{r:?}");
    rejected(&f, "fixture-evidence");
}
#[test]
fn modified_payload_is_refused() {
    let f = fixture();
    fs::write(f.root.join("usr/bin/messages"), b"tamper").unwrap();
    rejected(&f, "file-hash");
}
#[test]
fn absent_payload_is_refused() {
    let f = fixture();
    fs::remove_file(f.root.join("usr/bin/messages")).unwrap();
    rejected(&f, "missing-file");
}
#[test]
fn extra_unlisted_payload_is_refused() {
    let f = fixture();
    write(&f.root, "usr/bin/hidden-android", b"unlisted", 0o755);
    rejected(&f, "unlisted-file");
}
#[test]
fn symlink_payload_and_symlink_parent_are_refused() {
    let f = fixture();
    fs::remove_file(f.root.join("usr/bin/messages")).unwrap();
    symlink("permissions", f.root.join("usr/bin/messages")).unwrap();
    rejected(&f, "unsafe-file");
    let g = fixture();
    fs::rename(g.root.join("usr/bin"), g.root.join("saved-bin")).unwrap();
    symlink("../saved-bin", g.root.join("usr/bin")).unwrap();
    rejected(&g, "unsafe-file");
}
#[test]
fn path_escape_is_refused() {
    let mut f = fixture();
    f.inventory.files[0].path = "../outside".into();
    rejected(&f, "unsafe-path");
}
#[test]
fn mode_change_is_refused() {
    let f = fixture();
    fs::set_permissions(
        f.root.join("usr/bin/messages"),
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    rejected(&f, "file-mode");
}
#[test]
fn wrong_elf_architecture_even_with_matching_digest_is_refused() {
    let mut f = fixture();
    let p = f.root.join("usr/bin/messages");
    let mut b = fs::read(&p).unwrap();
    b[18..20].copy_from_slice(
        &if cfg!(target_arch = "aarch64") {
            62u16
        } else {
            183u16
        }
        .to_le_bytes(),
    );
    fs::write(p, &b).unwrap();
    f.inventory.files[1].sha256 = hash(&b);
    rejected(&f, "native-elf");
}
#[test]
fn malformed_elf_even_with_matching_digest_is_refused() {
    let mut f = fixture();
    write(&f.root, "usr/bin/messages", b"\x7fELF", 0o755);
    f.inventory.files[1].sha256 = hash(b"\x7fELF");
    rejected(&f, "native-elf");
}
#[test]
fn bionic_interpreter_even_with_matching_digest_is_refused() {
    let mut f = fixture();
    let p = f.root.join("usr/bin/messages");
    let mut b = fs::read(&p).unwrap();
    let needle = if cfg!(target_arch = "aarch64") {
        b"/lib/ld-linux-aarch64.so.1".as_slice()
    } else {
        b"/lib64/ld-linux-x86-64.so.2".as_slice()
    };
    let at = b.windows(needle.len()).position(|w| w == needle).unwrap();
    b[at..at + needle.len()].fill(0);
    b[at..at + 21].copy_from_slice(b"/system/bin/linker64\0");
    fs::write(p, &b).unwrap();
    f.inventory.files[1].sha256 = hash(&b);
    rejected(&f, "native-elf");
}
#[test]
fn mismatched_factory_set_and_kernel_release_are_refused() {
    let mut f = fixture();
    f.inventory.artifacts[0].component_set = "other-factory".into();
    rejected(&f, "component-set");
    f.inventory.artifacts[0].component_set = f.profile.component_set.clone();
    f.inventory.artifacts[0].kernel_release = "other-kernel".into();
    rejected(&f, "kernel-release");
}
#[test]
fn missing_or_unpinned_factory_artifact_is_refused() {
    let mut f = fixture();
    f.inventory.artifacts.clear();
    rejected(&f, "missing-artifact");
    f = fixture();
    f.profile.artifacts[0].sha256 = None;
    rejected(&f, "unpinned-artifact");
}
#[test]
fn mismatched_package_metadata_and_dependency_are_refused() {
    let mut f = fixture();
    f.inventory.packages[0].version = "other-version".into();
    rejected(&f, "package-metadata");
    f = fixture();
    f.inventory.packages[1].version = "other-version".into();
    rejected(&f, "package-dependency");
}
#[test]
fn duplicate_resource_owner_and_missing_owner_are_refused() {
    let mut f = fixture();
    f.inventory.services[1].owns.push("messages".into());
    rejected(&f, "resource-owner");
    f = fixture();
    f.inventory.services[0].owns.clear();
    rejected(&f, "resource-owner");
}
#[test]
fn missing_service_dependency_and_cycle_are_refused() {
    let mut f = fixture();
    f.inventory.services[0].dependencies.push("missing".into());
    rejected(&f, "service-dependency");
    f = fixture();
    f.inventory.services[1].dependencies.push("messages".into());
    rejected(&f, "service-cycle");
}
#[test]
fn android_java_and_vendor_userspace_are_refused() {
    for runtime in ["android-java", "bionic-vendor"] {
        let mut f = fixture();
        f.inventory.services[0].runtime = runtime.into();
        rejected(&f, "non-native-runtime");
    }
}
#[test]
fn unresolved_or_omitted_gates_cannot_admit_deployment() {
    let mut f = fixture();
    f.profile.gates[0].state = "unresolved".into();
    rejected(&f, "unresolved-gate");
    f = fixture();
    f.profile.gates.remove(0);
    rejected(&f, "missing-gate");
}
#[test]
fn unsupported_or_unknown_capability_cannot_be_enabled() {
    let mut f = fixture();
    f.inventory.enable_capabilities.push("messages".into());
    rejected(&f, "unsupported-capability");
    f.inventory.enable_capabilities[0] = "unknown".into();
    rejected(&f, "unsupported-capability");
}
#[test]
fn profile_identity_and_unknown_schema_are_refused() {
    let mut f = fixture();
    f.inventory.profile = "other-device".into();
    rejected(&f, "profile-mismatch");
    f = fixture();
    f.inventory.schema = 2;
    rejected(&f, "schema");
}

#[test]
fn service_contract_requires_profile_declared_dependencies() {
    let mut f = fixture();
    f.inventory.services[0].dependencies.clear();
    rejected(&f, "service-contract");
    let mut g = fixture();
    g.inventory.services[0].id = "substitute-owner".into();
    rejected(&g, "service-contract");
}
#[test]
fn service_contract_profile_cannot_omit_or_duplicate_owners() {
    let mut f = fixture();
    f.profile.service_contracts.clear();
    rejected(&f, "service-contract");
    let mut g = fixture();
    g.profile.service_contracts[1].owns.push("messages".into());
    rejected(&g, "service-contract");
}

fn mutate_load(f: &mut Fixture, executable: bool, change: impl FnOnce(&mut [u8], usize)) {
    assert!(
        check(f).inventory_valid,
        "actual compiled positive fixture must conform"
    );
    let path = f.root.join("usr/bin/messages");
    let mut bytes = fs::read(&path).unwrap();
    let start = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
    let count = u16::from_le_bytes(bytes[56..58].try_into().unwrap()) as usize;
    let at = (0..count)
        .map(|i| start + i * 56)
        .find(|&at| {
            let kind = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            let flags = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap());
            kind == 1 && (flags & 1 != 0) == executable
        })
        .expect("original compiler must have requested load segment");
    change(&mut bytes, at);
    fs::write(path, &bytes).unwrap();
    f.inventory.files[1].sha256 = hash(&bytes);
    rejected(f, "native-elf");
}
#[test]
fn load_file_extent_outside_actual_payload_or_overflow_is_refused() {
    for overflow in [false, true] {
        let mut f = fixture();
        mutate_load(&mut f, true, |b, at| {
            let offset = if overflow {
                u64::MAX - 1
            } else {
                b.len() as u64 + 1
            };
            b[at + 8..at + 16].copy_from_slice(&offset.to_le_bytes());
        });
    }
}
#[test]
fn load_memory_extent_smaller_than_file_is_refused() {
    let mut f = fixture();
    mutate_load(&mut f, true, |b, at| {
        let file_size = u64::from_le_bytes(b[at + 32..at + 40].try_into().unwrap());
        assert!(file_size > 1);
        b[at + 40..at + 48].copy_from_slice(&(file_size - 1).to_le_bytes());
    });
}
#[test]
fn load_virtual_extent_overflow_is_refused() {
    let mut f = fixture();
    mutate_load(&mut f, true, |b, at| {
        b[at + 16..at + 24].copy_from_slice(&(u64::MAX - 4).to_le_bytes())
    });
}
#[test]
fn load_non_power_of_two_alignment_is_refused() {
    let mut f = fixture();
    mutate_load(&mut f, true, |b, at| {
        b[at + 48..at + 56].copy_from_slice(&3u64.to_le_bytes())
    });
}
#[test]
fn load_address_and_file_offset_incongruence_is_refused() {
    let mut f = fixture();
    mutate_load(&mut f, true, |b, at| {
        let address = u64::from_le_bytes(b[at + 16..at + 24].try_into().unwrap());
        let align = u64::from_le_bytes(b[at + 48..at + 56].try_into().unwrap());
        assert!(align > 1);
        b[at + 16..at + 24].copy_from_slice(&(address + 1).to_le_bytes());
    });
}
#[test]
fn entry_outside_file_backed_executable_loads_is_refused() {
    let mut f = fixture();
    mutate_load(&mut f, true, |b, _| {
        b[24..32].copy_from_slice(&(u64::MAX - 1).to_le_bytes())
    });
}
#[test]
fn malformed_nonexecutable_load_is_also_refused() {
    let mut f = fixture();
    mutate_load(&mut f, false, |b, at| {
        b[at + 8..at + 16].copy_from_slice(&u64::MAX.to_le_bytes())
    });
}

#[test]
fn excessive_valid_package_metadata_is_refused_before_loading() {
    let mut f = fixture();
    let name = f.inventory.packages[0].metadata.clone();
    let mut text = String::from("# ");
    text.push_str(&"authored package metadata note; ".repeat(70000));
    text.push_str(&format!(
        "\npkgname = messages\npkgver = 1.0.0-1\narch = {}\ndepend = permissions=1.0.0-1\n",
        f.profile.architecture
    ));
    write(&f.root, &name, text.as_bytes(), 0o644);
    f.inventory
        .files
        .iter_mut()
        .find(|row| row.path == name)
        .unwrap()
        .sha256 = hash(text.as_bytes());
    rejected(&f, "metadata-bound");
}
