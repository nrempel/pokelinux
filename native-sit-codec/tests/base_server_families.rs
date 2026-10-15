use pokelinux_sit_codec::response::BaseSetupResponse;
use std::net::Ipv6Addr;

#[test]
fn original_report_preserves_servers_without_the_local_address_family() {
    let text = include_str!("response-fixtures/cross-selectors.txt");
    let hex = text.lines().nth(1).unwrap();
    let packet: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let parsed = BaseSetupResponse::parse(&packet, 0).unwrap();
    assert!(parsed.ipv4.is_some());
    assert!(parsed.ipv6.is_none());
    assert!(parsed.address_info_v6().is_none());
    // Captured original modern report: IPv4 local address, independent IPv6 servers.
    let dns = ["2001:db8::53", "2001:db8:1::53"].map(|s| s.parse::<Ipv6Addr>().unwrap().octets());
    let pcscf = ["2001:db8:2::1", "2001:db8:2::2", "2001:db8:2::3"]
        .map(|s| s.parse::<Ipv6Addr>().unwrap().octets());
    assert_eq!(parsed.dns.ipv6, Some(dns), "LOST_INDEPENDENT_SERVER_FAMILY");
    assert_eq!(parsed.pcscf.ipv6, Some(pcscf));
    assert_eq!(parsed.dns.ipv4, None);
    assert_eq!(parsed.pcscf.ipv4, None);
}
