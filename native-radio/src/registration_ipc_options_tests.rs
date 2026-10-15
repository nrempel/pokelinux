//! Finite complete-tail option combinations retain None separately from Some(0).
use super::*;
#[test]
fn every_valid_zero_option_combination_roundtrips_without_inventing_present_fields() {
    for flags in [0u8, 1, 3, 4, 5, 7] {
        let wire = crate::registration::RegistrationWire::new(
            omarchy_ims::registration::RegistrationWire {
                registration_state_wire: 255,
                feature_wire: 128,
                feature_mask_wire: i32::MIN,
                status_code_wire: u16::MAX,
                uri_bytes: &[],
                reason_bytes: &[],
                feature_mask_ext_wire: (flags & 1 != 0).then_some(0),
                sip_delegate_mask_wire: (flags & 2 != 0).then_some(0),
                feature_ext_wire: (flags & 4 != 0).then_some(0),
            },
        )
        .unwrap();
        let frame = Frame {
            identity: ProcessIdentity::from_bytes([0x12; 16]),
            attempt: 1,
            revision: 9,
            kind: Kind::RegistrationWire {
                slot_id: 1,
                message_id: 3012,
                wire,
            },
        };
        let bytes = frame.encode().unwrap();
        assert_eq!(bytes[HEADER_LEN + 20], flags);
        assert_eq!(decode_exact(&bytes).unwrap(), frame);
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).unwrap().is_none());
            assert_eq!(
                decode_exact(&bytes[..end]).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
        for invalid in [2, 6] {
            let mut bytes = bytes.clone();
            bytes[HEADER_LEN + 20] = invalid;
            assert!(decode_exact(&bytes).is_err());
        }
    }
}
