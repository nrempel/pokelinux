//! Fixed captured-original comparisons for the bounded modern-only projection.
use pokelinux_sit_codec::{
    modern_report::{ModernSetupReport22, ReportError},
    response::BaseSetupResponse,
};

const TABLE: &str = include_str!("modern-report-fixtures/v22.tsv");
const HEADER: &str = "case_id\tinput144_hex\texpected_token\tconstructor_context_cid\tpresentation_index\tcause\topaque_i64_at_8\tcid\tactive\tprotocol_code\tmtu_v4\tmtu_v6\tifname\taddress_text\tdnses\tgateways\tpcscf\tstructured_address_vector_empty\tqos_td_vectors_empty";

fn octets(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    assert!(
        text.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(pair, 16).unwrap()
        })
        .collect()
}

fn fields(case: &str) -> Vec<&'static str> {
    let mut lines = TABLE.lines();
    assert_eq!(lines.next(), Some(HEADER));
    let rows: Vec<_> = lines
        .filter(|line| line.split('\t').next() == Some(case))
        .collect();
    assert_eq!(rows.len(), 1, "missing or duplicate fixed case");
    let fields: Vec<_> = rows[0].split('\t').collect();
    assert_eq!(fields.len(), 19);
    fields
}

fn nullable(text: &str) -> Option<String> {
    (text != "<null>").then(|| text.to_owned())
}

fn addresses(text: &str) -> Vec<String> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split('|').map(str::to_owned).collect()
    }
}

fn response_and_expected(case: &str) -> (BaseSetupResponse, u8, ModernSetupReport22) {
    let f = fields(case);
    let packet = octets(f[1]);
    assert_eq!(packet.len(), 144);
    let token: u32 = f[2].parse().unwrap();
    let constructor_context: u8 = f[3].parse().unwrap();
    let index: u8 = f[4].parse().unwrap();
    let response = BaseSetupResponse::parse(&packet, token)
        .expect("SETUP_FAILURE: qualified base parser must succeed before report RED");
    assert_eq!(response.token, token);
    assert_eq!(response.context.get(), constructor_context);
    assert_eq!(response.reported_activity, f[8].parse::<u8>().unwrap());
    assert_eq!(response.dns_selector, packet[37]);
    assert_eq!(response.pcscf_selector, packet[78]);
    assert_eq!(response.dns.ipv4.is_some(), packet[37] & 1 != 0);
    assert_eq!(response.dns.ipv6.is_some(), packet[37] & 2 != 0);
    assert_eq!(response.pcscf.ipv4.is_some(), packet[78] & 1 != 0);
    assert_eq!(response.pcscf.ipv6.is_some(), packet[78] & 2 != 0);
    assert_eq!(f[17], "true");
    assert_eq!(f[18], "true");
    let expected = ModernSetupReport22 {
        cause: f[5].parse().unwrap(),
        opaque_i64_at_8: f[6].parse().unwrap(),
        cid: f[7].parse().unwrap(),
        active: f[8].parse().unwrap(),
        protocol_code: f[9].parse().unwrap(),
        mtu_v4: f[10].parse().unwrap(),
        mtu_v6: f[11].parse().unwrap(),
        ifname: nullable(f[12]),
        address_text: nullable(f[13]),
        dnses: addresses(f[14]),
        gateways: addresses(f[15]),
        pcscf: addresses(f[16]),
        structured_addresses: [],
        qos_sessions: [],
        traffic_descriptors: [],
    };
    (response, index, expected)
}

fn compare(case: &str) {
    let (response, index, expected) = response_and_expected(case);
    let before = response.clone();
    let actual = ModernSetupReport22::project(&response, index)
        .expect("MISSING_MODERN_SIT_REPORT_PROJECTION");
    assert_eq!(actual, expected, "all emitted report observations: {case}");
    assert_eq!(response, before, "projection must preserve its typed input");
}

#[test]
fn original_modern_v4() {
    compare("v4");
}
#[test]
fn original_modern_v6() {
    compare("v6");
}
#[test]
fn original_modern_dual_index0() {
    compare("dual-index0");
}
#[test]
fn original_modern_dual_index17() {
    compare("dual-index17");
}
#[test]
fn original_modern_cross_selectors() {
    compare("cross-selectors");
}
#[test]
fn original_modern_nonip() {
    compare("nonip");
}
#[test]
fn original_modern_inactive() {
    compare("inactive");
}
#[test]
fn original_modern_v6_long_strings() {
    compare("v6-long-strings");
}

#[test]
fn fixed_table_has_exact_fourteen_original_inputs() {
    let expected = [
        "v4",
        "v6",
        "dual-index0",
        "dual-index17",
        "cross-selectors",
        "nonip",
        "inactive",
        "v6-long-strings",
        "v4-local-zero",
        "v6-local-zero",
        "dual-zero-first-dns-pcscf",
        "v6-compatible",
        "v6-mapped",
        "v6-zero-run-tie",
    ];
    assert_eq!(TABLE.lines().count(), expected.len() + 1);
    for case in expected {
        let _ = response_and_expected(case);
    }
    let (zero, index0, mut report0) = response_and_expected("dual-index0");
    let (seventeen, index17, report17) = response_and_expected("dual-index17");
    assert_eq!(zero, seventeen);
    assert_eq!((index0, index17), (0, 17));
    report0.ifname = report17.ifname.clone();
    assert_eq!(
        report0, report17,
        "constructor index changes presentation name only"
    );
}

#[test]
fn refuses_unqualified_presentation_index_before_missing_capability() {
    let (response, _, _) = response_and_expected("v4");
    for index in [100, 255] {
        assert_eq!(
            ModernSetupReport22::project(&response, index),
            Err(ReportError::InvalidPresentationIndex)
        );
    }
}

#[test]
fn original_modern_v4_local_zero() {
    compare("v4-local-zero");
}

#[test]
fn original_modern_v6_local_zero() {
    compare("v6-local-zero");
}

#[test]
fn original_modern_dual_zero_first_dns_pcscf() {
    compare("dual-zero-first-dns-pcscf");
}

#[test]
fn original_modern_v6_compatible() {
    compare("v6-compatible");
}

#[test]
fn original_modern_v6_mapped() {
    compare("v6-mapped");
}

#[test]
fn original_modern_v6_zero_run_tie() {
    compare("v6-zero-run-tie");
}

#[test]
fn refuses_inconsistent_public_observations_before_missing_capability() {
    let (response, index, _) = response_and_expected("v4");
    let mutations: [fn(&mut BaseSetupResponse); 9] = [
        |r| r.reported_activity = 3,
        |r| r.dns_selector = 4,
        |r| r.pcscf_selector = 4,
        |r| r.ipv4 = None,
        |r| r.ipv6 = Some([0; 16]),
        |r| r.dns.ipv4 = None,
        |r| r.dns.ipv6 = Some([[0; 16]; 2]),
        |r| r.pcscf.ipv4 = None,
        |r| r.pcscf.ipv6 = Some([[0; 16]; 3]),
    ];
    for mutation in mutations {
        let mut invalid = response.clone();
        mutation(&mut invalid);
        assert_eq!(
            ModernSetupReport22::project(&invalid, index),
            Err(ReportError::InvalidResponse)
        );
    }
}
