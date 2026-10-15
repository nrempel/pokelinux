use pokelinux_sit_codec::{
    Protocol,
    response::{AddressInfo, BaseSetupResponse, DecodeError},
};
fn bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn v4() -> Vec<u8> {
    bytes(
        include_str!("response-fixtures/v4.txt")
            .lines()
            .nth(1)
            .unwrap(),
    )
}
fn getter<const N: usize>(family: &Option<AddressInfo<N>>) -> [u8; 193] {
    let mut out = [0; 193];
    if let Some(p) = family {
        out[0] = 1;
        out[1..1 + N].copy_from_slice(&p.address);
        for (i, address) in p.dns.iter().enumerate() {
            out[1 + N * (i + 1)..1 + N * (i + 2)].copy_from_slice(address);
        }
        for (i, address) in p.pcscf.iter().enumerate() {
            out[1 + N * (i + 4)..1 + N * (i + 5)].copy_from_slice(address);
        }
    }
    out
}
fn compare(text: &str) {
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    let n: Vec<u32> = lines[0]
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    assert_eq!(n.len(), 6);
    let input = bytes(lines[1]);
    assert_eq!(input.len(), 144);
    let parsed = BaseSetupResponse::parse(&input, n[0]).expect("MISSING_BASE_SIT_RESPONSE_DECODER");
    assert_eq!(parsed.token, n[0]);
    assert_eq!(u32::from(parsed.context.get()), n[1]);
    assert_eq!(u32::from(parsed.reported_activity), n[2]);
    assert_eq!(
        parsed.protocol,
        match n[3] {
            1 => Protocol::Ipv4,
            2 => Protocol::Ipv6,
            3 => Protocol::Ipv4v6,
            5 => Protocol::NonIp,
            _ => panic!("original protocol outside fixed subset"),
        }
    );
    assert_eq!(parsed.reported_mtu_v4, n[4]);
    assert_eq!(u32::from(parsed.opaque_u8_at_131), n[5]);
    assert_eq!(parsed.dns_selector, input[37]);
    assert_eq!(parsed.pcscf_selector, input[78]);
    assert_eq!(
        getter(&parsed.address_info_v4()).as_slice(),
        bytes(lines[2])
    );
    assert_eq!(
        getter(&parsed.address_info_v6()).as_slice(),
        bytes(lines[3])
    );
}
#[test]
fn original_v4() {
    compare(include_str!("response-fixtures/v4.txt"));
}
#[test]
fn original_v6() {
    compare(include_str!("response-fixtures/v6.txt"));
}
#[test]
fn original_dual() {
    compare(include_str!("response-fixtures/dual.txt"));
}
#[test]
fn original_inactive() {
    compare(include_str!("response-fixtures/inactive.txt"));
}
#[test]
fn original_cross_selectors() {
    compare(include_str!("response-fixtures/cross-selectors.txt"));
}
#[test]
fn original_nonip() {
    compare(include_str!("response-fixtures/nonip.txt"));
}
#[test]
fn refuses_every_truncation_and_unqualified_extensions() {
    let valid = v4();
    for length in 0..144 {
        assert_eq!(
            BaseSetupResponse::parse(&valid[..length], 0),
            Err(DecodeError::UnsupportedLength)
        );
    }
    for length in [145, 245, 246, 4096] {
        let mut packet = valid.clone();
        packet.resize(length, 0);
        assert_eq!(
            BaseSetupResponse::parse(&packet, 0),
            Err(DecodeError::UnsupportedLength)
        );
    }
}
#[test]
fn refuses_wrong_packet_type() {
    for value in [0, 2, 255] {
        let mut p = v4();
        p[0] = value;
        assert_eq!(BaseSetupResponse::parse(&p, 0), Err(DecodeError::WrongType));
    }
}
#[test]
fn refuses_unknown_header_bytes() {
    for offset in [1, 11] {
        let mut p = v4();
        p[offset] = 1;
        assert_eq!(
            BaseSetupResponse::parse(&p, 0),
            Err(DecodeError::UnsupportedHeader)
        );
    }
}
#[test]
fn refuses_wrong_message_and_declared_length() {
    let mut p = v4();
    p[2..4].copy_from_slice(&0x601u16.to_le_bytes());
    assert_eq!(
        BaseSetupResponse::parse(&p, 0),
        Err(DecodeError::WrongMessage)
    );
    for length in [0, 12, 143, 145, 65535] {
        let mut p = v4();
        p[4..6].copy_from_slice(&(length as u16).to_le_bytes());
        assert_eq!(
            BaseSetupResponse::parse(&p, 0),
            Err(DecodeError::DeclaredLength)
        );
    }
}
#[test]
fn refuses_unmatched_token() {
    assert_eq!(
        BaseSetupResponse::parse(&v4(), 1),
        Err(DecodeError::TokenMismatch)
    );
    let mut p = v4();
    p[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        BaseSetupResponse::parse(&p, 0),
        Err(DecodeError::TokenMismatch)
    );
}
#[test]
fn reports_raw_header_error_without_success_fields() {
    for error in [1, 2, 32, 33, 255] {
        let mut p = v4();
        p[10] = error;
        assert_eq!(
            BaseSetupResponse::parse(&p, 0),
            Err(DecodeError::HeaderError(error))
        );
    }
}
#[test]
fn reports_raw_unsigned_pdp_status_without_success_fields() {
    for status in [1u16, 32768, 65535] {
        let mut p = v4();
        p[12..14].copy_from_slice(&status.to_le_bytes());
        assert_eq!(
            BaseSetupResponse::parse(&p, 0),
            Err(DecodeError::PdpStatus(status))
        );
    }
}
#[test]
fn refuses_unknown_context_activity_protocol_and_address_selectors() {
    for (offset, value, error) in [
        (14, 0, DecodeError::InvalidContext),
        (15, 3, DecodeError::UnsupportedActivity),
        (15, 255, DecodeError::UnsupportedActivity),
        (16, 0, DecodeError::UnsupportedProtocol),
        (16, 4, DecodeError::UnsupportedProtocol),
        (16, 255, DecodeError::UnsupportedProtocol),
        (37, 4, DecodeError::UnsupportedSelector),
        (78, 255, DecodeError::UnsupportedSelector),
    ] {
        let mut p = v4();
        p[offset] = value;
        assert_eq!(BaseSetupResponse::parse(&p, 0), Err(error));
    }
}
