//! Two-way agreement with reference outputs of the Python 0.2 release on the thesis code at
//! 1bb9de4. The reference directory is named by SRL_REF; the tests are ignored by default and fail without it.
//! Each test reports every differing path, not only the first.

use std::path::PathBuf;

mod common;
use common::*;

use srl::json::{Value, json};
use srl::pyrand::Random;
use srl::{audit, bench, catalog};

fn reference() -> PathBuf {
    std::env::var_os("SRL_REF")
        .map(PathBuf::from)
        .expect("SRL_REF must name the captured reference directory")
}

fn load(name: &str) -> Value {
    let p = reference().join(name);
    srl::json::loads(&std::fs::read(&p).unwrap()).unwrap()
}

#[test]
fn comparator_detects_planted_differences() {
    let a = json!({"x": [1, 2, 3], "y": {"z": "a"}, "f": 0.5});
    let mut d = Vec::new();
    diff("t", &a, &a.clone(), &mut d);
    assert!(d.is_empty());
    for b in [
        json!({"x": [1, 3, 2], "y": {"z": "a"}, "f": 0.5}),
        json!({"x": [1, 2, 3], "y": {"z": "b"}, "f": 0.5}),
        json!({"x": [1, 2, 3], "y": {}, "f": 0.5}),
        json!({"x": [1, 2], "y": {"z": "a"}, "f": 0.5}),
        json!({"x": [1, 2, 3], "y": {"z": "a"}, "f": 0.25}),
    ] {
        let mut d = Vec::new();
        diff("t", &a, &b, &mut d);
        assert!(!d.is_empty(), "{b}");
    }
}

#[test]
#[ignore = "needs SRL_REF, the captured 0.2 reference directory"]
fn data_layer() {
    let (rows, s) = snapshot();
    let R: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut m: serde_json::Map<String, Value> =
                r.f.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
            m.insert("row".into(), json!(r.row));
            Value::Object(m)
        })
        .collect();
    check("rows", &json!(R), &load("rows.json"));
    let S: serde_json::Map<String, Value> =
        s.S.iter()
            .map(|(k, x)| {
                (
                    k.clone(),
                    json!({
        "name": x.name, "address": x.address, "city": x.city, "state": x.state, "zip5": x.zip5,
        "size": x.size, "rows": x.rows}),
                )
            })
            .collect();
    let Q: Vec<Value> = s.Q.iter().map(|(a, b, c)| json!([a, b, c])).collect();
    let issues: Vec<Value> = s.issues.iter().map(|(a, b, c)| json!([a, b, c])).collect();
    check(
        "records",
        &json!({"S": S, "Q": Q, "issues": issues}),
        &load("records.json"),
    );
    let D: serde_json::Map<String, Value> =
        s.D.iter()
            .map(|(t, e)| {
                (
                    t.clone(),
                    json!([e.class, e.strict, e.merged, e.justification]),
                )
            })
            .collect();
    check("catalog", &Value::Object(D), &load("catalog.json"));
    check(
        "merged_of",
        &json!(catalog::merged_of(&s.D)),
        &load("merged_of.json"),
    );
    check("audit", &audit::audit(&rows, &s), &load("audit.json"));
    for (v, f) in [
        ("strict", "matrix-strict.json"),
        ("merged", "matrix-merged.json"),
    ] {
        let M: serde_json::Map<String, Value> =
            audit::matrix(&s, srl::catalog::Variant::parse(v).unwrap())
                .into_iter()
                .map(|(p, (c, E))| (p, json!([c, E])))
                .collect();
        check(v, &Value::Object(M), &load(f));
    }
}

#[test]
#[ignore = "needs SRL_REF, the captured 0.2 reference directory"]
fn python_random_and_baskets() {
    let mut got = serde_json::Map::new();
    for seed in [0u64, 1, 2, 12345, (1 << 40) + 7] {
        let mut xs: Vec<usize> = (0..30).collect();
        Random::new(seed).shuffle(&mut xs);
        let mut g = Random::new(seed);
        let mut h = Random::new(seed);
        got.insert(
            seed.to_string(),
            json!({
            "bits32": [Random::new(seed).getrandbits(32)],
            "seq32": (0..8).map(|_| g.getrandbits(32)).collect::<Vec<_>>(),
            "shuffle30": xs,
            "sample": Random::new(seed).sample(&(0..100).collect::<Vec<_>>(), 20),
            "sample_small": Random::new(seed).sample(&(0..10).collect::<Vec<_>>(), 3),
            "choice": [Random::new(seed).choice(&(0..1000).collect::<Vec<_>>())],
            "randint": (0..10).map(|_| h.randint(1, 9)).collect::<Vec<_>>()}),
        );
    }
    check("pyrandom", &Value::Object(got), &load("pyrandom.json"));
    let (_, s) = snapshot();
    let M = audit::matrix(&s, srl::catalog::Variant::Strict);
    let B: serde_json::Map<String, Value> = (0..6u64)
        .map(|seed| {
            (
                seed.to_string(),
                json!(
                    bench::baskets(&M, seed)
                        .into_iter()
                        .map(|(c, r, n, P)| json!([c, r, n, P]))
                        .collect::<Vec<_>>()
                ),
            )
        })
        .collect();
    check("baskets", &Value::Object(B), &load("baskets.json"));
}

#[test]
#[ignore = "needs SRL_REF, the captured 0.2 reference directory"]
fn suite_seed1_instances() {
    let (_, s) = snapshot();
    let M = audit::matrix(&s, srl::catalog::Variant::Strict);
    let ref_suite = load("suite1.json");
    let B = bench::baskets(&M, 1);
    let mut d = Vec::new();
    let mut i = 0;
    for (c, rule, size, P) in &B {
        for k in [0usize, 1] {
            let x = &ref_suite[i];
            i += 1;
            assert_eq!(
                (
                    x["class"].as_str().unwrap(),
                    x["rule"].as_str().unwrap(),
                    x["size"].as_u64().unwrap() as usize,
                    x["k"].as_u64().unwrap() as usize
                ),
                (*c, *rule, *size, k)
            );
            let I = audit::basket(&M, P, k);
            let name = format!("{c}/{rule}/{size}/k{k}");
            diff(
                &format!("{name}.strict"),
                &record(&I, true, 300, &x["strict"]),
                &x["strict"],
                &mut d,
            );
            for v in ["merged", "identity"] {
                let J = instance(&x[v]);
                diff(
                    &format!("{name}.{v}"),
                    &record(&J, false, 0, &x[v]),
                    &x[v],
                    &mut d,
                );
            }
        }
    }
    assert_eq!(i, ref_suite.as_array().unwrap().len());
    assert!(
        d.is_empty(),
        "{} differences:\n{}",
        d.len(),
        d[..d.len().min(40)].join("\n")
    );
}

#[test]
#[ignore = "needs SRL_REF, the captured 0.2 reference directory"]
fn random_instances() {
    let mut d = Vec::new();
    for (i, x) in load("random.json").as_array().unwrap().iter().enumerate() {
        let I = instance(x);
        diff(
            &format!("random[{i}]"),
            &record(&I, true, 300, x),
            x,
            &mut d,
        );
    }
    assert!(
        d.is_empty(),
        "{} differences:\n{}",
        d.len(),
        d[..d.len().min(40)].join("\n")
    );
}
