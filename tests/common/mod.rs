//! Comparison of Rust output with captured thesis-code references, shared by the
//! reference tests.
#![allow(dead_code)]

use srl::audit::{self, Snapshot};
use srl::bench::{self, essential_json, infeasibility_json};
use srl::bits::Bits;
use srl::certify;
use srl::instance::{decode, encode, hypergraph, Instance};
use srl::json::{json, Value};
use srl::run::Clock;
use srl::thesis::{berge, fka, fkb, Mbf};
use srl::{data, qbl, quotient, solve};

/// Paths at which a and b differ (object key order ignored; numbers by value).
pub fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                match (x.get(k), y.get(k)) {
                    (Some(u), Some(v)) => diff(&format!("{path}.{k}"), u, v, out),
                    (u, _) => out.push(format!("{path}.{k}: only {}", if u.is_some() { "rust" } else { "python" })),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                diff(&format!("{path}[{i}]"), u, v, out);
            }
        }
        (Value::Number(x), Value::Number(y)) if x.as_f64() == y.as_f64() => {}
        _ if a == b => {}
        _ => {
            let s = |v: &Value| { let s = v.to_string(); if s.len() > 160 { format!("{}…", &s[..160]) } else { s } };
            out.push(format!("{path}: rust {} python {}", s(a), s(b)));
        }
    }
}

pub fn check(name: &str, rust: &Value, python: &Value) {
    let mut d = Vec::new();
    diff(name, rust, python, &mut d);
    assert!(d.is_empty(), "{} differences, first ones:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}

pub fn snapshot() -> (Vec<qbl::Row>, Snapshot) {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = audit::snapshot(&rows, &data::text("catalog/mappings.csv").unwrap());
    (rows, s)
}

pub fn labels(I: &Instance, C: &[Vec<usize>]) -> Value {
    let mut L: Vec<Vec<String>> = C.iter().map(|T| I.labels(T)).collect();
    L.sort();
    json!(L)
}

/// The reference record of an instance, computed by the Rust port.
pub fn record(I: &Instance, deep: bool, fkb_cap: usize, rec: &Value) -> Value {
    let n = I.support().len();
    let mut out = srl::with_width!(n, W => {
        let S = hypergraph::<W>(I);
        let H: Vec<Value> = S.H.edges.iter().zip(&S.prov).map(|(e, pv)| json!([
            I.labels(&decode(&S.labels, e)),
            pv.iter().map(|(p, F)| json!([I.P[*p], I.labels(F)])).collect::<Vec<_>>()])).collect();
        let (st, sig) = bench::stats(I);
        let m = solve::minimum(I, &[], &Clock::unlimited());
        let mut sig: Vec<Vec<String>> = sig.iter().map(|c| I.labels(c)).collect();
        sig.sort();
        let mut r = json!({"V": I.V, "P": I.P, "E": I.E.iter().map(|E| I.labels(E)).collect::<Vec<_>>(),
            "k": I.k, "F": I.F.iter().map(|F| I.labels(F)).collect::<Vec<_>>(),
            "support": I.support().iter().map(|&v| I.V[v].clone()).collect::<Vec<_>>(),
            "H": H, "stats": st, "sig": sig, "infeasibility": infeasibility_json(I),
            "essential": essential_json(I), "min_status": bench::minimum_status(&m),
            "min_size": match &m { solve::Minimum::Optimal(T) => json!(T.len()), _ => Value::Null }});
        if deep {
            let C: Vec<Vec<usize>> = berge::transversal(&S.H, &Clock::unlimited(), usize::MAX).unwrap()
                .iter().map(|t| decode(&S.labels, t)).collect();
            r["berge_status"] = json!("complete");
            r["berge"] = labels(I, &C);
            let mut Cs = C.clone();
            Cs.sort_by(|a, b| a.len().cmp(&b.len()).then(I.labels(a).cmp(&I.labels(b))));
            r["witnesses"] = json!(Cs.iter().take(50).map(|T| {
                let wit: serde_json::Map<String, Value> = certify::witnesses(I, T).iter().map(|(v, w)|
                    (I.V[*v].clone(), w.as_ref().map_or(Value::Null, |w| bench::witness(I, w)))).collect();
                Value::Object(wit)
            }).collect::<Vec<_>>());
            r["orbits"] = json!(bench::orbits(&C, &bench::stats(I).1));
            if !C.is_empty() && C.len() <= 2000 {
                let enc = |C: &[Vec<usize>]| Mbf::new(S.H.n, C.iter().map(|T| encode(&S.labels, T)).collect());
                let (v, tau) = fka::fk_a(&enc(&C), &S.H, 200_000).unwrap();
                let (dual, delta, depth) = fka::fk_a_memo(&enc(&C), &S.H);
                let mut th = json!({"dual": v.dual, "tree_nodes": tau, "dag_nodes": delta,
                                    "memo_dual": dual, "depth": depth});
                if C.len() > 1 {
                    let drop = Cs[0].clone();
                    let Cm: Vec<Vec<usize>> = C.iter().filter(|T| **T != drop).cloned().collect();
                    let (v2, t2) = fka::fk_a(&enc(&Cm), &S.H, 200_000).unwrap();
                    th["minus_one"] = json!({"dropped": I.labels(&drop), "dual": v2.dual, "tree_nodes": t2,
                        "CA": v2.CA.map(|S2: Bits<W>| I.labels(&decode(&S.labels, &S2))),
                        "dag_nodes": fka::fk_a_memo(&enc(&Cm), &S.H).1});
                }
                r["thesis"] = th;
            }
            let mut prefix = Vec::new();
            let mut nodes = Vec::new();
            let status = {
                let mut k = 0;
                let res = fkb::dualise(&S.H, &Clock::unlimited(), &mut |t| {
                    k += 1;
                    if k <= fkb_cap { prefix.push(json!(I.labels(&decode(&S.labels, &t)))); }
                    k <= fkb_cap
                });
                if let Ok(n) = &res { nodes = n.clone(); }
                if res.is_ok() { "complete" } else { "cap" }
            };
            r["fkb_prefix"] = json!(prefix);
            r["fkb_status"] = json!(status);
            if rec.get("dualise_nodes").is_some() {
                r["dualise_nodes"] = json!(nodes);
                r["dualise"] = r["berge"].clone();
            }
            if rec.get("exhaustive").is_some() {
                r["exhaustive"] = labels(I, &solve::exhaustive(I));
            }
            // the new routes must give the same family
            let q = quotient::count(I, &Clock::unlimited()).unwrap();
            assert_eq!((q.panels, q.orbits), (C.len() as u128, r["orbits"].as_u64().unwrap() as usize));
            if C.len() <= 2000 {
                let mut cuts = Vec::new();
                solve::cut_stream(I, &Clock::unlimited(), &mut |T| { cuts.push(T); true }).unwrap();
                assert_eq!(labels(I, &cuts), r["berge"]);
                assert_eq!(certify::complete(I, &C), certify::Completeness::Complete);
            }
        }
        r
    });
    if let Some(o) = out.as_object_mut() {
        o.retain(|k, _| rec.get(k).is_some());
    }
    out
}

pub fn instance(r: &Value) -> Instance {
    let s = |v: &Value| v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let ss = |v: &Value| v.as_array().unwrap().iter().map(s).collect::<Vec<_>>();
    Instance::new(s(&r["V"]), s(&r["P"]), ss(&r["E"]), r["k"].as_i64().unwrap(), ss(&r["F"])).unwrap()
}
