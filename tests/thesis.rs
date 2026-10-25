//! Agreement with the thesis code (Hypergraph-Dualization 1bb9de4, through srl 0.2.0) on the
//! committed reference: 300 random instances in full (hypergraph and provenance, certificates,
//! Berge family, FK-B order and per-pass node counts, FK-A tree and DAG node counts, the
//! conflicting assignment of a non-dual pair, exhaustive search), and on the 88 seed-1
//! instances the hypergraph, family size, first 50 FK-B panels, orbits and FK-A counts.

mod common;

use common::{diff, instance, record, snapshot};
use srl::json::{self, json, Value};
use srl::{audit, bench};

fn reference(name: &str) -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference").join(name);
    json::loads(&std::fs::read(p).unwrap()).unwrap()
}

#[test]
fn random_instances() {
    let mut d = Vec::new();
    for (i, x) in reference("random.json")["instances"].as_array().unwrap().iter().enumerate() {
        diff(&format!("random[{i}]"), &record(&instance(x), true, 300, x), x, &mut d);
    }
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}

#[test]
fn seed1_instances() {
    let (_, s) = snapshot();
    let M = audit::matrix(&s, "strict");
    let R = reference("suite-seed1.json");
    let X = R["instances"].as_array().unwrap();
    let mut d = Vec::new();
    let mut i = 0;
    for (c, rule, size, P) in bench::baskets(&M, 1) {
        for k in [0usize, 1] {
            let x = &X[i];
            i += 1;
            assert_eq!(json!([c, rule, size, k]), json!([x["class"], x["rule"], x["size"], x["k"]]));
            let I = audit::basket(&M, &P, k);
            let mut r = record(&I, true, 50, &json!({"H": 0, "berge": 0, "fkb_prefix": 0, "orbits": 0, "thesis": 0}));
            r["count"] = json!(r["berge"].as_array().unwrap().len());
            r.as_object_mut().unwrap().remove("berge");
            for key in ["class", "rule", "size", "k"] {
                r[key] = x[key].clone();
            }
            if x["thesis"].is_null() {
                r["thesis"] = Value::Null;
            }
            diff(&format!("{c}/{rule}/{size}/k{k}"), &r, x, &mut d);
        }
    }
    assert_eq!(i, X.len());
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}
