use pokelinux_messaging::receive::{ReferenceWidth, decode_delivery};
fn unhex(value: &str) -> Vec<u8> {
    if value == "-" {
        return Vec::new();
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn native_incoming_text_metadata_matches_actual_aosp_decoder() {
    let mut count = 0;
    for line in include_str!("fixtures/aosp-deliver.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 9);
        assert_eq!(fields[0], "sms-deliver");
        assert_eq!(fields[8], "PASS");
        let parsed = decode_delivery(&unhex(fields[2]))
            .unwrap_or_else(|error| panic!("original case {}: {error:?}", fields[1]));
        assert_eq!(
            parsed.sender.as_bytes(),
            unhex(fields[3]),
            "sender: {}",
            fields[1]
        );
        assert_eq!(
            parsed.text.as_bytes(),
            unhex(fields[4]),
            "text: {}",
            fields[1]
        );
        assert_eq!(
            parsed.timestamp_millis,
            fields[5].parse::<i64>().unwrap(),
            "timestamp: {}",
            fields[1]
        );
        assert_eq!(parsed.reply_path, fields[7] == "1");
        if fields[6] == "-" {
            assert!(parsed.concatenation.is_none());
        } else {
            let values: Vec<_> = fields[6]
                .split(':')
                .map(|v| v.parse::<u16>().unwrap())
                .collect();
            let concat = parsed.concatenation.unwrap();
            assert_eq!(
                [
                    if concat.width == ReferenceWidth::Eight {
                        8
                    } else {
                        16
                    },
                    concat.reference,
                    u16::from(concat.total),
                    u16::from(concat.sequence)
                ],
                values.as_slice(),
                "concat: {}",
                fields[1]
            );
        }
        count += 1;
    }
    assert_eq!(count, 33);
}
