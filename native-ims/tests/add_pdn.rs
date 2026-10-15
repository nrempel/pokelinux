//! Expected bytes come from the qualified original getBytes run, not this encoder.
use omarchy_ims::add_pdn::{AddPdn, EncodeError, WireTime, encode};
use std::net::{Ipv4Addr, Ipv6Addr};
fn config<'a>(mode: u8, ipv4: &'a [Ipv4Addr], ipv6: &'a [Ipv6Addr], absent: bool) -> AddPdn<'a> {
    AddPdn {
        pdn_type_wire: 0,
        ip_type_wire: 3,
        pdn_ipv4: Ipv4Addr::new(192, 0, 2, 10),
        pdn_ipv6: "2001:db8::10".parse().unwrap(),
        hd_enabled_wire: 1,
        operator_id_wire: 7,
        registration_style_wire: 2,
        new_pdn_wire: 1,
        volte_auto_config_wire: 0,
        rcs_auto_config_wire: 1,
        offering_type_wire: 0x1234,
        registration_type_wire: 4,
        init_type_wire: 0,
        ims_net_type_wire: mode,
        mac: if absent {
            None
        } else {
            Some([2, 0, 0, 0, 0, 1])
        },
        pcscf_ipv4: ipv4,
        pcscf_ipv6: ipv6,
        primary_dns_ipv4: Ipv4Addr::new(192, 0, 2, 53),
        primary_dns_ipv6: "2001:db8::53".parse().unwrap(),
        secondary_dns_ipv4: Ipv4Addr::new(198, 51, 100, 53),
        secondary_dns_ipv6: "2001:db8:1::53".parse().unwrap(),
        last_access_network_wire: 7,
        last_access_time: if absent {
            None
        } else {
            Some(WireTime {
                year: 2020,
                month: 1,
                day: 2,
                hour: 3,
                minute: 4,
                second: 5,
                utc_offset_minutes: -210,
            })
        },
        time: if absent {
            None
        } else {
            Some(WireTime {
                year: 2024,
                month: 2,
                day: 29,
                hour: 23,
                minute: 58,
                second: 59,
                utc_offset_minutes: 330,
            })
        },
        tac_or_lac_wire: 0x12345678,
        cell_id32_wire: 0x01020304,
        remaining_timer_wire: 0x01020304,
        mcc_mnc: if absent { None } else { Some("310260") },
        country: if absent { None } else { Some("US") },
        mtu_wire: 1500,
        rcs_capability_wire: 0x10203040,
        vendor_capability_wire: 0x1234,
        extended_rcs_capability_wire: 0x55667788,
        e911_aid: if absent { None } else { Some("ORACLE") },
        band_wire: 0x01020304,
        cell_id64_wire: 0x0102030405060708,
        network_interface_index_wire: 0x00010203,
        sip_delegate_capability_wire: 0x1112131415161718,
    }
}
fn original(id: &str, refusal: Option<EncodeError>) {
    let fields: Vec<_> = include_str!("fixtures/shannon-add-pdn.tsv")
        .lines()
        .find(|line| line.split('\t').next() == Some(id))
        .unwrap()
        .split('\t')
        .collect();
    assert_eq!(fields.len(), 6);
    let ipv4: Vec<_> = (0..fields[2].parse::<u8>().unwrap())
        .map(|i| Ipv4Addr::new(192, 0, 2, 100 + i))
        .collect();
    let ipv6: Vec<_> = (0..fields[3].parse::<u16>().unwrap())
        .map(|i| Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 100 + i))
        .collect();
    let actual = encode(&config(
        fields[1].parse().unwrap(),
        &ipv4,
        &ipv6,
        fields[4] == "1",
    ));
    if let Some(error) = refusal {
        assert_eq!(actual, Err(error));
        return;
    }
    let expected: Vec<_> = fields[5]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u8::from_str_radix(std::str::from_utf8(bytes).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(
        actual,
        Ok(expected),
        "{id}: complete original bytes including padding"
    );
}
#[test]
fn original_mixed_addresses_wire0() {
    original("numeric-v4-v6-pcscf1-wire0-fixed-time", None)
}
#[test]
fn original_empty_pcscf_wire2_fixed_time() {
    original("empty-pcscf-wire2-fixed-time", None)
}
#[test]
fn original_empty_pcscf_wire2_null_optionals() {
    original("empty-pcscf-wire2-null-optionals", None)
}
#[test]
fn original_ipv4_count8() {
    original("pcscf-v4-count8", None)
}
#[test]
fn original_ipv6_count8() {
    original("pcscf-v6-count8", None)
}
#[test]
fn native_refuses_original_ipv4_count9_instead_of_clamping() {
    original("pcscf-v4-count9", Some(EncodeError::TooManyIpv4))
}
#[test]
fn native_refuses_original_ipv6_count9() {
    original("pcscf-v6-count9", Some(EncodeError::TooManyIpv6))
}
#[test]
fn rejects_unqualified_selectors() {
    for mode in [1, 3, 255] {
        assert_eq!(
            encode(&config(mode, &[], &[], false)),
            Err(EncodeError::UnsupportedSelector)
        );
    }
}
#[test]
fn rejects_noncanonical_mcc_mnc() {
    for value in ["31026", "3102600", "31a260", "é0123"] {
        let mut c = config(2, &[], &[], false);
        c.mcc_mnc = Some(value);
        assert_eq!(encode(&c), Err(EncodeError::InvalidMccMnc));
    }
}
#[test]
fn rejects_oversized_or_nonascii_fixed_text() {
    for value in ["USA", "é"] {
        let mut c = config(2, &[], &[], false);
        c.country = Some(value);
        assert_eq!(encode(&c), Err(EncodeError::InvalidCountry));
    }
    for value in ["012345678901234567890123456789012", "é"] {
        let mut c = config(2, &[], &[], false);
        c.e911_aid = Some(value);
        assert_eq!(encode(&c), Err(EncodeError::InvalidE911Aid));
    }
}
#[test]
fn refuses_lossy_or_invalid_civil_dates() {
    let valid = config(2, &[], &[], false).time.unwrap();
    for time in [
        WireTime {
            year: 1999,
            ..valid
        },
        WireTime {
            year: 2256,
            ..valid
        },
        WireTime {
            year: 2023,
            ..valid
        },
        WireTime { month: 0, ..valid },
        WireTime { day: 0, ..valid },
        WireTime { hour: 24, ..valid },
        WireTime {
            minute: 60,
            ..valid
        },
        WireTime {
            second: 60,
            ..valid
        },
    ] {
        let mut c = config(2, &[], &[], false);
        c.time = Some(time);
        assert_eq!(encode(&c), Err(EncodeError::InvalidTime));
        c.time = None;
        c.last_access_time = Some(time);
        assert_eq!(encode(&c), Err(EncodeError::InvalidTime));
    }
}
