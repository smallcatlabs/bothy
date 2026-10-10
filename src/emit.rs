//! Writing an entry as a TOML file the way a person would (B5, B14):
//! fields in a fixed order, optional fields omitted rather than written
//! empty, prose in the string form that needs the fewest escapes. A
//! string with no quote, backslash or newline is a plain `"..."`; any
//! other is a literal `'''` block whose closing delimiter ends the last
//! line, so the stored value is exactly the text and nothing more.
//!
//! This is the writer `import` and, later, `new` use. `amend` edits an
//! existing file through `toml_edit` instead and never rewrites it.

use std::fmt::Write as _;

use crate::schema::{Date, Decision, Park};

/// One TOML string value, quoted as the text requires.
pub fn string(value: &str) -> String {
    if !value.contains(['"', '\\', '\n', '\r']) {
        format!("\"{value}\"")
    } else if !value.contains("'''") {
        format!("'''\n{value}'''")
    } else {
        let mut s = String::from("\"");
        for c in value.chars() {
            match c {
                '"' => s.push_str("\\\""),
                '\\' => s.push_str("\\\\"),
                '\n' => s.push_str("\\n"),
                '\r' => s.push_str("\\r"),
                '\t' => s.push_str("\\t"),
                c => s.push(c),
            }
        }
        s.push('"');
        s
    }
}

fn list(values: &[String]) -> String {
    let items: Vec<String> = values.iter().map(|v| string(v)).collect();
    format!("[{}]", items.join(", "))
}

fn field(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "{key} = {}", string(value));
}

fn opt(out: &mut String, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        field(out, key, v);
    }
}

fn date(out: &mut String, key: &str, value: &Date) {
    let _ = writeln!(out, "{key} = {value}");
}

pub fn decision(d: &Decision) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "id = \"{}\"", d.id);
    field(&mut out, "area", &d.area);
    if !d.title.is_empty() {
        field(&mut out, "title", &d.title);
    }
    field(&mut out, "ruling", &d.ruling);
    opt(&mut out, "why", &d.why);
    if !d.details.is_empty() {
        let _ = writeln!(out, "details = {}", list(&d.details));
    }
    if !d.pinned.is_empty() {
        let _ = writeln!(out, "pinned = {}", list(&d.pinned));
    }
    if !d.evidence.is_empty() {
        let _ = writeln!(out, "evidence = {}", list(&d.evidence));
    }
    opt(&mut out, "verdict", &d.verdict);
    opt(&mut out, "history", &d.history);
    if let Some(x) = &d.decided {
        date(&mut out, "decided", x);
    }
    opt(&mut out, "decided_from", &d.decided_from);
    if !d.amended.is_empty() {
        let dates: Vec<String> = d.amended.iter().map(ToString::to_string).collect();
        let _ = writeln!(out, "amended = [{}]", dates.join(", "));
    }
    if d.status != "ruled" {
        field(&mut out, "status", &d.status);
    }
    if let Some(by) = d.superseded_by {
        let _ = writeln!(out, "superseded_by = \"{by}\"");
    }
    out
}

pub fn park(p: &Park) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "id = \"{}\"", p.id);
    field(&mut out, "area", &p.area);
    field(&mut out, "item", &p.item);
    field(&mut out, "home", &p.home);
    opt(&mut out, "trigger", &p.trigger);
    opt(&mut out, "why", &p.why);
    opt(&mut out, "verdict", &p.verdict);
    date(&mut out, "parked", &p.parked);
    opt(&mut out, "parked_from", &p.parked_from);
    if let Some(x) = &p.fired {
        date(&mut out, "fired", x);
    }
    if let Some(by) = p.fired_by {
        let _ = writeln!(out, "fired_by = \"{by}\"");
    }
    opt(&mut out, "measured", &p.measured);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_take_the_form_needing_fewest_escapes() {
        assert_eq!(string("plain text"), "\"plain text\"");
        assert_eq!(string("it's fine"), "\"it's fine\"");
        assert_eq!(string("a \"quote\""), "'''\na \"quote\"'''");
        assert_eq!(string("two\nlines"), "'''\ntwo\nlines'''");
        assert_eq!(
            string("has ''' inside \"q\""),
            "\"has ''' inside \\\"q\\\"\""
        );
    }

    #[test]
    fn every_form_round_trips() {
        for text in [
            "plain",
            "a \"quote\" and a back\\slash",
            "two\nlines\n",
            "x ''' y \"z\"",
            "",
        ] {
            let doc = format!("v = {}", string(text));
            let table: toml::Table = toml::from_str(&doc).unwrap_or_else(|e| panic!("{doc}: {e}"));
            assert_eq!(table["v"].as_str(), Some(text), "{doc}");
        }
    }
}
