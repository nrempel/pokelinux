use pokelinux_messaging::{Concatenation, Encoding, Error, encode_part, plan_submit};
const PHONE: &str = "+15550100";
#[test]
fn gsm_extension_is_not_split_from_its_escape() {
    let text = format!("{}^{}", "A".repeat(151), "B".repeat(9));
    let parts = plan_submit(PHONE, &text, false, 0x1234).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].text, "A".repeat(151));
    assert_eq!(parts[1].text, format!("^{}", "B".repeat(9)));
    assert_eq!(
        parts.iter().map(|p| p.text.as_str()).collect::<String>(),
        text
    );
    for (index, part) in parts.iter().enumerate() {
        assert_eq!(part.encoding, Encoding::Gsm7);
        assert_eq!(
            part.concatenation,
            Some(Concatenation {
                reference: 0x1234,
                total: 2,
                sequence: index as u8 + 1
            })
        );
        assert!(part.tpdu.len() < 164);
    }
}
#[test]
fn multipart_utf16_never_splits_a_surrogate_pair_or_changes_encoding() {
    let text = format!("{}😀{}", "中".repeat(65), "Z".repeat(6));
    let parts = plan_submit(PHONE, &text, true, 0x1234).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].text, "中".repeat(65));
    assert_eq!(parts[1].text, format!("😀{}", "Z".repeat(6)));
    assert!(parts.iter().all(|p| p.encoding == Encoding::Utf16));
    let ascii_then_emoji = format!("{}😀", "A".repeat(70));
    assert!(
        plan_submit(PHONE, &ascii_then_emoji, false, 1)
            .unwrap()
            .iter()
            .all(|p| p.encoding == Encoding::Utf16)
    );
}
#[test]
fn single_part_limits_and_first_multipart_transition_are_exact() {
    for (text, count) in [
        ("A".repeat(160), 1),
        ("A".repeat(161), 2),
        ("中".repeat(70), 1),
        ("中".repeat(71), 2),
        ("😀".repeat(35), 1),
        ("😀".repeat(36), 2),
    ] {
        let parts = plan_submit(PHONE, &text, false, 0).unwrap();
        assert_eq!(parts.len(), count);
        assert_eq!(
            parts.iter().map(|p| p.text.as_str()).collect::<String>(),
            text
        );
        assert_eq!(parts[0].concatenation.is_some(), count != 1);
    }
}
#[test]
fn unsafe_recipient_header_and_resource_inputs_are_refused() {
    for recipient in [
        "",
        "+",
        "++123",
        "123;456",
        "123\n456",
        "１２３",
        "+123456789012345678901",
    ] {
        assert_eq!(
            plan_submit(recipient, "hello", false, 1),
            Err(Error::InvalidRecipient)
        );
    }
    assert_eq!(plan_submit(PHONE, "", false, 1), Err(Error::EmptyMessage));
    assert_eq!(
        plan_submit(PHONE, &"A".repeat(16385), false, 1),
        Err(Error::MessageTooLong)
    );
    for header in [
        Concatenation {
            reference: 0,
            total: 0,
            sequence: 0,
        },
        Concatenation {
            reference: 0,
            total: 2,
            sequence: 0,
        },
        Concatenation {
            reference: 0,
            total: 2,
            sequence: 3,
        },
    ] {
        assert_eq!(
            encode_part(PHONE, "a", Encoding::Gsm7, false, Some(header)),
            Err(Error::InvalidConcatenation)
        );
    }
    assert_eq!(
        encode_part(PHONE, &"A".repeat(161), Encoding::Gsm7, false, None),
        Err(Error::PartTooLong)
    );
    assert_eq!(
        encode_part(PHONE, "中", Encoding::Gsm7, false, None),
        Err(Error::Encoding)
    );
}

#[test]
fn exact_multipart_budget_is_admitted_and_next_character_is_refused() {
    // Exercise both the byte ceiling and part ceiling using texts with different
    // unit costs. The accepted boundary must retain the original body exactly.
    for (character, per_part) in [("A", 152), ("^", 76), ("中", 66), ("😀", 33)] {
        let body = character.repeat(per_part * pokelinux_messaging::MAX_PARTS);
        let parts = plan_submit(PHONE, &body, false, 0xffff).unwrap();
        assert_eq!(parts.len(), pokelinux_messaging::MAX_PARTS);
        assert_eq!(
            parts.iter().map(|p| p.text.as_str()).collect::<String>(),
            body
        );
        let last = parts.last().unwrap().concatenation.unwrap();
        assert_eq!(
            (last.reference, last.total, last.sequence),
            (0xffff, 32, 32)
        );
        assert_eq!(
            plan_submit(PHONE, &(body + character), false, 0xffff),
            Err(Error::MessageTooLong)
        );
    }
}
