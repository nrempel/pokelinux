use pokelinux_messaging::status_report::{DecodeError, decode_status_report_hex};
fn text(value: &str) -> Option<String> {
    if value == "-" {
        return None;
    }
    let bytes: Vec<_> = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect();
    Some(String::from_utf8(bytes).unwrap())
}
#[test]
fn native_report_fields_and_optional_text_match_actual_aosp_parsers() {
    let mut compared = 0;
    let mut original_parsed_native_rejections = 0;
    for line in include_str!("fixtures/aosp-status-report.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 16);
        assert_eq!(fields[0], "sms-status-report");
        assert_eq!(fields[15], "PASS");
        if fields[3] != "compare" {
            if fields[4] == "parsed" {
                original_parsed_native_rejections += 1;
            }
            continue;
        }
        assert_eq!(fields[4], "parsed");
        assert_eq!(fields[14], "1");
        let parsed = decode_status_report_hex(fields[2])
            .unwrap_or_else(|error| panic!("original {}: {error:?}", fields[1]));
        assert_eq!(parsed.service_center, text(fields[5]), "SMSC {}", fields[1]);
        assert_eq!(
            parsed.message_reference,
            fields[6].parse::<u8>().unwrap(),
            "MR {}",
            fields[1]
        );
        assert_eq!(
            parsed.recipient,
            text(fields[7]).unwrap(),
            "recipient {}",
            fields[1]
        );
        assert_eq!(
            parsed.service_center_timestamp_millis,
            fields[8].parse::<i64>().unwrap(),
            "SC timestamp {}",
            fields[1]
        );
        // Original full parser consumes/discards TP-DT. This fixture column was
        // executed through its original private timestamp decoder separately.
        assert_eq!(
            parsed.discharge_timestamp_millis,
            fields[9].parse::<i64>().unwrap(),
            "discharge timestamp {}",
            fields[1]
        );
        assert_eq!(
            parsed.status,
            fields[10].parse::<u8>().unwrap(),
            "status {}",
            fields[1]
        );
        let pid = parsed
            .optional
            .as_ref()
            .and_then(|p| p.protocol_identifier)
            .unwrap_or(0);
        let coding = parsed
            .optional
            .as_ref()
            .and_then(|p| p.data_coding_scheme)
            .unwrap_or(0);
        assert_eq!(pid, fields[11].parse::<u8>().unwrap(), "PID {}", fields[1]);
        assert_eq!(
            coding,
            fields[12].parse::<u8>().unwrap(),
            "DCS {}",
            fields[1]
        );
        let body = parsed
            .optional
            .as_ref()
            .and_then(|p| p.user_data.as_ref())
            .map(|p| p.text.as_str());
        assert_eq!(
            body,
            text(fields[13]).as_deref(),
            "optional data {}",
            fields[1]
        );
        compared += 1;
    }
    assert_eq!(compared, 57);
    // Stricter native rejection is deliberate, not an assertion that AOSP also
    // rejects every malformed/calendar/trailing/unsupported representation.
    assert_eq!(original_parsed_native_rejections, 41);
    let oversized = include_str!("fixtures/aosp-status-report.tsv")
        .lines()
        .find(|line| line.starts_with("sms-status-report\ttpdu-165-short-prefix\t"))
        .unwrap();
    let fields: Vec<_> = oversized.split('\t').collect();
    assert_eq!(fields[4], "parsed");
    assert_eq!(
        decode_status_report_hex(fields[2]),
        Err(DecodeError::Malformed)
    );
}
