use pokelinux_radio::{
    assembly::{Assembly, AssemblyError, Limits},
    indication::{IndicationKind, RadioIndication},
};
fn part(index: i32, total: i32, bytes: &[u8]) -> RadioIndication {
    RadioIndication {
        message_id: 33,
        slot_id: 0,
        payload: bytes.to_vec(),
        kind: IndicationKind::Segment {
            index,
            total_length: total,
        },
    }
}
fn new(total: i32) -> Assembly {
    Assembly::new(
        33,
        0,
        total,
        Limits {
            bytes: 8,
            fragments: 3,
        },
    )
    .unwrap()
}
#[test]
fn original_valid_ordering_and_duplicate_cases_match() {
    for reverse in [false, true] {
        let mut a = new(3);
        let (first, last) = if reverse {
            (part(1, 3, &[2, 3]), part(0, 3, &[1]))
        } else {
            (part(0, 3, &[1]), part(1, 3, &[2, 3]))
        };
        assert_eq!(a.push(&first), Ok(None));
        assert_eq!(a.push(&first), Ok(None));
        assert_eq!(a.push(&last), Ok(Some(vec![1, 2, 3])));
        assert_eq!(a.push(&last), Err(AssemblyError::Finished));
    }
}
#[test]
fn conflicts_are_terminal_and_never_pad_or_truncate() {
    for (bad, reason) in [
        (part(0, 3, &[9]), AssemblyError::Duplicate),
        (part(1, 5, &[2, 3]), AssemblyError::Total),
        (part(1, 1, &[2, 3]), AssemblyError::Total),
    ] {
        let mut a = new(3);
        a.push(&part(0, 3, &[1])).unwrap();
        assert_eq!(a.push(&bad), Err(reason));
        assert_eq!(a.push(&part(1, 3, &[2, 3])), Err(reason));
    }
}
#[test]
fn gaps_overshoot_and_invalid_indices_fail_without_panics() {
    let mut a = new(2);
    a.push(&part(0, 2, &[1])).unwrap();
    assert_eq!(a.push(&part(2, 2, &[2])), Err(AssemblyError::Gap));
    assert_eq!(
        new(1).push(&part(0, 1, &[1, 2])),
        Err(AssemblyError::Length)
    );
    for index in [-1, 3, i32::MAX] {
        assert_eq!(
            new(2).push(&part(index, 2, &[1])),
            Err(AssemblyError::Index)
        );
    }
}
#[test]
fn bounds_and_routing_are_explicit() {
    assert!(
        Assembly::new(
            33,
            0,
            9,
            Limits {
                bytes: 8,
                fragments: 3
            }
        )
        .is_err()
    );
    assert!(
        Assembly::new(
            33,
            0,
            -1,
            Limits {
                bytes: 8,
                fragments: 3
            }
        )
        .is_err()
    );
    let mut p = part(0, 2, &[1]);
    p.slot_id = 1;
    assert_eq!(new(2).push(&p), Err(AssemblyError::Routing));
    p.slot_id = 0;
    p.message_id = 34;
    assert_eq!(new(2).push(&p), Err(AssemblyError::Routing));
    assert_eq!(new(0).push(&part(0, 0, &[])), Ok(Some(vec![])));
    assert_eq!(new(2).push(&part(0, 2, &[])), Err(AssemblyError::Length));
}
