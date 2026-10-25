//! Identity and ownership catalog checks against synthetic and frozen snapshots.

use std::collections::{BTreeMap, HashMap};

use srl::{audit, certify, data, entities, instance::Instance, qbl};

fn row(n: usize, name: &str, address: &str, product: &str) -> qbl::Row {
    let f: HashMap<String, String> = [
        ("Vendor Name", name),
        ("Billing Address", address),
        ("City", "X"),
        ("State", "WA"),
        ("ZIP Code", "12345"),
        ("Size", "Small"),
        ("Product", product),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    qbl::Row { row: n, f }
}

fn snapshot(rows: &[qbl::Row]) -> audit::Snapshot {
    audit::snapshot(rows, &data::text("catalog/mappings.csv").unwrap())
}

#[test]
fn entity_groups_close_identity_and_owner_edges_transitively() {
    let pairs: Vec<entities::Pair> = vec![
        ("r0001".into(), "r0002".into(), "same".into(), "evidence".into()),
        ("r0002".into(), "r0003".into(), "site".into(), "evidence".into()),
    ];
    let owners = BTreeMap::from([("parent".into(), vec!["r0003".into(), "r0004".into()])]);

    let groups = entities::groups(&pairs, &owners);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups["parent"], ["r0001", "r0002", "r0003", "r0004"]);
}

#[test]
fn same_pairs_merge_but_site_pairs_remain_distinct_in_the_identity_variant() {
    let I = Instance::new(
        vec!["r0001".into(), "r0002".into(), "r0003".into()],
        vec!["product".into()],
        vec![vec!["r0001".into(), "r0002".into(), "r0003".into()]],
        0,
        Vec::new(),
    )
    .unwrap();
    let pairs: Vec<entities::Pair> = vec![
        ("r0001".into(), "r0002".into(), "same".into(), "evidence".into()),
        ("r0002".into(), "r0003".into(), "site".into(), "evidence".into()),
    ];
    let rep: HashMap<&String, &String> = pairs
        .iter()
        .filter(|p| p.2 == "same")
        .map(|p| (&p.1, &p.0))
        .collect();

    let merged = entities::merge(&I, &rep);

    assert_eq!(merged.V, ["r0001", "r0003"]);
    assert_eq!(merged.E[0], [0, 1]);
}

#[test]
fn owner_scenario_has_an_explicit_infeasibility_certificate() {
    let s = snapshot(&[
        row(2, "Alpha Foods", "1 Main St", "beans pinto can"),
        row(3, "Beta Foods", "2 Main St", "beans pinto can"),
    ]);
    let owners = entities::owners(
        "group,supplier,name,relation,source\nAlliance,r0002,Alpha Foods,member,test source\nAlliance,r0003,Beta Foods,member,test source\n",
        &s,
    )
    .unwrap();
    let M = audit::matrix(&s, "strict");
    let I = entities::with_owners(&M, &["beans pinto can".into()], 0, &owners);

    let (p, failed) = certify::infeasibility(&I).expect("the group scenario removes every supplier");

    assert_eq!(p, 0);
    assert_eq!(failed.iter().map(|&v| I.V[v].as_str()).collect::<Vec<_>>(), ["r0002", "r0003"]);
}

#[test]
fn malformed_short_identity_row_is_rejected() {
    let s = snapshot(&[
        row(2, "Alpha Foods", "1 Main St", "beans pinto can"),
        row(3, "Alpha Foods", "2 Main St", "beans pinto can"),
    ]);
    let text = "first,candidate,verdict,evidence\nr0002,r0003,same\n";

    assert!(entities::identities(text, &s).is_err());
}

#[test]
fn identity_candidate_without_evidence_is_rejected() {
    let s = snapshot(&[
        row(2, "Alpha Foods", "1 Main St", "beans pinto can"),
        row(3, "Alpha Foods", "2 Main St", "beans pinto can"),
    ]);
    let text = "first,candidate,verdict,evidence\nr0002,r0003,same,\n";

    assert!(entities::identities(text, &s).is_err());
}

#[test]
fn frozen_default_snapshot_identity_and_owner_catalogs_validate() {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = snapshot(&rows);
    let identities = entities::identities(&data::text("catalog/identities.csv").unwrap(), &s).unwrap();
    let owners = entities::owners(&data::text("catalog/owners.csv").unwrap(), &s).unwrap();

    assert!(!identities.is_empty());
    assert!(!owners.is_empty());
    let A = srl::json::loads(&data::bytes("catalog/usaspending.json").unwrap()).unwrap();
    assert!(entities::awards(&A, &s).is_ok());
}

#[test]
fn missing_award_amount_is_an_error() {
    let s = snapshot(&[row(2, "Alpha Foods", "1 Main St", "beans pinto can")]);
    let A = srl::json::json!({"links": {"r0002": ["UEI"]}, "recipients": {"UEI": {}}});
    assert!(entities::awards(&A, &s).is_err());
}
