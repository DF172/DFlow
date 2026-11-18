// G1 approved four semantic stroke-width exceptions on 2026-10-21:
// attributes/lineweight, mt_bold, shapes/poly_e, status/lwt.
// Whitelist: these four paths only, widths only 1 / 1.5 / 2 / 3.
// Any other icon keeps the family default of 1.5; a new exception must go
// through G1 again and be added here by name.

const EXCEPTIONS: &[(&str, &[u8], &[&str])] = &[
    (
        "attributes/lineweight",
        include_bytes!("../assets/icons/attributes/lineweight.svg"),
        &["1.5", "3"],
    ),
    (
        "mt_bold",
        include_bytes!("../assets/icons/mt_bold.svg"),
        &["1.5", "3"],
    ),
    (
        "shapes/poly_e",
        include_bytes!("../assets/icons/shapes/poly_e.svg"),
        &["1.5", "3"],
    ),
    (
        "status/lwt",
        include_bytes!("../assets/icons/status/lwt.svg"),
        &["1", "1.5", "2", "3"],
    ),
];

fn stroke_widths(svg: &[u8]) -> Vec<String> {
    let text = std::str::from_utf8(svg).expect("icon svg is utf-8");
    let mut widths: Vec<String> = text
        .match_indices("stroke-width=\"")
        .filter_map(|(index, needle)| {
            let rest = &text[index + needle.len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        })
        .collect();
    widths.sort();
    widths.dedup();
    widths
}

#[test]
fn the_four_semantic_stroke_width_exceptions_stay_exact() {
    for (name, svg, expected) in EXCEPTIONS {
        let mut want: Vec<String> = expected.iter().map(|width| width.to_string()).collect();
        want.sort();
        want.dedup();
        assert_eq!(
            stroke_widths(svg),
            want,
            "{name} stroke widths drifted; re-run G1 before changing this whitelist"
        );
    }
}

#[test]
fn every_exception_stays_inside_the_approved_width_ladder() {
    for (name, svg, _) in EXCEPTIONS {
        for width in stroke_widths(svg) {
            assert!(
                matches!(width.as_str(), "1" | "1.5" | "2" | "3"),
                "{name} uses unapproved stroke-width {width}"
            );
        }
    }
}
