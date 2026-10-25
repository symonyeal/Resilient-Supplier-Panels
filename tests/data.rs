//! Workbook -> records with provenance -> products -> instance.
//!
//! Real fixture: the frozen snapshot data/raw/QBL.xlsx (public USDA data, hash in
//! data/manifest.json). Row-2 literals were read off the workbook with openpyxl (2026-10-15),
//! an independent reader.

use std::collections::{BTreeSet, HashMap};

use srl::{audit, catalog, data, json, qbl};

fn row(n: usize, name: &str, addr: &str, zip: &str, product: &str) -> qbl::Row {
    let f: HashMap<String, String> = [("Vendor Name", name), ("Billing Address", addr), ("City", "X"), ("State", "WA"),
        ("ZIP Code", zip), ("Size", "Small"), ("Product", product)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    qbl::Row { row: n, f }
}

fn mappings() -> String {
    data::text("catalog/mappings.csv").unwrap()
}

#[test]
fn snapshot_hash_matches_manifest() {
    let m = json::loads(&data::bytes("manifest.json").unwrap()).unwrap();
    assert_eq!(data::sha256(&data::bytes("raw/QBL.xlsx").unwrap()), m["QBL.xlsx"]["sha256"].as_str().unwrap());
}

#[test]
fn reader_on_frozen_snapshot() {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    assert_eq!((rows.len(), rows[0].row), (983, 2));
    assert_eq!(rows[0].get("Vendor Name"), "1st Choice Logistics DBA JAWS Transportation");
    assert_eq!(rows[0].get("ZIP Code"), "83211");
    assert_eq!(rows[0].get("Product"), "Potato Bulk For Process \nFRZ Potatoes Russet Fresh CTN");
}

#[test]
fn multiline_cells_split_and_tokens_normalised() {
    let (_, Q, _) = qbl::records(&[row(2, "A Co", "1 Main St", "12345", "WF -Wheat Flour\n  Beans Pinto Can \n")]);
    let t: BTreeSet<&str> = Q.iter().map(|q| q.1.as_str()).collect();
    assert_eq!(t, BTreeSet::from(["beans pinto can", "wf - wheat flour"]));
    assert!(Q.iter().all(|q| q.2 == 2));
}

#[test]
fn repeated_rows_merge_and_name_collisions_stay_distinct() {
    let rows = [row(2, "A Co", "1 Main St", "12345", "Fresh Apples"), row(3, "A  CO", "1 Main St", "12345", "Pears Bosc Fresh Ctn"),
                row(4, "A Co", "9 Other Rd", "12345", "Fresh Apples"), row(5, "A Co, Inc.", "1 Main St", "12345", "Fresh Apples")];
    let (S, _, issues) = qbl::records(&rows);
    assert_eq!(S.len(), 3);                              // rows 2 and 3 share the identity key
    let kinds: BTreeSet<(&str, usize)> = issues.iter().map(|(r, k, _)| (*k, *r)).collect();
    assert!(kinds.contains(&("same_identity", 3)));
    assert!(kinds.contains(&("identity_candidate", 4)) && kinds.contains(&("identity_candidate", 5)));
}

#[test]
fn missing_fields_are_reported_not_dropped_silently() {
    let (S, Q, issues) = qbl::records(&[row(2, "", "1", "1", "X Can"), row(3, "B Co", "1", "1", "  ")]);
    assert!(S.is_empty() && Q.is_empty());
    let got: BTreeSet<(usize, &str)> = issues.iter().map(|(r, k, _)| (*r, *k)).collect();
    assert_eq!(got, BTreeSet::from([(2, "missing_name"), (3, "missing_product")]));
}

#[test]
fn zip_codes_padded_and_staleness() {
    assert_eq!(qbl::records(&[row(2, "A", "1", "8518", "x")]).0["r0002"].zip5, "08518");
    assert_eq!(qbl::age_days("2026-10-15T07:16:47Z", "2027-02-01"), 109);
    assert!(qbl::stale("2026-10-15T07:16:47Z", "2027-02-01") && !qbl::stale("2026-10-15T07:16:47Z", "2026-11-01"));
}

#[test]
fn rules_classify_each_vocabulary() {
    assert_eq!(catalog::rule("100193 - pork shoulder, picnics").0, "material");
    assert_eq!(catalog::rule("311999 deb dry edible beans").0, "naics");
    assert_eq!(catalog::rule("wf - wheat flour"), ("program", Some("wf".into())));
    assert_eq!(catalog::rule("beans pinto can").0, "description");
    assert_eq!(catalog::rule("walnuts").0, "commodity");
}

#[test]
fn mappings_split_double_codes_and_mark_broad() {
    let t: Vec<String> = ["dyo - high protein yogurt dfm - fluid milk", "fresh fruit", "walnuts"].map(String::from).to_vec();
    let D = catalog::load(&t, &mappings());
    assert_eq!(D[&t[0]].strict, vec!["dyo - high protein yogurt", "dfm - fluid milk"]);
    assert_eq!((D[&t[1]].class.as_str(), D[&t[2]].class.as_str()), ("broad", "commodity"));
}

#[test]
fn every_snapshot_token_has_an_entry_and_one_merged_image() {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = audit::snapshot(&rows, &mappings());
    let classes = ["material", "naics", "program", "description", "commodity", "broad"];
    assert!(s.D.values().all(|e| classes.contains(&e.class.as_str())));
    let M = catalog::merged_of(&s.D);
    assert_eq!((M["wf - wheat flour"].as_str(), M["pp - peanut products"].as_str(), M["pp - pasta products"].as_str()),
               ("wf", "pp", "pa"));
}

#[test]
fn multi_product_tokens_reach_both_products() {
    let (S, Q, _) = qbl::records(&[row(2, "A", "1", "1", "DYO - High Protein Yogurt DFM - Fluid Milk")]);
    let s = audit::Snapshot { D: catalog::load(Q.iter().map(|q| &q.1), &mappings()), S, Q, issues: vec![] };
    let M = audit::matrix(&s, "strict");
    assert_eq!(M["dfm - fluid milk"].1, BTreeSet::from(["r0002".to_string()]));
}

/// "dddddd - … dddddd - " or "code - … code - name": two codes on one line.
fn two_codes(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    let dash = |j: usize| c.get(j..j + 3) == Some(&[' ', '-', ' '][..]);
    // index after "dddddd - " at i
    let material = |i: usize| (c.len() >= i + 9 && c[i..i + 6].iter().all(char::is_ascii_digit) && dash(i + 6)).then_some(i + 9);
    // index after "[a-z]{lo,5} digit? - " at i; a lowercase letter must follow when `letter`
    let code = |i: usize, lo: usize, letter: bool| {
        let n = c[i.min(c.len())..].iter().take_while(|x| x.is_ascii_lowercase()).count();
        if !(lo..=5).contains(&n) {
            return None;
        }
        let ok = |j: usize| dash(j) && (!letter || c.get(j + 3).is_some_and(char::is_ascii_lowercase));
        let digit = c.get(i + n).is_some_and(char::is_ascii_digit);
        [i + n + 1, i + n].into_iter().filter(|&j| j == i + n || digit).find(|&j| ok(j)).map(|j| j + 3)
    };
    // a later match, after a space that follows index e
    let later = |e: usize, f: &dyn Fn(usize) -> bool| (e + 1..c.len()).any(|j| c[j - 1] == ' ' && f(j));
    (0..c.len()).any(|i| material(i).is_some_and(|e| later(e, &|j| material(j).is_some())))
        || code(0, 1, false).is_some_and(|e| later(e, &|j| code(j, 2, true).is_some()))
}

#[test]
fn no_snapshot_token_holds_two_codes_unsplit() {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = audit::snapshot(&rows, &mappings());
    let bad: Vec<&String> = s.D.iter().filter(|(t, e)| two_codes(t) && e.strict.len() != 2 && *t != "pa - pasta - pasta product")
        .map(|(t, _)| t).collect();
    assert!(bad.is_empty(), "{bad:?}");
    // truth values of the regex in Python
    for (t, want) in [("100127 - beef can-24/24 oz 110477 - chicken pouch 36/10 oz", true),
                      ("dyo - high protein yogurt dfm - fluid milk", true), ("ab - c", false), ("ab -  cd - e", true),
                      ("pa - pasta - pasta product", false), ("ab - x cd1 - e", true), ("ab - x c - e", false),
                      ("ab - x abcdef - e", false), ("100127 - 110477 - x", false), ("ab - x tor1 - x", true)] {
        assert_eq!(two_codes(t), want, "{t}");
    }
}
