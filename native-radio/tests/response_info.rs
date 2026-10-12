use pokelinux_radio::RadioExternalResponseInfo;

#[test]
fn generated_binding_matches_original_android_typed_object() {
    let rows = include_str!("fixtures/response-info.tsv");
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 7);
        assert_eq!(fields[0], "responseInfo");
        let values: Vec<i32> = fields[1..6].iter().map(|s| s.parse().unwrap()).collect();
        let value = RadioExternalResponseInfo {
            slotId: values[0],
            serial: values[1],
            rilcMsgId: values[2],
            length: values[3],
            error: values[4],
        };
        assert_eq!(fields[6].len(), 56);
        let expected: Vec<u8> = fields[6]
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(rsbinder::to_bytes(&value).unwrap(), expected);
        let decoded: RadioExternalResponseInfo = rsbinder::from_bytes(&expected).unwrap();
        assert_eq!(
            [
                decoded.slotId,
                decoded.serial,
                decoded.rilcMsgId,
                decoded.length,
                decoded.error
            ],
            values.as_slice()
        );
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn malformed_object_boundaries_are_rejected() {
    // Presence marker followed by an invalid size prefix.
    for size in [i32::MIN, -1, 0, 1, 2, 3, 24, i32::MAX] {
        let mut bytes = 1_i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&size.to_le_bytes());
        assert!(
            rsbinder::from_bytes::<RadioExternalResponseInfo>(&bytes).is_err(),
            "size {size}"
        );
    }
    // A complete object cannot leave arbitrary bytes outside its declared size.
    let mut bytes = rsbinder::to_bytes(&RadioExternalResponseInfo::default()).unwrap();
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    assert!(rsbinder::from_bytes::<RadioExternalResponseInfo>(&bytes).is_err());
}
