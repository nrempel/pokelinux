use pokelinux_messaging::{Concatenation, Encoding, encode_part};
fn unhex(value: &str) -> Vec<u8> {
    if value == "-" {
        return Vec::new();
    }
    assert_eq!(value.len() % 2, 0);
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn native_tpdu_matches_actual_aosp_encoder_bytes() {
    let mut count = 0;
    for line in include_str!("fixtures/aosp-submit.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 8);
        assert_eq!(fields[0], "sms-submit");
        assert_eq!(fields[7], "PASS");
        let expected = unhex(fields[6]);
        let bcd_bytes = usize::from(expected[2]).div_ceil(2);
        let encoding = match expected[5 + bcd_bytes] {
            0 => Encoding::Gsm7,
            8 => Encoding::Utf16,
            _ => panic!("unexpected reference DCS"),
        };
        let header = unhex(fields[4]);
        let concatenation = if header.is_empty() {
            None
        } else {
            assert_eq!(&header[..2], &[8, 4]);
            assert_eq!(header.len(), 6);
            Some(Concatenation {
                reference: u16::from_be_bytes([header[2], header[3]]),
                total: header[4],
                sequence: header[5],
            })
        };
        let text = String::from_utf8(unhex(fields[5])).unwrap();
        let actual = encode_part(fields[2], &text, encoding, fields[3] == "1", concatenation)
            .unwrap_or_else(|error| panic!("original-code case {}: {error:?}", fields[1]));
        assert_eq!(actual, expected, "original-code case {}", fields[1]);
        count += 1;
    }
    assert_eq!(count, 27);
}
