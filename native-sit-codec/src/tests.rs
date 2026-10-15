use super::*;
fn original_fixture(text: &str) -> [u8; INITIAL_LTE_SETUP_V3_BYTES] {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(
        compact.len(),
        983 * 2,
        "positive control: complete original packet"
    );
    let bytes: Vec<u8> = (0..compact.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&compact[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(&bytes[2..6], &[0, 6, 0xd7, 3]);
    assert_eq!(
        bytes[245], 2,
        "positive control: original caller discriminator"
    );
    bytes.try_into().unwrap()
}
fn compare(cid: u8, apn: &str, protocol: Protocol, fixture: &str) {
    let expected = original_fixture(fixture);
    let request = InitialLteSetupV3::new(NonZeroU8::new(cid).unwrap(), apn, protocol).unwrap();
    assert_eq!(
        request
            .encode()
            .expect("MISSING_INITIAL_SIT_SETUP_V3_ENCODER"),
        expected
    );
}
#[test]
fn complete_original_v3_ip() {
    compare(
        1,
        "pokelinux.invalid",
        Protocol::Ipv4,
        include_str!("../tests/fixtures/v3-ip.hex"),
    );
}
#[test]
fn complete_original_v3_ipv6() {
    compare(
        9,
        "pokelinux.invalid",
        Protocol::Ipv6,
        include_str!("../tests/fixtures/v3-ipv6.hex"),
    );
}
#[test]
fn complete_original_v3_dual_index0() {
    compare(
        9,
        "pokelinux.invalid",
        Protocol::Ipv4v6,
        include_str!("../tests/fixtures/v3-dual-index0.hex"),
    );
}
#[test]
fn complete_original_v3_dual_index17() {
    compare(
        9,
        "pokelinux.invalid",
        Protocol::Ipv4v6,
        include_str!("../tests/fixtures/v3-dual-index17.hex"),
    );
}
#[test]
fn complete_original_v3_nonip() {
    compare(
        2,
        "pokelinux.invalid",
        Protocol::NonIp,
        include_str!("../tests/fixtures/v3-nonip.hex"),
    );
}
#[test]
fn complete_original_v3_other_apn() {
    compare(
        1,
        "x.invalid",
        Protocol::Ipv4,
        include_str!("../tests/fixtures/v3-other-apn.hex"),
    );
}
#[test]
fn refuses_unbounded_or_unsupported_apn_before_encoding() {
    let cid = NonZeroU8::new(1).unwrap();
    for apn in [
        "".to_string(),
        "a".repeat(100),
        "a\0b".to_string(),
        "upperCASE".to_string(),
        "space name".to_string(),
        "é.invalid".to_string(),
    ] {
        assert_eq!(
            InitialLteSetupV3::new(cid, &apn, Protocol::Ipv4),
            Err(Error::InvalidApn)
        );
    }
}
#[test]
fn original_interface_index_pair_has_identical_complete_packet() {
    assert_eq!(
        original_fixture(include_str!("../tests/fixtures/v3-dual-index0.hex")),
        original_fixture(include_str!("../tests/fixtures/v3-dual-index17.hex"))
    );
}
