//! The brief's worked examples (docs/design-brief.md §4) parse as the
//! schema says; a field the schema does not know is refused (B17).

use bothy::schema::{Decision, Park};

#[test]
fn the_brief_c54_parses() {
    let text = r#"
id = "C54"
area = "store"
title = "Authority is the coarser observation's"
ruling = """
A listing owns membership, address, league and liveness; a fetch owns
contents and nothing more; a fetch never revives what a listing retired.
"""
why = "Five review rounds on 2026-09-02 found this shape in five places before it was stated once (REFRESH-SLICE.md)."
details = ["acquisition-store/src/lib.rs doc, C54"]
pinned = []
evidence = []
decided = 2026-09-02
amended = []
verdict = ""
status = "ruled"
"#;
    let d: Decision = toml::from_str(text).expect("parses");
    assert_eq!(d.id.to_string(), "C54");
    assert_eq!(
        d.decided.as_ref().map(ToString::to_string).as_deref(),
        Some("2026-09-02")
    );
    assert!(d.ruling.starts_with("A listing owns"));
}

#[test]
fn the_brief_k12_parses_and_fires() {
    let text = r#"
id = "K12"
area = "store"
item = "User-scoped annotations home (`user.db`) + scope taxonomy"
home = "the store crate"
trigger = "the first user-scoped kind actually written (shop template, currency ratios, saved searches)"
parked = 2026-09-03
fired = 2026-10-01
fired_by = "C110"
"#;
    let p: Park = toml::from_str(text).expect("parses");
    assert_eq!(p.fired_by.map(|i| i.to_string()).as_deref(), Some("C110"));
    assert!(p.fired.is_some());
}

#[test]
fn a_misspelled_field_is_refused() {
    let text = r#"
id = "C1"
area = "x"
title = "t"
ruling = "r"
whyy = "a typo"
"#;
    let err = toml::from_str::<Decision>(text).expect_err("unknown field refused");
    assert!(err.message().contains("whyy"), "{}", err.message());
}

#[test]
fn a_bad_id_is_refused_with_a_reason() {
    let text = r#"
id = "C054"
area = "x"
title = "t"
ruling = "r"
"#;
    let err = toml::from_str::<Decision>(text).expect_err("leading zero refused");
    assert!(err.message().contains("leading zero"), "{}", err.message());
}
