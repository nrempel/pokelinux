//! One protected startup profile. Namespace ownership belongs to the Service;
//! this module only validates configuration and the exact opened input file.
use rustix::fs::{CWD, FileType, Mode, OFlags, fstat, openat};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

const MAX_PROFILE_BYTES: usize = 4096;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "backend", rename_all = "snake_case", deny_unknown_fields)]
pub enum Profile {
    VirtualPair {
        peer_namespace: PathBuf,
    },
    WirelessOpen {
        target: Target,
        frequency_mhz: u32,
        bssid: [u8; 6],
        ssid: Vec<u8>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub namespace: Namespace,
    pub ifindex: u32,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
    pub dev: u64,
    pub ino: u64,
}

impl Profile {
    /// Validate the same selectors after decoding the trusted worker startup
    /// record. This performs no namespace lookup, backend I/O or mutation.
    pub fn validate(&self) -> io::Result<()> {
        let valid = match self {
            Self::VirtualPair { peer_namespace } => {
                peer_namespace.is_absolute()
                    && !peer_namespace.as_os_str().as_encoded_bytes().contains(&0)
            }
            Self::WirelessOpen {
                target,
                frequency_mhz,
                bssid,
                ssid,
            } => {
                target.namespace.ino > 0
                    && target.ifindex > 1
                    && !target.name.is_empty()
                    && target.name.len() <= 15
                    && target.name != "lo"
                    && target
                        .name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
                    && (1..=100_000).contains(frequency_mhz)
                    && *bssid != [0; 6]
                    && bssid[0] & 1 == 0
                    && !ssid.is_empty()
                    && ssid.len() <= 32
            }
        };
        if valid {
            Ok(())
        } else {
            Err(io::ErrorKind::InvalidData.into())
        }
    }
}

/// Open exactly once, validate that descriptor, and read only from it. NONBLOCK
/// prevents a FIFO pathname from waiting for a writer before type validation.
pub fn load(path: &Path) -> io::Result<Profile> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fd = openat(
        CWD,
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    from_file(File::from(fd))
}

fn from_file(file: File) -> io::Result<Profile> {
    let metadata = fstat(&file)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
        || metadata.st_uid != 0
        || metadata.st_gid != 0
        || metadata.st_mode & 0o7777 != 0o600
        || metadata.st_nlink != 1
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut bytes = Vec::with_capacity(MAX_PROFILE_BYTES + 1);
    file.take((MAX_PROFILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PROFILE_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let profile: Profile = serde_json::from_slice(&bytes)?;
    profile.validate()?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::fs::{CWD, Mode, mkfifoat};
    use serde_json::{Value, json};
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt, chown, symlink},
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            assert_eq!(
                rustix::process::geteuid().as_raw(),
                0,
                "isolated UID0 file-boundary tests"
            );
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "connectivity-startup-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            path
        }
        fn json(&self, name: &str, value: &Value) -> PathBuf {
            self.file(name, &serde_json::to_vec(value).unwrap())
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn wireless() -> Value {
        let namespace = File::open("/proc/thread-self/ns/net")
            .unwrap()
            .metadata()
            .unwrap();
        json!({"backend":"wireless_open", "target":{
            "namespace":{"dev":namespace.dev(),"ino":namespace.ino()},
            "ifindex":2,"name":"wlan1"},"frequency_mhz":2412,
            "bssid":[2,0,0,0,0,1],"ssid":[112,107,108]})
    }

    #[test]
    fn actual_root0600_file_loads_both_explicit_backends() {
        let root = Directory::new();
        for (number, value) in [
            wireless(),
            json!({"backend":"virtual_pair", "peer_namespace":"/proc/self/ns/net"}),
        ]
        .iter()
        .enumerate()
        {
            let path = root.json(&number.to_string(), value);
            let profile = load(&path).expect("protected explicit startup profile must load");
            assert_eq!(serde_json::to_value(&profile).unwrap(), *value);
            profile.validate().unwrap();
        }
    }

    #[test]
    fn opened_inode_remains_the_only_input_after_path_replacement() {
        let root = Directory::new();
        let expected = wireless();
        let path = root.json("profile", &expected);
        let opened = File::open(&path).unwrap();
        let opened_inode = opened.metadata().unwrap().ino();
        fs::rename(&path, root.0.join("retained-original")).unwrap();
        root.file("profile", b"not the opened profile");
        assert_ne!(fs::metadata(&path).unwrap().ino(), opened_inode);
        assert_eq!(
            serde_json::to_value(from_file(opened).unwrap()).unwrap(),
            expected
        );
    }

    #[test]
    fn actual_symlink_fifo_directory_and_namespace_path_are_refused() {
        let root = Directory::new();
        let target = root.json("target", &wireless());
        let linked = root.0.join("symlink");
        symlink(&target, &linked).unwrap();
        assert!(load(&linked).is_err());
        let fifo = root.0.join("fifo");
        mkfifoat(CWD, &fifo, Mode::RUSR | Mode::WUSR).unwrap();
        assert!(
            load(&fifo).is_err(),
            "FIFO must be refused without blocking for a writer"
        );
        assert!(load(&root.0).is_err());
        assert!(
            load(Path::new("/proc/thread-self/ns/net")).is_err(),
            "old namespace argv2 has no fallback"
        );
    }

    #[test]
    fn actual_wrong_modes_owner_group_and_hardlink_are_refused() {
        let root = Directory::new();
        let path = root.json("profile", &wireless());
        for mode in [0o400, 0o644, 0o666, 0o4600, 0o2600, 0o1600] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            assert!(load(&path).is_err(), "exact0600 required: {mode:o}");
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        chown(&path, Some(1000), Some(0)).unwrap();
        assert!(load(&path).is_err());
        chown(&path, Some(0), Some(1000)).unwrap();
        assert!(load(&path).is_err());
        chown(&path, Some(0), Some(0)).unwrap();
        fs::hard_link(&path, root.0.join("alias")).unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn actual_file_bound_accepts4096_and_refuses4097() {
        let root = Directory::new();
        let mut body = serde_json::to_vec(&wireless()).unwrap();
        body.resize(4096, b' ');
        let exact = root.file("exact", &body);
        assert!(
            load(&exact).is_ok(),
            "4096-byte protected JSON is within the bound"
        );
        body.push(b' ');
        let oversized = root.file("oversized", &body);
        assert!(load(&oversized).is_err());
    }

    #[test]
    fn relative_missing_and_plain_namespace_inputs_are_refused() {
        assert!(load(Path::new("relative.json")).is_err());
        assert!(load(Path::new("")).is_err());
        let root = Directory::new();
        assert!(load(&root.0.join("absent")).is_err());
        assert!(load(&root.file("old-input", b"/proc/1/ns/net")).is_err());
        assert!(load(&root.file("empty", b"")).is_err());
    }

    #[test]
    fn malformed_missing_unknown_and_cross_backend_fields_are_refused() {
        let root = Directory::new();
        let mut unknown = wireless();
        unknown["fallback"] = json!(true);
        let mut target_unknown = wireless();
        target_unknown["target"]["lifetime"] = json!(1);
        let mut namespace_unknown = wireless();
        namespace_unknown["target"]["namespace"]["path"] = json!("/proc/self/ns/net");
        let mut missing = wireless();
        missing.as_object_mut().unwrap().remove("backend");
        let cases = [
            json!({}),
            json!(null),
            json!([]),
            unknown,
            target_unknown,
            namespace_unknown,
            missing,
            json!({"backend":"virtual_pair","peer_namespace":"/proc/self/ns/net","target":{}}),
            json!({"backend":"implicit_virtual","peer_namespace":"/proc/self/ns/net"}),
        ];
        for (number, value) in cases.iter().enumerate() {
            assert!(load(&root.json(&number.to_string(), value)).is_err());
        }
        assert!(load(&root.file("malformed", b"{not JSON}")).is_err());
        let mut missing_ssid = wireless();
        missing_ssid.as_object_mut().unwrap().remove("ssid");
        assert!(load(&root.json("missing-ssid", &missing_ssid)).is_err());
    }

    #[test]
    fn duplicate_backend_target_namespace_and_selector_fields_are_refused() {
        let root = Directory::new();
        let value = wireless();
        let body = serde_json::to_string(&value).unwrap();
        let dev = format!("\"dev\":{}", value["target"]["namespace"]["dev"]);
        for (number, duplicate) in [
            body.replacen(
                "\"backend\":",
                "\"backend\":\"wireless_open\",\"backend\":",
                1,
            ),
            body.replacen("\"target\":", "\"target\":{},\"target\":", 1),
            body.replacen("\"ifindex\":2", "\"ifindex\":2,\"ifindex\":2", 1),
            body.replacen(&dev, &format!("{dev},{dev}"), 1),
        ]
        .iter()
        .enumerate()
        {
            assert!(load(&root.file(&number.to_string(), duplicate.as_bytes())).is_err());
        }
    }

    #[test]
    fn invalid_wireless_selectors_are_refused_at_load_and_validate() {
        let root = Directory::new();
        let mut cases = Vec::new();
        for (key, invalid) in [
            ("ifindex", json!(0)),
            ("ifindex", json!(1)),
            ("name", json!("lo")),
            ("name", json!("")),
            ("name", json!("bad/name")),
            ("name", json!("abcdefghijklmnop")),
            ("name", json!("wlan\u{0}")),
        ] {
            let mut value = wireless();
            value["target"][key] = invalid;
            cases.push(value);
        }
        for (key, invalid) in [
            ("frequency_mhz", json!(0)),
            ("frequency_mhz", json!(100001)),
            ("bssid", json!(vec![0; 6])),
            ("bssid", json!([1, 0, 0, 0, 0, 1])),
            ("ssid", json!([])),
            ("ssid", json!(vec![0; 33])),
        ] {
            let mut value = wireless();
            value[key] = invalid;
            cases.push(value);
        }
        let mut zero_inode = wireless();
        zero_inode["target"]["namespace"]["ino"] = json!(0);
        cases.push(zero_inode);
        for (number, value) in cases.iter().enumerate() {
            assert!(load(&root.json(&number.to_string(), value)).is_err());
            let profile: Profile = serde_json::from_value(value.clone()).unwrap();
            assert!(
                profile.validate().is_err(),
                "worker decoded profiles require the same semantic checks"
            );
        }
    }

    #[test]
    fn type_width_byte_arrays_and_relative_peer_paths_are_refused() {
        let root = Directory::new();
        for (number, (pointer, invalid)) in [
            ("/target/ifindex", json!(-1)),
            ("/target/ifindex", json!(4294967296_u64)),
            ("/target/namespace/ino", json!(-1)),
            ("/frequency_mhz", json!(2412.5)),
            ("/bssid", json!([2, 0, 0, 0, 1])),
            ("/bssid", json!([2, 0, 0, 0, 0, 256])),
            ("/ssid", json!([256])),
            ("/ssid", json!("pkl")),
        ]
        .iter()
        .enumerate()
        {
            let mut value = wireless();
            *value.pointer_mut(pointer).unwrap() = invalid.clone();
            assert!(load(&root.json(&number.to_string(), &value)).is_err());
        }
        for (number, path) in ["", "relative/ns", "../ns", "relative\u{0}"]
            .iter()
            .enumerate()
        {
            let profile = Profile::VirtualPair {
                peer_namespace: PathBuf::from(*path),
            };
            assert!(profile.validate().is_err());
            assert!(
                load(&root.json(
                    &format!("peer-{number}"),
                    &serde_json::to_value(profile).unwrap()
                ))
                .is_err()
            );
        }
        let profile = Profile::VirtualPair {
            peer_namespace: PathBuf::from("/absolute\u{0}path"),
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn validation_preserves_raw_ssid_bytes_and_namespace_selector() {
        let mut value = wireless();
        value["ssid"] = json!([0, 255]);
        value["target"]["namespace"]["dev"] = json!(0);
        value["target"]["name"] = json!("wlan_1.a-2");
        let profile: Profile = serde_json::from_value(value.clone()).unwrap();
        profile
            .validate()
            .expect("valid raw bytes and syntactic namespace selector");
        assert_eq!(serde_json::to_value(profile).unwrap(), value);
    }
}
