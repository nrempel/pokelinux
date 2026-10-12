use omarchy_ims::reader::{DataReader, MAX_INPUT_BYTES, ReadError};
fn bytes(value: &str) -> Vec<u8> {
    if value == "-" {
        return vec![];
    }
    assert!(value.len().is_multiple_of(2));
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|part| u8::from_str_radix(std::str::from_utf8(part).unwrap(), 16).unwrap())
        .collect()
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn invoke(reader: &mut DataReader<'_>, method: &str, argument: i32) -> Result<String, ReadError> {
    match method {
        "construct" => Ok("-".into()),
        "getByte" => reader.read_i8().map(|v| v.to_string()),
        "getByteAsInt" => reader.read_u8().map(|v| v.to_string()),
        "getShort" => reader.read_i16().map(|v| v.to_string()),
        "getShortAsInt" => reader.read_u16().map(|v| v.to_string()),
        "getInt" => reader.read_i32().map(|v| v.to_string()),
        "getLong" => reader.read_i64().map(|v| v.to_string()),
        "getBytes" => reader.read_bytes(argument).map(hex),
        "skipBytes" => reader.skip_bytes(argument).map(|()| "-".into()),
        "getCpFormatByteLenString" => reader
            .read_cp_byte_field()
            .map(|v| v.map_or_else(|| "-".into(), hex)),
        "getCpFormatShortLenString" => reader
            .read_cp_short_field()
            .map(|v| v.map_or_else(|| "-".into(), hex)),
        _ => panic!("unknown authored oracle method"),
    }
}
#[test]
fn actual_little_endian_scalar_is_decoded() {
    let mut reader = DataReader::new(&[0x78, 0x56, 0x34, 0x12]).unwrap();
    assert_eq!(reader.read_i32(), Ok(0x1234_5678));
    assert_eq!(reader.finish(), Ok(()));
}
#[test]
fn original_synthetic_reader_values_and_remaining_match() {
    let mut matched = 0;
    let mut rejected = 0;
    for row in include_str!("fixtures/shannon-reader.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 10);
        let input = bytes(fields[2]);
        let offset = fields[3].parse().unwrap();
        let length = fields[4].parse().unwrap();
        let constructed = DataReader::from_range(&input, offset, length);
        if fields[9] == "InvalidRange" {
            assert!(
                matches!(constructed, Err(ReadError::InvalidRange)),
                "{}",
                fields[0]
            );
            rejected += 1;
            continue;
        }
        let mut reader = constructed.unwrap();
        let value = invoke(&mut reader, fields[1], fields[5].parse().unwrap());
        match fields[9] {
            "match" => {
                assert_eq!(value, Ok(fields[7].to_string()), "{}", fields[0]);
                assert_eq!(
                    reader.remaining(),
                    fields[8].parse::<usize>().unwrap(),
                    "{}",
                    fields[0]
                );
                matched += 1;
            }
            "Truncated" | "NegativeLength" => {
                let expected = if fields[9] == "Truncated" {
                    ReadError::Truncated
                } else {
                    ReadError::NegativeLength
                };
                assert_eq!(value, Err(expected), "{}", fields[0]);
                assert_eq!(reader.position(), 0, "{} failure changed cursor", fields[0]);
                assert_eq!(
                    reader.read_u8(),
                    Err(expected),
                    "{} failed reader reactivated",
                    fields[0]
                );
                rejected += 1;
            }
            _ => panic!("unknown native policy"),
        }
    }
    assert_eq!((matched, rejected), (148, 96));
}
#[test]
fn signed_unsigned_and_long_boundaries_are_preserved() {
    let mut reader = DataReader::new(&[0xff, 0xff, 0x00, 0x80, 0xff, 0xff]).unwrap();
    assert_eq!(reader.read_i8(), Ok(-1));
    assert_eq!(reader.read_u8(), Ok(255));
    assert_eq!(reader.read_i16(), Ok(i16::MIN));
    assert_eq!(reader.read_u16(), Ok(u16::MAX));
    assert_eq!(reader.finish(), Ok(()));
    for value in [i64::MIN, -1, 0, 1, i64::MAX, 0x0807_0605_0403_0201] {
        let data = value.to_le_bytes();
        let mut reader = DataReader::new(&data).unwrap();
        assert_eq!(reader.read_i64(), Ok(value));
        assert_eq!(reader.finish(), Ok(()));
    }
}
#[test]
fn every_truncated_scalar_is_atomic_and_permanently_fenced() {
    for length in 0..8 {
        let data = [0xff; 8];
        let mut reader = DataReader::from_range(&data, 0, length).unwrap();
        assert_eq!(reader.read_i64(), Err(ReadError::Truncated));
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.remaining(), length as usize);
        assert_eq!(reader.skip_bytes(0), Err(ReadError::Truncated));
        assert_eq!(reader.finish(), Err(ReadError::Truncated));
    }
}
#[test]
fn opaque_cp_fields_never_invent_text_or_strip_nul() {
    let data = [4, 0xc3, 0xa9, 0, 0xff, 3, 0, 65, 0, 66];
    let mut reader = DataReader::new(&data).unwrap();
    assert_eq!(reader.read_cp_byte_field(), Ok(Some(&data[1..5])));
    assert_eq!(reader.read_cp_short_field(), Ok(Some(&data[7..10])));
    assert_eq!(reader.finish(), Ok(()));
}
#[test]
fn zero_cp_field_consumes_only_its_prefix() {
    let mut reader = DataReader::new(&[0, 0, 0, 7]).unwrap();
    assert_eq!(reader.read_cp_byte_field(), Ok(None));
    assert_eq!(reader.remaining(), 3);
    assert_eq!(reader.read_cp_short_field(), Ok(None));
    assert_eq!(reader.read_u8(), Ok(7));
    assert_eq!(reader.finish(), Ok(()));
}
#[test]
fn malformed_cp_field_is_atomic_and_latches_first_error() {
    for data in [&[128, 65][..], &[255][..]] {
        let mut reader = DataReader::new(data).unwrap();
        assert_eq!(reader.read_cp_byte_field(), Err(ReadError::NegativeLength));
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.read_bytes(-1), Err(ReadError::NegativeLength));
    }
    for data in [&[2, 65][..], &[][..]] {
        let mut reader = DataReader::new(data).unwrap();
        assert_eq!(reader.read_cp_byte_field(), Err(ReadError::Truncated));
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.read_bytes(-1), Err(ReadError::Truncated));
    }
    for data in [&[0, 128, 65][..], &[255, 255][..]] {
        let mut reader = DataReader::new(data).unwrap();
        assert_eq!(reader.read_cp_short_field(), Err(ReadError::NegativeLength));
        assert_eq!(reader.position(), 0);
    }
    for data in [&[2, 0, 65][..], &[1][..]] {
        let mut reader = DataReader::new(data).unwrap();
        assert_eq!(reader.read_cp_short_field(), Err(ReadError::Truncated));
        assert_eq!(reader.position(), 0);
    }
}
#[test]
fn counts_cannot_rewind_overflow_or_cross_the_view() {
    for value in [-1, i32::MIN] {
        let mut reader = DataReader::new(&[1, 2]).unwrap();
        assert_eq!(reader.skip_bytes(value), Err(ReadError::NegativeLength));
        assert_eq!(reader.position(), 0);
    }
    for value in [3, i32::MAX] {
        let mut reader = DataReader::new(&[1, 2]).unwrap();
        assert_eq!(reader.read_bytes(value), Err(ReadError::Truncated));
        assert_eq!(reader.position(), 0);
    }
}
#[test]
fn whole_input_and_selected_view_are_bounded() {
    let large = vec![0; MAX_INPUT_BYTES + 1];
    assert!(matches!(
        DataReader::new(&large),
        Err(ReadError::InputTooLarge)
    ));
    assert!(matches!(
        DataReader::from_range(&large, 0, 1),
        Err(ReadError::InputTooLarge)
    ));
    for (offset, length) in [(-1, 1), (0, -1), (1, 2), (i32::MAX, 1), (0, i32::MAX)] {
        assert!(matches!(
            DataReader::from_range(&[1, 2], offset, length),
            Err(ReadError::InvalidRange)
        ));
    }
    let allowed = vec![7; MAX_INPUT_BYTES];
    let mut reader = DataReader::new(&allowed).unwrap();
    assert_eq!(
        reader.read_bytes(MAX_INPUT_BYTES as i32),
        Ok(allowed.as_slice())
    );
    assert_eq!(reader.finish(), Ok(()));
    let mut reader = DataReader::from_range(&[99, 1, 2, 98], 1, 2).unwrap();
    assert_eq!(reader.read_u16(), Ok(513));
    assert_eq!(reader.finish(), Ok(()));
}
#[test]
fn trailing_or_failed_packet_cannot_be_finished_as_valid() {
    assert_eq!(
        DataReader::new(&[1]).unwrap().finish(),
        Err(ReadError::TrailingData)
    );
    assert_eq!(DataReader::new(&[]).unwrap().finish(), Ok(()));
    let mut reader = DataReader::new(&[1]).unwrap();
    assert_eq!(reader.read_i32(), Err(ReadError::Truncated));
    assert_eq!(reader.finish(), Err(ReadError::Truncated));
}
