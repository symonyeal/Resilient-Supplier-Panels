//! Product dictionary: token -> (class, strict products, merged products, justification).
//!
//! Rules (docs/DATA.md), applied in order; data/catalog/mappings.csv overrides them:
//!   material     '^dddddd - '         a numbered USDA material
//!   naics        '^dddddd x'          NAICS-prefixed commodity code (international programmes)
//!   program      '^code - name'       AMS programme code; merged variant reads it as its code
//!   description  contains a pack/form word (can, pkg, ctn, frz, ...)
//!   commodity    anything else: a named commodity without specification
//! Strict product = the token itself unless a mapping says otherwise.

use std::collections::{BTreeMap, HashMap};

pub const PACK: [&str; 24] = ["can", "cans", "pkg", "ctn", "carton", "bag", "bags", "box", "pouch",
                              "bulk", "lb", "lbs", "oz", "dozen", "jug", "bottle", "frz", "iqf",
                              "chilled", "jar", "tote", "drum", "pail", "case"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub class: String,
    pub strict: Vec<String>,
    pub merged: Vec<String>,
    pub justification: String,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The rule class of a token, and the programme code for a programme token.
pub fn rule(t: &str) -> (&'static str, Option<String>) {
    let c: Vec<char> = t.chars().collect();
    let digits6 = c.len() >= 7 && c[..6].iter().all(char::is_ascii_digit);
    if digits6 && c[6] == ' ' {
        if c.len() >= 9 && c[7] == '-' && c[8] == ' ' {
            return ("material", None);
        }
        if c.len() >= 8 && !crate::qbl::space(c[7]) {
            return ("naics", None);
        }
    }
    let letters = c.iter().take_while(|x| x.is_ascii_lowercase()).count();
    if (1..=5).contains(&letters) {
        let mut j = letters;
        if c.get(j).is_some_and(|d| d.is_ascii_digit()) {
            j += 1;
        }
        if c.get(j..j + 3) == Some(&[' ', '-', ' ']) {
            return ("program", Some(c[..j].iter().collect()));
        }
    }
    let runs = t.split(|x: char| !word(x)).filter(|w| !w.is_empty());
    if runs.into_iter().any(|w| PACK.contains(&w)) { ("description", None) } else { ("commodity", None) }
}

/// Rows of a CSV file (comma, double-quote quoting, doubled quotes), header first.
pub fn csv(text: &str) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut cell, mut quoted, mut any) = (Vec::new(), Vec::new(), String::new(), false, false);
    let mut it = text.chars().peekable();
    while let Some(ch) = it.next() {
        if quoted {
            match ch {
                '"' if it.peek() == Some(&'"') => {
                    it.next();
                    cell.push('"');
                }
                '"' => quoted = false,
                _ => cell.push(ch),
            }
            continue;
        }
        match ch {
            '"' => {
                quoted = true;
                any = true;
            }
            ',' => {
                row.push(std::mem::take(&mut cell));
                any = true;
            }
            '\r' => {}
            '\n' => {
                if any || !cell.is_empty() {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                any = false;
            }
            _ => {
                cell.push(ch);
                any = true;
            }
        }
    }
    if any || !cell.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// Dictionary entry for every token, from the rules and the override file's text.
pub fn load<'a>(tokens: impl IntoIterator<Item = &'a String>, mappings: &str) -> BTreeMap<String, Entry> {
    let rows = csv(mappings);
    let head = &rows[0];
    let col = |name: &str| head.iter().position(|h| h == name).expect("mappings.csv column");
    let (t, c, s, m, j) = (col("token"), col("class"), col("strict"), col("merged"), col("justification"));
    let over: HashMap<&str, &Vec<String>> = rows[1..].iter().map(|r| (r[t].as_str(), r)).collect();
    let split = |x: &str| x.split('|').map(String::from).collect();
    tokens.into_iter().map(|tok| {
        let e = if let Some(r) = over.get(tok.as_str()) {
            Entry { class: r[c].clone(), strict: split(&r[s]), merged: split(&r[m]),
                    justification: format!("mapping: {}", r[j]) }
        } else {
            match rule(tok) {
                (_, Some(code)) => Entry { class: "program".into(), strict: vec![tok.clone()],
                                           justification: format!("rule: programme code {}", code.to_uppercase()),
                                           merged: vec![code] },
                (cls, None) => Entry { class: cls.into(), strict: vec![tok.clone()], merged: vec![tok.clone()],
                                       justification: format!("rule: {cls}") },
            }
        };
        (tok.clone(), e)
    }).collect()
}

/// Strict product -> merged product, read off the parallel lists.
pub fn merged_of(D: &BTreeMap<String, Entry>) -> BTreeMap<String, String> {
    let mut M = BTreeMap::new();
    for e in D.values() {
        for (p, q) in e.strict.iter().zip(&e.merged) {
            let was = M.entry(p.clone()).or_insert_with(|| q.clone());
            assert_eq!(was, q, "strict product {p} has two merged images");
        }
    }
    M
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert_eq!(rule("100127 - beef can"), ("material", None));
        assert_eq!(rule("311615 chicken"), ("naics", None));
        assert_eq!(rule("wf - wheat flour"), ("program", Some("wf".into())));
        assert_eq!(rule("tor1 - tortillas"), ("program", Some("tor1".into())));
        assert_eq!(rule("beans pinto can"), ("description", None));
        assert_eq!(rule("beans canned"), ("commodity", None));
        assert_eq!(rule("chicken 8 pc ctn-30 lb"), ("description", None));
    }

    #[test]
    fn csv_quotes() {
        assert_eq!(csv("a,b\n\"x, y\",\"he said \"\"no\"\"\"\n"),
                   vec![vec!["a", "b"], vec!["x, y", "he said \"no\""]]);
    }
}
