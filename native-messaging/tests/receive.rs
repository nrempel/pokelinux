use pokelinux_messaging::receive::{ReceiveError, decode_delivery};
fn unhex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}
fn hello() -> Vec<u8> {
    unhex("000008915155100000006201213141510005e8329bfd06")
}
#[test]
fn plain_text_sender_timestamp_and_reply_path_are_observed() {
    let mut pdu = hello();
    let received = decode_delivery(&pdu).unwrap();
    assert_eq!(received.sender, "+15550100");
    assert_eq!(received.text, "hello");
    assert_eq!(received.timestamp_millis, 1791810855000);
    assert!(!received.reply_path);
    assert_eq!(received.concatenation, None);
    pdu[1] |= 0x80;
    assert!(decode_delivery(&pdu).unwrap().reply_path);
}
#[test]
fn truncated_and_oversized_payloads_never_panic_or_produce_text() {
    let good = hello();
    for end in 0..good.len() {
        assert!(decode_delivery(&good[..end]).is_err(), "prefix {end}");
    }
    let mut extra = good;
    extra.push(0);
    assert!(decode_delivery(&extra).is_err());
    assert!(decode_delivery(&[0; 177]).is_err());
}
#[test]
fn unsupported_data_and_invalid_lengths_are_explicit() {
    let mut pdu = hello();
    pdu[9] = 4;
    assert!(matches!(
        decode_delivery(&pdu),
        Err(ReceiveError::Unsupported)
    ));
    let mut pdu = hello();
    pdu[2] = 255;
    assert!(decode_delivery(&pdu).is_err());
    let mut pdu = hello();
    pdu[11] = 0x31;
    assert!(decode_delivery(&pdu).is_err());
    let mut pdu = hello();
    pdu[1] |= 0x40;
    assert!(decode_delivery(&pdu).is_err());
}

#[test]
fn invalid_utf16_escape_and_concatenation_do_not_produce_text() {
    let mut pdu = hello();
    pdu[9] = 8;
    pdu[17] = 2;
    pdu.truncate(18);
    pdu.extend([0xd8, 0]);
    assert!(decode_delivery(&pdu).is_err());
    let mut pdu = hello();
    pdu[17] = 1;
    pdu.truncate(18);
    pdu.push(0x1b);
    assert!(decode_delivery(&pdu).is_err());
    let fixture = include_str!("fixtures/aosp-deliver.tsv")
        .lines()
        .find(|line| line.starts_with("sms-deliver\tconcat-gsm-first\t"))
        .unwrap();
    let pdu = unhex(fixture.split('\t').nth(2).unwrap());
    for (position, value) in [(23, 0), (24, 0), (24, 3)] {
        let mut corrupt = pdu.clone();
        corrupt[position] = value;
        assert!(decode_delivery(&corrupt).is_err());
    }
}
#[test]
fn arbitrary_bounded_inputs_and_byte_mutations_cannot_panic() {
    let good = hello();
    for position in 0..good.len() {
        for value in 0..=255 {
            let mut bytes = good.clone();
            bytes[position] = value;
            let _ = decode_delivery(&bytes);
        }
    }
    let mut seed = 0xace1u32;
    for length in 0..=176 {
        for _ in 0..16 {
            let mut bytes = vec![0; length];
            for byte in &mut bytes {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                *byte = seed as u8;
            }
            let _ = decode_delivery(&bytes);
        }
    }
}
