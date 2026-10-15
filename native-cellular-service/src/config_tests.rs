use super::*;
use crate::expected;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    time::{SystemTime, UNIX_EPOCH},
};
struct Input(PathBuf);
impl Input {
    fn create() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pokelinux-data-config-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, include_bytes!("../tests/fixtures/config.json")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        Self(path)
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}
#[test]
fn protected_file_decodes_complete_setup() {
    let input = Input::create();
    let bytes = fs::read(&input.0).unwrap();
    let metadata = fs::symlink_metadata(&input.0).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.mode() & 0o777, 0o600);
    assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(bytes, include_bytes!("../tests/fixtures/config.json"));
    let wanted: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wanted["schema"], 1);
    println!("CONFIG-CONTROL protected regular file/full public setup fixture ready");
    let config = Config::load(&input.0)
        .unwrap_or_else(|e| panic!("MISSING_NATIVE_DATA_CONFIG_DECODE: {e:?}"));
    assert_eq!(config.device, PathBuf::from("/dev/binderfs/data"));
    assert_eq!(
        config.service,
        "android.hardware.radio.data.IRadioData/slot1"
    );
    assert_eq!(config.expected_callback_uid, 1000);
    let setup = config.setup;
    let actual = json!({"serial":setup.serial,"accessNetwork":setup.access_network.0,
        "dataProfileInfo":expected::profile_json(&setup.profile),"roamingAllowed":setup.roaming_allowed,
        "reason":setup.reason.0,"addresses":expected::addresses_json(&setup.addresses),"dnses":setup.dnses,
        "pduSessionId":setup.pdu_session_id,"sliceInfo":expected::slice_json(setup.slice.as_ref()),
        "matchAllRuleAllowed":setup.match_all_rule_allowed});
    assert_eq!(actual, wanted["request"]);
    println!("CONFIG-COMPLETE all10 arguments/all21 profile fields preserved");
}
#[test]
fn group_readable_config_is_refused() {
    let input = Input::create();
    fs::set_permissions(&input.0, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(Config::load(&input.0).is_err());
}
#[test]
fn symbolic_config_is_refused() {
    let input = Input::create();
    let link = input.0.parent().unwrap().join("link.json");
    std::os::unix::fs::symlink(&input.0, &link).unwrap();
    assert!(Config::load(&link).is_err());
}
#[test]
fn oversized_config_is_refused() {
    let input = Input::create();
    fs::write(&input.0, vec![b' '; 32769]).unwrap();
    assert!(Config::load(&input.0).is_err());
}

// Additional qualification against the already repaired decoder; not a RED.
#[test]
fn valid_oversized_config_is_refused() {
    let input = Input::create();
    assert!(
        Config::load(&input.0).is_ok(),
        "valid protected control must load"
    );
    let original = fs::read(&input.0).unwrap();
    let expected: Value = serde_json::from_slice(&original).unwrap();
    assert!(original.len() < 32769);
    let mut padded = original;
    padded.resize(32769, b' ');
    assert_eq!(serde_json::from_slice::<Value>(&padded).unwrap(), expected);
    fs::write(&input.0, &padded).unwrap();
    let parent = fs::symlink_metadata(input.0.parent().unwrap()).unwrap();
    let file = fs::symlink_metadata(&input.0).unwrap();
    let uid = rustix::process::geteuid().as_raw();
    assert!(parent.is_dir() && file.is_file());
    assert_eq!(parent.uid(), uid);
    assert_eq!(file.uid(), uid);
    assert_eq!(parent.mode() & 0o777, 0o700);
    assert_eq!(file.mode() & 0o777, 0o600);
    assert_eq!(file.nlink(), 1);
    assert_eq!(file.len(), 32769);
    let actual = fs::read(&input.0).unwrap();
    assert_eq!(actual, padded);
    assert_eq!(serde_json::from_slice::<Value>(&actual).unwrap(), expected);
    println!("CONFIG-SIZE-CONTROL valid JSON/protected32769-byte file ready");
    let error = Config::load(&input.0)
        .err()
        .expect("oversized valid JSON accepted");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn worker_refuses_replaced_config_with_same_serial() {
    let input = Input::create();
    let (config, identity) = Config::load_identified(&input.0).unwrap();
    let hex: String = identity.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "c633c4b950556ed14935ff72599e9e6cd4c5d04b811ac13ae7bac5bc3c3fef4e"
    );
    assert_eq!(
        Config::load_matching(&input.0, identity)
            .unwrap()
            .setup
            .serial,
        config.setup.serial
    );
    let mut replacement: Value = serde_json::from_slice(&fs::read(&input.0).unwrap()).unwrap();
    replacement["request"]["dataProfileInfo"]["apn"] = json!("replaced.invalid");
    fs::write(&input.0, serde_json::to_vec(&replacement).unwrap()).unwrap();
    let actual = Config::load(&input.0).unwrap();
    assert_eq!(actual.setup.serial, config.setup.serial);
    assert_eq!(actual.setup.profile.apn, "replaced.invalid");
    let (_, replacement_identity) = Config::load_identified(&input.0).unwrap();
    assert_ne!(replacement_identity, identity);
    let metadata = fs::symlink_metadata(&input.0).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
    assert_eq!(metadata.mode() & 0o777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    println!(
        "CONFIG-BINDING-CONTROL same serial/valid protected replacement, actual content identity differs"
    );
    assert!(
        Config::load_matching(&input.0, identity).is_err(),
        "MISSING_PARENT_WORKER_CONFIG_IDENTITY_FENCE"
    );
}
