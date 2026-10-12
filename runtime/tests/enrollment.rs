#![cfg(target_os = "linux")]
use omarchy_phone_core::enrollment::Enrollment;
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::{fs, path::PathBuf};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phone-enrollment-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let identity = rcgen::generate_simple_self_signed(vec!["android.test".into()]).unwrap();
        fs::write(path.join("ca.pem"), identity.cert.pem()).unwrap();
        fs::write(path.join("client.pem"), identity.cert.pem()).unwrap();
        fs::write(
            path.join("client.key"),
            identity.signing_key.serialize_pem(),
        )
        .unwrap();
        let config = serde_json::json!({"endpoint":"127.0.0.1:37179","server_name":"android.test","server_sha256":format!("{:x}",Sha256::digest(identity.cert.der().as_ref()))});
        fs::write(path.join("endpoint.json"), config.to_string()).unwrap();
        for name in ["client.key", "endpoint.json"] {
            fs::set_permissions(path.join(name), fs::Permissions::from_mode(0o600)).unwrap();
        }
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn loads_one_private_enrollment_without_network_or_mutating_files() {
    let f = Fixture::new();
    let before = fs::read(f.0.join("endpoint.json")).unwrap();
    let loaded = Enrollment::load(&f.0).unwrap();
    assert_eq!(loaded.address().to_string(), "127.0.0.1:37179");
    assert_eq!(fs::read(f.0.join("endpoint.json")).unwrap(), before);
}
#[test]
fn rejects_unsafe_permissions_and_symlinks() {
    let f = Fixture::new();
    fs::set_permissions(f.0.join("client.key"), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Enrollment::load(&f.0).is_err());
    fs::set_permissions(f.0.join("client.key"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Enrollment::load(&f.0).is_err());
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
    fs::rename(f.0.join("client.key"), f.0.join("saved.key")).unwrap();
    std::os::unix::fs::symlink("saved.key", f.0.join("client.key")).unwrap();
    assert!(Enrollment::load(&f.0).is_err());
}
#[test]
fn rejects_duplicate_fields_invalid_pins_remote_endpoints_and_excess_data() {
    let f = Fixture::new();
    let path = f.0.join("endpoint.json");
    let original = fs::read_to_string(&path).unwrap();
    for bad in [
        original.replace(
            "\"endpoint\":",
            "\"endpoint\":\"127.0.0.1:1\",\"endpoint\":",
        ),
        original.replace("127.0.0.1:37179", "192.0.2.1:37179"),
        original.replace("127.0.0.1:37179", "localhost:37179"),
        original.replace("127.0.0.1:37179", "127.0.0.1:0"),
        original.replace("\"server_sha256\":\"", "\"server_sha256\":\"G"),
        format!("{original}{{}}"),
        " ".repeat(4097),
    ] {
        fs::write(&path, bad).unwrap();
        assert!(Enrollment::load(&f.0).is_err());
    }
}
#[test]
fn rejects_malformed_oversize_and_mismatched_key_material() {
    let f = Fixture::new();
    let path = f.0.join("client.key");
    for bad in [
        "not a key".to_string(),
        " ".repeat(65537),
        rcgen::KeyPair::generate().unwrap().serialize_pem(),
    ] {
        fs::write(&path, bad).unwrap();
        assert!(Enrollment::load(&f.0).is_err());
    }
}
