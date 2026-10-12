use omarchy_ims::DataWriter;
#[test]
fn matches_original_factory_shannon_bytecode() {
    let mut count = 0;
    for row in include_str!("fixtures/shannon-data-writer.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 3);
        let value: i32 = fields[1].parse().unwrap();
        let mut writer = DataWriter::default();
        match fields[0] {
            "writeByte" => writer.write_byte(value),
            "writeShort" => writer.write_short(value),
            "writeInt" => writer.write_int(value),
            _ => panic!("unknown original-code fixture"),
        }
        let bytes = writer.into_bytes();
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, fields[2], "{}({value})", fields[0]);
        count += 1;
    }
    assert_eq!(count, 45);
}
