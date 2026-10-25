//! USDA AMS Qualified Bidders List: workbook -> rows -> suppliers and qualifications.
//!
//! Harmless normalisation (whitespace, case, spacing round " - ", ZIP zero-padding) is done
//! here. Semantic interpretation (granularity, merged products) is in catalog.rs and
//! data/catalog/mappings.csv. See docs/DATA.md.
//!
//! Whitespace is Python's (`str.isspace`); case folding is lowercase, which equals Python's
//! `casefold` on every character of the frozen snapshot (tests/parity.rs).

use std::collections::{BTreeMap, HashMap};

use crate::xlsx;

pub const SUFFIX: [&str; 13] = ["inc", "llc", "llp", "lp", "co", "company", "corp", "corporation",
                                "ltd", "the", "incorporated", "and", "of"];
pub const STALE_DAYS: i64 = 90;

/// A data row: header -> text, and its sheet row number.
#[derive(Clone, Debug)]
pub struct Row {
    pub row: usize,
    pub f: HashMap<String, String>,
}

impl Row {
    pub fn get(&self, h: &str) -> &str {
        self.f.get(h).map_or("", String::as_str)
    }
}

/// Rows of the first sheet keyed by the header row, blank rows dropped.
pub fn read(workbook: &[u8]) -> Result<Vec<Row>, String> {
    let table = xlsx::sheet(workbook)?;
    let Some(((_, head), body)) = table.split_first() else { return Ok(Vec::new()) };
    Ok(body.iter()
        .filter(|(_, cells)| cells.values().any(|x| !strip(x).is_empty()))
        .map(|(n, cells)| Row {
            row: *n,
            f: head.iter().map(|(c, h)| (h.clone(), cells.get(c).cloned().unwrap_or_default()))
                .collect(),
        })
        .collect())
}

/// Python's `str.isspace`.
pub fn space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Python's `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(space)
}

/// Words separated by whitespace (Python `str.split()`).
pub fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(space).filter(|w| !w.is_empty())
}

/// Whitespace collapsed and stripped, case folded.
pub fn clean(s: &str) -> String {
    words(s).collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Product token: whitespace collapsed, case folded, ' - ' spacing unified
/// (re.sub(r"\s+-\s*|\s*-\s+", " - ", clean(s)) on a single-spaced string).
pub fn token(s: &str) -> String {
    let c: Vec<char> = clean(s).chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == ' ' && c.get(i + 1) == Some(&'-') {
            i += if c.get(i + 2) == Some(&' ') { 3 } else { 2 };
            out.push_str(" - ");
        } else if c[i] == '-' && c.get(i + 1) == Some(&' ') {
            i += 2;
            out.push_str(" - ");
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

fn zfill(s: &str, w: usize) -> String {
    format!("{}{s}", "0".repeat(w.saturating_sub(s.chars().count())))
}

/// The 5-digit ZIP: the part before '-', zero-padded (Excel stored ZIPs as numbers).
pub fn zip5(s: &str) -> String {
    let d = strip(s).split('-').next().unwrap_or("");
    if d.chars().count() <= 5 { zfill(d, 5) } else { zfill(d, 9).chars().take(5).collect() }
}

/// Name with punctuation and legal suffixes removed: a review key, never a merge key.
pub fn core(name: &str) -> String {
    let s: String = clean(name).chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || space(c) { c } else { ' ' })
        .collect();
    words(&s).filter(|w| !SUFFIX.contains(w)).collect::<Vec<_>>().join(" ")
}

#[derive(Clone, Debug)]
pub struct Supplier {
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip5: String,
    pub size: String,
    pub rows: Vec<usize>,
}

/// (row, kind, detail): missing_name, missing_product, same_identity (merged: equal key),
/// identity_candidate (equal core name, kept apart).
pub type Issue = (usize, &'static str, String);

/// (S, Q, issues) of `records`.
pub type Records = (BTreeMap<String, Supplier>, Vec<(String, String, usize)>, Vec<Issue>);

/// Suppliers keyed by identity (name, address, ZIP5), qualifications with source rows.
///
/// OUTPUT  S ...... id -> supplier; id = 'r' + first sheet row
///         Q ...... [(id, token, row)]
///         issues . [(row, kind, detail)]
pub fn records(rows: &[Row]) -> Records {
    let (mut S, mut Q, mut issues): Records = (BTreeMap::new(), Vec::new(), Vec::new());
    let mut by_key: HashMap<(String, String, String), String> = HashMap::new();
    let mut by_core: HashMap<String, String> = HashMap::new();
    for r in rows {
        let (n, name) = (r.row, clean(r.get("Vendor Name")));
        let toks: Vec<String> = r.get("Product").split(['\r', '\n']).map(token)
            .filter(|t| !t.is_empty()).collect();
        if name.is_empty() || toks.is_empty() {
            issues.push((n, if name.is_empty() { "missing_name" } else { "missing_product" }, String::new()));
            continue;
        }
        let key = (name.clone(), clean(r.get("Billing Address")), zip5(r.get("ZIP Code")));
        let sid = if let Some(sid) = by_key.get(&key) {
            if let Some(s) = S.get_mut(sid) {
                s.rows.push(n);
            }
            issues.push((n, "same_identity", sid.clone()));
            sid.clone()
        } else {
            let sid = format!("r{n:04}");
            by_key.insert(key.clone(), sid.clone());
            let c = core(&name);
            if let Some(first) = by_core.get(&c) {
                issues.push((n, "identity_candidate", first.clone()));
            }
            by_core.entry(c).or_insert_with(|| sid.clone());
            S.insert(sid.clone(), Supplier {
                name: words(r.get("Vendor Name")).collect::<Vec<_>>().join(" "),
                address: key.1, city: clean(r.get("City")), state: clean(r.get("State")).to_uppercase(),
                zip5: key.2, size: clean(r.get("Size")), rows: vec![n] });
            sid
        };
        Q.extend(toks.into_iter().map(|t| (sid.clone(), t, n)));
    }
    (S, Q, issues)
}

/// Days from the snapshot's retrieval date (an ISO timestamp) to `today` (ISO date).
pub fn age_days(retrieved_utc: &str, today: &str) -> i64 {
    days(&today[..10]) - days(&retrieved_utc[..10])
}

/// The snapshot is older than STALE_DAYS.
pub fn stale(retrieved_utc: &str, today: &str) -> bool {
    age_days(retrieved_utc, today) > STALE_DAYS
}

/// Days since 1970-01-01 of a proleptic Gregorian YYYY-MM-DD.
pub fn days(iso: &str) -> i64 {
    let p: Vec<i64> = iso.split('-').map(|x| x.parse().unwrap()).collect();
    let (y, m, d) = (p[0] - (p[1] <= 2) as i64, p[1], p[2]);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation() {
        assert_eq!(token("WF -Wheat  Flour"), "wf - wheat flour");
        assert_eq!(token("a- b"), "a - b");
        assert_eq!(token("a-b"), "a-b");
        assert_eq!(token("a --b"), "a - -b");
        assert_eq!(zip5("8518"), "08518");
        assert_eq!(zip5("123456789"), "12345");
        assert_eq!(zip5("1234-5678"), "01234");
        assert_eq!(core("The Acme Co., Inc."), "acme");
    }

    #[test]
    fn calendar() {
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!(age_days("2026-10-15T12:00:00Z", "2027-01-14"), 91);
        assert!(stale("2026-10-15T12:00:00Z", "2027-01-14"));
        assert!(!stale("2026-10-15T12:00:00Z", "2027-01-13"));
    }
}
