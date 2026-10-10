//! An id is a letter and a number: `C54`, `K1`, `B3`. It is never
//! reused (B3). The file that holds entry `C54` is `C054.toml` (B5):
//! the number zero-padded to three digits so a directory listing sorts
//! numerically, while the id in prose stays `C54`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A typed id. `letter` is an ASCII capital; `number` is at least 1.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Id {
    pub letter: char,
    pub number: u32,
}

impl Id {
    /// Parses `C54`. Refuses a lowercase letter, a leading zero (`C054`
    /// is a file name, not an id), a zero, and anything else.
    pub fn parse(s: &str) -> Result<Id, String> {
        let mut chars = s.chars();
        let letter = match chars.next() {
            Some(c) if c.is_ascii_uppercase() => c,
            _ => {
                return Err(format!(
                    "\"{s}\" is not an id: an id starts with a capital letter, like C54"
                ));
            }
        };
        let digits = chars.as_str();
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!(
                "\"{s}\" is not an id: a capital letter followed by digits, like C54"
            ));
        }
        if digits.starts_with('0') {
            return Err(format!(
                "\"{s}\" is not an id: write it without the leading zero (the zero-padded form is for file names)"
            ));
        }
        let number: u32 = digits
            .parse()
            .map_err(|_| format!("\"{s}\" is not an id: the number is too large"))?;
        Ok(Id { letter, number })
    }

    /// The file stem for this id: `C054` for `C54`, `C1234` for `C1234`.
    pub fn file_stem(&self) -> String {
        format!("{}{:03}", self.letter, self.number)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.letter, self.number)
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Id, D::Error> {
        let s = String::deserialize(d)?;
        Id::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prints() {
        let id = Id::parse("C54").unwrap();
        assert_eq!((id.letter, id.number), ('C', 54));
        assert_eq!(id.to_string(), "C54");
        assert_eq!(id.file_stem(), "C054");
        assert_eq!(Id::parse("Z1234").unwrap().file_stem(), "Z1234");
    }

    #[test]
    fn refuses_what_is_not_an_id() {
        for bad in ["c54", "C", "54", "C054", "C0", "CA", "C5 4", ""] {
            assert!(Id::parse(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn orders_by_letter_then_number() {
        let mut ids = ["C54", "B2", "C7", "B10"].map(|s| Id::parse(s).unwrap());
        ids.sort();
        let printed: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(printed, ["B2", "B10", "C7", "C54"]);
    }
}
