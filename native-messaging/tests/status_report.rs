use pokelinux_messaging::status_report::{
    DecodeError, MAX_PARAMETER_INDICATORS, MAX_PDU_BYTES, ReportQualifier, decode_status_report,
    decode_status_report_hex,
};
fn base() -> Vec<u8> {
    // Empty SMSC, report/ref255, +15550100, 2026-10-12 13:14:15 UTC,
    // discharge 2026-10-12 13:14:16 UTC, raw status0.
    vec![
        0, 2, 255, 8, 0x91, 0x51, 0x55, 0x10, 0, 0x62, 0x01, 0x21, 0x31, 0x41, 0x51, 0, 0x62, 0x01,
        0x21, 0x31, 0x41, 0x61, 0, 0,
    ]
}
#[test]
fn full_numeric_report_metadata_is_bounded_and_separate_from_delivery_authority() {
    let report = decode_status_report(&base()).expect("bounded GSM status-report decoding missing");
    assert_eq!(report.service_center, None);
    assert_eq!(report.message_reference, 255);
    assert_eq!(report.recipient, "+15550100");
    assert_eq!(report.status, 0);
    assert_eq!(report.qualifier, ReportQualifier::Submit);
    assert!(report.more_messages);
    assert!(!report.loop_prevention);
    assert_eq!(
        report.discharge_timestamp_millis - report.service_center_timestamp_millis,
        1000
    );
    assert!(report.optional.is_none());
}
#[test]
fn every_required_truncation_and_oversize_input_is_rejected() {
    let pdu = base();
    for length in 0..pdu.len() {
        assert_eq!(
            decode_status_report(&pdu[..length]),
            Err(DecodeError::Malformed),
            "cut {length}"
        );
    }
    assert_eq!(
        decode_status_report(&[0; MAX_PDU_BYTES + 1]),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn transport_hex_requires_bounded_whole_ascii_octets() {
    for hex in ["", "0", "0002f", "gg", " 00", "00\n", "é", "０２"] {
        assert_eq!(
            decode_status_report_hex(hex),
            Err(DecodeError::Malformed),
            "{hex:?}"
        );
    }
    assert_eq!(
        decode_status_report_hex(&"00".repeat(MAX_PDU_BYTES + 1)),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn unsupported_parameter_extensions_are_never_silently_dropped() {
    let mut pdu = base();
    pdu.extend([0x80, 1]);
    assert_eq!(decode_status_report(&pdu), Err(DecodeError::Unsupported));
    let mut pdu = base();
    pdu.extend(vec![0x80; MAX_PARAMETER_INDICATORS]);
    pdu.push(0);
    assert_eq!(decode_status_report(&pdu), Err(DecodeError::Malformed));
}

#[test]
fn all_authored_fixture_acceptance_and_rejection_classes_are_enforced() {
    let mut counts = [0; 3];
    for line in include_str!("fixtures/status-report-input.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3);
        let result = decode_status_report_hex(fields[1]);
        match fields[2] {
            "compare" => {
                result.unwrap_or_else(|error| panic!("{}: {error:?}", fields[0]));
                counts[0] += 1;
            }
            "malformed" => {
                assert_eq!(result, Err(DecodeError::Malformed), "{}", fields[0]);
                counts[1] += 1;
            }
            "unsupported" => {
                assert_eq!(result, Err(DecodeError::Unsupported), "{}", fields[0]);
                counts[2] += 1;
            }
            _ => panic!("unknown fixture policy"),
        }
    }
    assert_eq!(counts, [57, 61, 8]);
}
#[test]
fn absent_optional_fields_stay_absent_and_empty_data_stays_present() {
    let mut pdu = base();
    pdu.push(0);
    let value = decode_status_report(&pdu).unwrap().optional.unwrap();
    assert!(value.protocol_identifier.is_none());
    assert!(value.data_coding_scheme.is_none());
    assert!(value.user_data.is_none());
    pdu.pop();
    pdu.extend([4, 0]);
    let value = decode_status_report(&pdu).unwrap().optional.unwrap();
    assert!(value.protocol_identifier.is_none());
    assert!(value.data_coding_scheme.is_none());
    assert_eq!(value.user_data.unwrap().text, "");
}
#[test]
fn optional_required_values_and_user_data_may_not_be_truncated() {
    let mut pdu = base();
    pdu.extend([7, 0, 8, 4, 0, 65, 0, 66]);
    for length in base().len() + 1..pdu.len() {
        assert_eq!(
            decode_status_report(&pdu[..length]),
            Err(DecodeError::Malformed),
            "cut {length}"
        );
    }
    assert_eq!(
        decode_status_report(&pdu)
            .unwrap()
            .optional
            .unwrap()
            .user_data
            .unwrap()
            .text,
        "AB"
    );
    pdu.push(0);
    assert_eq!(decode_status_report(&pdu), Err(DecodeError::Malformed));
}
#[test]
fn hexadecimal_case_changes_no_metadata_and_never_trims() {
    let lower: String = base().iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        decode_status_report_hex(&lower.to_uppercase()),
        decode_status_report_hex(&lower)
    );
    assert_eq!(
        decode_status_report_hex(&format!("{lower} ")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_status_report_hex(&format!(" {lower}")),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn the_complete_supported_maximum_is_accepted_and_one_extra_byte_is_rejected() {
    let line = include_str!("fixtures/status-report-input.tsv")
        .lines()
        .find(|line| line.starts_with("max-complete-bound\t"))
        .unwrap();
    let hex = line.split('\t').nth(1).unwrap();
    assert_eq!(hex.len(), MAX_PDU_BYTES * 2);
    let report = decode_status_report_hex(hex).unwrap();
    assert_eq!(report.recipient.len(), 2);
    assert_eq!(report.service_center.unwrap().len(), 23);
    assert_eq!(
        report.optional.unwrap().user_data.unwrap().text,
        " ".repeat(70)
    );
    assert_eq!(
        decode_status_report_hex(&(hex.to_owned() + "00")),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn single_byte_mutations_keep_decoded_fields_bounded_without_panics() {
    for index in 0..base().len() {
        for byte in 0..=255 {
            let mut pdu = base();
            pdu[index] = byte;
            if let Ok(report) = decode_status_report(&pdu) {
                assert!(report.recipient.len() <= 21);
                assert!(
                    report
                        .service_center
                        .is_none_or(|center| center.len() <= 23)
                );
                if let Some(data) = report.optional.and_then(|p| p.user_data) {
                    assert!(data.text.len() <= 640);
                }
            }
        }
    }
}
#[test]
fn reference_zero_collisions_remain_distinct_untrusted_reports() {
    let mut pdu = base();
    pdu[2] = 0;
    let first = decode_status_report(&pdu).unwrap();
    pdu[23] = 64;
    let second = decode_status_report(&pdu).unwrap();
    assert_eq!(first.message_reference, 0);
    assert_eq!(second.message_reference, 0);
    assert_ne!(first, second);
    assert_eq!(first.status, 0);
    assert_eq!(second.status, 64);
}

#[test]
fn short_smsc_prefix_cannot_hide_a_tpdu_over_the_report_budget() {
    let mut pdu = vec![0, 2, 255, 3, 0x91, 0x21, 0xf3];
    pdu.extend(&base()[9..]);
    pdu.extend([7, 0, 8, 140]);
    pdu.extend([0, 32].repeat(70));
    assert_eq!(pdu.len(), 166);
    assert_eq!(
        decode_status_report(&pdu),
        Err(DecodeError::Malformed),
        "well-framed 165-byte TPDU must not escape through short SMSC prefix"
    );
}
