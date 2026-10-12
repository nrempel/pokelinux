use omarchy_ims::{encode_registration_state, encode_stack_start};
#[test]
fn matches_original_shannon_message_encoders() {
    let mut count = 0;
    for row in include_str!("fixtures/shannon-messages.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let first = fields[1].parse().unwrap();
        let second = fields[2].parse().unwrap();
        let bytes = match fields[0] {
            "RilReqStackStartReq" => encode_stack_start(first, second),
            "RilReqSetImsRegistrationState" => encode_registration_state(first, second),
            _ => panic!("unknown original-code fixture"),
        };
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, fields[3], "{}({first},{second})", fields[0]);
        count += 1;
    }
    assert_eq!(count, 162);
}
