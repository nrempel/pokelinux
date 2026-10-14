//! Scalar vectors captured from actual Original QML alpha7, independently of
//! this Rust parser. Real producer/Qt/pixel qualification remains separate.
use pokelinux_appearance::style;
use serde_json::json;
const COLORS: &str = "background='#1a1b26'\nforeground='#a9b1d6'\naccent='#7aa2f7'\nred='#f7768e'\ngreen='#9ece6a'\n";
fn projected(
    surface: &str,
    surface_alpha: &str,
    border: &str,
    border_alpha: &str,
    text: &str,
) -> serde_json::Value {
    let user = format!(
        "[popups]\nbackground=\"{surface}\"\nbackground-alpha={surface_alpha}\nborder=\"{border}\"\nborder-alpha={border_alpha}\ntext=\"{text}\"\ntext-alpha=0.125\n"
    );
    serde_json::to_value(
        style::project(COLORS, "", &user).expect("actual Original supported color"),
    )
    .unwrap()
}
#[test]
fn original_half_surface_preserves_16bit_alpha() {
    let value = projected("#d06020", "0.5", "#40a0e0", "1", "#e6e8ea");
    assert_eq!(
        value["palette"]["surface"],
        json!({"r":53456,"g":24672,"b":8224,"a":32768})
    );
    assert_eq!(
        value["palette"]["border"],
        json!({"r":16448,"g":41120,"b":57568,"a":65535})
    );
}
#[test]
fn original_companion_replaces_both_intrinsic_alphas() {
    let value = projected("#d0602080", "0.5", "#40a0e080", "0.5", "#e6e8ea");
    assert_eq!(
        value["palette"]["surface"],
        json!({"r":53456,"g":24672,"b":8224,"a":32768})
    );
    assert_eq!(
        value["palette"]["border"],
        json!({"r":16448,"g":41120,"b":57568,"a":32768})
    );
}
#[test]
fn original_intrinsic_only_color_consumers_remain_opaque() {
    let value = projected("#d0602080", "1", "#40a0e080", "1", "#e6e8ea");
    assert_eq!(value["palette"]["surface"]["a"], 65535);
    assert_eq!(value["palette"]["border"]["a"], 65535);
}
#[test]
fn original_direct_text_argb_ignores_companion() {
    let value = projected("#d06020", "1", "#40a0e0", "1", "#804488cc");
    assert_eq!(
        value["palette"]["foreground"],
        json!({"r":17476,"g":34952,"b":52428,"a":32896})
    );
}
