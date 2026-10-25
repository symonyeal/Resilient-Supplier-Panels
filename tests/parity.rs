//! Two-way agreement with reference outputs of the Python 0.2 release on the thesis code at
//! 1bb9de4. The reference directory is named by SRL_REF; without it these tests are skipped.
//! Each test reports every differing path, not only the first.

use std::path::PathBuf;

use srl::audit::{self, Snapshot};
use srl::bench::{self, essential_json, infeasibility_json};
use srl::bits::Bits;
use srl::certify;
use srl::instance::{decode, encode, hypergraph, Instance};
use srl::json::{json, Value};
use srl::pyrand::Random;
use srl::run::Clock;
use srl::thesis::{berge, fka, fkb, Mbf};
use srl::{catalog, data, qbl, quotient, solve};

fn reference() -> Option<PathBuf> {
    std::env::var_os("SRL_REF").map(PathBuf::from)
}

fn load(name: &str) -> Value {
    let p = reference().unwrap().join(name);
    srl::json::loads(&std::fs::read(&p).unwrap()).unwrap()
}

/// Paths at which a and b differ (object key order ignored; numbers by value).
fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
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

fn check(name: &str, rust: &Value, python: &Value) {
    let mut d = Vec::new();
    diff(name, rust, python, &mut d);
    assert!(d.is_empty(), "{} differences, first ones:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}

#[test]
fn comparator_detects_planted_differences() {
    let a = json!({"x": [1, 2, 3], "y": {"z": "a"}, "f": 0.5});
    let mut d = Vec::new();
    diff("t", &a, &a.clone(), &mut d);
    assert!(d.is_empty());
    for b in [json!({"x": [1, 3, 2], "y": {"z": "a"}, "f": 0.5}), json!({"x": [1, 2, 3], "y": {"z": "b"}, "f": 0.5}),
              json!({"x": [1, 2, 3], "y": {}, "f": 0.5}), json!({"x": [1, 2], "y": {"z": "a"}, "f": 0.5}),
              json!({"x": [1, 2, 3], "y": {"z": "a"}, "f": 0.25})] {
        let mut d = Vec::new();
        diff("t", &a, &b, &mut d);
        assert!(!d.is_empty(), "{b}");
    }
}

fn snapshot() -> (Vec<qbl::Row>, Snapshot) {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = audit::snapshot(&rows, &data::text("catalog/mappings.csv").unwrap());
    (rows, s)
}

#[test]
fn data_layer() {
    if reference().is_none() {
        return;
    }
    let (rows, s) = snapshot();
    let R: Vec<Value> = rows.iter().map(|r| {
        let mut m: serde_json::Map<String, Value> = r.f.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        m.insert("row".into(), json!(r.row));
        Value::Object(m)
    }).collect();
    check("rows", &json!(R), &load("rows.json"));
    let S: serde_json::Map<String, Value> = s.S.iter().map(|(k, x)| (k.clone(), json!({
        "name": x.name, "address": x.address, "city": x.city, "state": x.state, "zip5": x.zip5,
        "size": x.size, "rows": x.rows}))).collect();
    let Q: Vec<Value> = s.Q.iter().map(|(a, b, c)| json!([a, b, c])).collect();
    let issues: Vec<Value> = s.issues.iter().map(|(a, b, c)| json!([a, b, c])).collect();
    check("records", &json!({"S": S, "Q": Q, "issues": issues}), &load("records.json"));
    let D: serde_json::Map<String, Value> = s.D.iter().map(|(t, e)| (t.clone(),
        json!([e.class, e.strict, e.merged, e.justification]))).collect();
    check("catalog", &Value::Object(D), &load("catalog.json"));
    check("merged_of", &json!(catalog::merged_of(&s.D)), &load("merged_of.json"));
    check("audit", &audit::audit(&rows, &s), &load("audit.json"));
    for (v, f) in [("strict", "matrix-strict.json"), ("merged", "matrix-merged.json")] {
        let M: serde_json::Map<String, Value> = audit::matrix(&s, v).into_iter()
            .map(|(p, (c, E))| (p, json!([c, E]))).collect();
        check(v, &Value::Object(M), &load(f));
    }
}

#[test]
fn python_random_and_baskets() {
    if reference().is_none() {
        return;
    }
    let mut got = serde_json::Map::new();
    for seed in [0u64, 1, 2, 12345, (1 << 40) + 7] {
        let mut xs: Vec<usize> = (0..30).collect();
        Random::new(seed).shuffle(&mut xs);
        let mut g = Random::new(seed);
        let mut h = Random::new(seed);
        got.insert(seed.to_string(), json!({
            "bits32": [Random::new(seed).getrandbits(32)],
            "seq32": (0..8).map(|_| g.getrandbits(32)).collect::<Vec<_>>(),
            "shuffle30": xs,
            "sample": Random::new(seed).sample(&(0..100).collect::<Vec<_>>(), 20),
            "sample_small": Random::new(seed).sample(&(0..10).collect::<Vec<_>>(), 3),
            "choice": [Random::new(seed).choice(&(0..1000).collect::<Vec<_>>())],
            "randint": (0..10).map(|_| h.randint(1, 9)).collect::<Vec<_>>()}));
    }
    check("pyrandom", &Value::Object(got), &load("pyrandom.json"));
    let (_, s) = snapshot();
    let M = audit::matrix(&s, "strict");
    let B: serde_json::Map<String, Value> = (0..6u64).map(|seed| (seed.to_string(),
        json!(bench::baskets(&M, seed).into_iter().map(|(c, r, n, P)| json!([c, r, n, P])).collect::<Vec<_>>()))).collect();
    check("baskets", &Value::Object(B), &load("baskets.json"));
}

fn labels(I: &Instance, C: &[Vec<usize>]) -> Value {
    let mut L: Vec<Vec<String>> = C.iter().map(|T| I.labels(T)).collect();
    L.sort();
    json!(L)
}

/// The reference record of an instance, computed by the Rust port.
fn record(I: &Instance, deep: bool, fkb_cap: usize, rec: &Value) -> Value {
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

fn instance(r: &Value) -> Instance {
    let s = |v: &Value| v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let ss = |v: &Value| v.as_array().unwrap().iter().map(s).collect::<Vec<_>>();
    Instance::new(s(&r["V"]), s(&r["P"]), ss(&r["E"]), r["k"].as_i64().unwrap(), ss(&r["F"])).unwrap()
}

#[test]
fn suite_seed1_instances() {
    if reference().is_none() {
        return;
    }
    let (_, s) = snapshot();
    let M = audit::matrix(&s, "strict");
    let ref_suite = load("suite1.json");
    let B = bench::baskets(&M, 1);
    let mut d = Vec::new();
    let mut i = 0;
    for (c, rule, size, P) in &B {
        for k in [0usize, 1] {
            let x = &ref_suite[i];
            i += 1;
            assert_eq!((x["class"].as_str().unwrap(), x["rule"].as_str().unwrap(), x["size"].as_u64().unwrap() as usize, x["k"].as_u64().unwrap() as usize),
                       (*c, *rule, *size, k));
            let I = audit::basket(&M, P, k);
            let name = format!("{c}/{rule}/{size}/k{k}");
            diff(&format!("{name}.strict"), &record(&I, true, 300, &x["strict"]), &x["strict"], &mut d);
            for v in ["merged", "identity"] {
                let J = instance(&x[v]);
                diff(&format!("{name}.{v}"), &record(&J, false, 0, &x[v]), &x[v], &mut d);
            }
        }
    }
    assert_eq!(i, ref_suite.as_array().unwrap().len());
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}

#[test]
fn random_instances() {
    if reference().is_none() || !reference().unwrap().join("random.json").exists() {
        return;
    }
    let mut d = Vec::new();
    for (i, x) in load("random.json").as_array().unwrap().iter().enumerate() {
        let I = instance(x);
        diff(&format!("random[{i}]"), &record(&I, true, 300, x), x, &mut d);
    }
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d[..d.len().min(40)].join("\n"));
}

#[test]
fn verdicts_and_site_from_the_saved_campaign() {
    if reference().is_none() || !reference().unwrap().join("site/cases.json").exists() {
        return;
    }
    let (rows, s) = snapshot();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let R = srl::json::loads(&std::fs::read(root.join("results/suite-seed1.json")).unwrap()).unwrap();
    let V: serde_json::Map<String, Value> = srl::verdicts::score(&R, &audit::audit(&rows, &s)).into_iter().collect();
    check("verdicts", &Value::Object(V), &load("verdicts.json"));
    let M = audit::matrix(&s, "strict");
    let names = srl::portfolio::names(&s);
    let site = load("site/cases.json");
    check("evidence", &srl::portfolio::evidence(&R, &M, &names).unwrap(), &site["evidence"]);
    // cases: the run timings are new measurements and the exhaustion key is renamed
    let strip = |mut c: Value| {
        for m in c["modes"].as_object_mut().unwrap().values_mut() {
            for k in ["t_first", "t_K", "t_total", "exitcode"] {
                m["family"]["run"].as_object_mut().unwrap().remove(k);
            }
            let ch = m["checks"].as_object_mut().unwrap();
            if let Some(e) = ch.remove("milp_exhaustion") {
                ch.insert("exhaustion".into(), e);
            }
        }
        c
    };
    // the shortlist starts from an optimum: HiGHS's choice in Python, the first in label order
    // here. With Python's optimum the shortlist is reproduced; the rest is compared without it.
    let rust: Vec<Value> = srl::portfolio::cases(&M, &names).unwrap();
    let python: Vec<Value> = site["cases"].as_array().unwrap().clone();
    let mut changed = Vec::new();
    for (r, p) in rust.iter().zip(&python) {
        let P: Vec<String> = p["products"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().into()).collect();
        for (k, m) in p["modes"].as_object().unwrap() {
            let I = audit::basket(&M, &P, k.parse().unwrap());
            let ix = |v: &Value| I.index(&v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().into()).collect::<Vec<_>>()).unwrap();
            let C: Vec<Vec<usize>> = m["family"]["panels"].as_array().unwrap().iter().map(ix).collect();
            let T = m["panels"].as_array().unwrap().first().map(|x| ix(&x["members"]));
            let got: Vec<Value> = srl::portfolio::shortlist(&I, &C, T.as_ref(), 6).iter().map(|S| json!(I.labels(S))).collect();
            let want: Vec<Value> = m["panels"].as_array().unwrap().iter().map(|x| x["members"].clone()).collect();
            check(&format!("shortlist {} k={k}", p["id"]), &json!(got), &json!(want));
            if r["modes"][k]["panels"] != m["panels"] {
                changed.push(format!("{} k={k}: first panel {} -> {}", p["id"], m["panels"][0]["members"], r["modes"][k]["panels"][0]["members"]));
            }
        }
    }
    println!("shortlists changed by the deterministic optimum: {changed:?}");
    let drop = |mut c: Value| {
        for m in c["modes"].as_object_mut().unwrap().values_mut() {
            m.as_object_mut().unwrap().remove("panels");
        }
        c["outages"].as_array_mut().unwrap().retain(|o| o["id"] != "pair");
        strip(c)
    };
    check("cases", &json!(rust.into_iter().map(drop).collect::<Vec<_>>()), &json!(python.into_iter().map(drop).collect::<Vec<_>>()));
}
