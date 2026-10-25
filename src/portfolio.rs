//! Certified public cases. The browser selects recorded results and runs no solver.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::audit::{self, round, Matrix};
use crate::bench::{self, essential_json, infeasibility_json, LIMITS};
use crate::certify::{self, Completeness};
use crate::contracts;
use crate::instance::Instance;
use crate::json::{json, Map, Value};
use crate::report::{median, Num};
use crate::run::Clock;
use crate::solve::{self, Minimum};

pub const CASES: [(&str, &str, &str, [&str; 3]); 3] = [
    ("canned", "Three canned products", "Many alternatives, one precise coverage question.",
     ["beans pinto can", "beans vegetarian can", "tomato sauce can"]),
    ("poultry", "Three poultry products", "Overlapping qualifications permit a compact panel.",
     ["chicken leg qtr frz bag-4/10 lb", "chicken oven roasted frz 8 pc ctn-30 lb", "chicken small chilled-bulk"]),
    ("scarce", "A scarce-product basket", "A single-source product blocks backup coverage.",
     ["lentils dry bag", "pears halves ex lt can", "sweet potato fresh ctn 40"]),
];

/// Panels by (size, labels).
fn sorted(I: &Instance, C: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut C = C.to_vec();
    C.sort_by(|a, b| a.len().cmp(&b.len()).then(I.labels(a).cmp(&I.labels(b))));
    C
}

/// Keep an optimum, representatives by size, then maximise the least symmetric difference to
/// the panels kept (fewer members, then label order, on ties). An illustrative shortlist,
/// not a ranking. C is in (size, labels) order.
pub fn shortlist(I: &Instance, C: &[Vec<usize>], T: Option<&Vec<usize>>, n: usize) -> Vec<Vec<usize>> {
    let Some(T) = T else { return Vec::new() };
    let mut out = vec![T.clone()];
    let sizes: std::collections::BTreeSet<usize> = C.iter().map(Vec::len).collect();
    for r in sizes {
        let S = C.iter().find(|S| S.len() == r).unwrap();
        if !out.contains(S) && out.len() < n {
            out.push(S.clone());
        }
    }
    let sym = |a: &Vec<usize>, b: &Vec<usize>| a.iter().filter(|v| !b.contains(v)).count() + b.iter().filter(|v| !a.contains(v)).count();
    while out.len() < n.min(C.len()) {
        let key = |S: &Vec<usize>| (out.iter().map(|U| sym(S, U)).min().unwrap(), std::cmp::Reverse(S.len()));
        let mut best: Option<&Vec<usize>> = None;
        for S in C.iter().filter(|S| !out.contains(S)) {
            if best.is_none_or(|b| key(S) > key(b) || (key(S) == key(b) && I.labels(S) > I.labels(b))) {
                best = Some(S);
            }
        }
        out.push(best.unwrap().clone());
    }
    out
}

/// Product coverage of a fixed panel under one explicit outage, read from E_p.
pub fn coverage(I: &Instance, T: &[usize], F: &[usize]) -> Value {
    let E: Vec<Vec<String>> = I.E.iter().map(|Ep| I.labels(&Ep.iter().copied().filter(|v| T.contains(v) && !F.contains(v)).collect::<Vec<_>>())).collect();
    let uncovered: Vec<&String> = I.P.iter().zip(&E).filter(|(_, e)| e.is_empty()).map(|(p, _)| p).collect();
    json!({"suppliers": E, "uncovered": uncovered, "covered": E.iter().all(|e| !e.is_empty())})
}

fn witnesses(I: &Instance, S: &[usize]) -> Value {
    let W: Map<String, Value> = certify::witnesses(I, S).iter()
        .map(|(v, w)| (I.V[*v].clone(), w.as_ref().map_or(Value::Null, |w| bench::witness(I, w)))).collect();
    Value::Object(W)
}

/// A complete small case: optimum, certified family and an independent exhaustion search.
pub fn analyse(I: &Instance) -> Result<Value, String> {
    let (C, run) = bench::run("berge", I, 10.0, 10_000, &LIMITS);
    if run["status"] != "complete" || run["count"].as_u64() != Some(C.len() as u64) {
        return Err(format!("public case was not fully stored: {}", run["status"]));
    }
    if !C.iter().all(|T| certify::minimal(I, T)) {
        return Err("public case contains an uncertified panel".into());
    }
    let C = sorted(I, &C);
    let exhaustion = certify::complete(I, &C);
    let m = solve::minimum(I, &[], &Clock::limit(10.0));
    if exhaustion != Completeness::Complete || matches!(m, Minimum::Optimal(_)) != !C.is_empty() {
        return Err("public case failed independent exhaustion or optimum check".into());
    }
    if let Minimum::Optimal(T) = &m {
        if !certify::minimal(I, T) || T.len() != C[0].len() {
            return Err("public case minimum disagrees with its certified family".into());
        }
    }
    if let Minimum::Stopped(s) = m {
        return Err(format!("public case minimum unresolved: {}", s.name()));
    }
    let T = C.first();                                   // the first optimum in label order
    let mut sizes: BTreeMap<usize, usize> = BTreeMap::new();
    for S in &C {
        *sizes.entry(S.len()).or_default() += 1;
    }
    let family = contracts::update(contracts::document(I), Map::from_iter([
        ("kind".to_string(), json!("enumeration")), ("run".to_string(), run),
        ("panels".to_string(), json!(C.iter().map(|S| I.labels(S)).collect::<Vec<_>>()))]));
    Ok(json!({"status": bench::minimum_status(&m), "minimum_size": T.map(Vec::len), "count": C.len(),
              "sizes": sizes.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<Map<_, _>>(),
              "essential": essential_json(I), "infeasibility": infeasibility_json(I),
              "checks": {"all_minimal": true, "exhaustion": "complete", "minimum_matches_family": true},
              "family": family,
              "panels": shortlist(I, &C, T, 6).iter().map(|S| json!({"members": I.labels(S), "witnesses": witnesses(I, S)}))
                  .collect::<Vec<_>>()}))
}

/// Both uniform failure budgets and fixed-panel outage checks for a real basket.
pub fn case(M: &Matrix, names: &BTreeMap<String, String>, id: &str, title: &str, note: &str, P: &[&str]) -> Result<Value, String> {
    let P: Vec<String> = P.iter().map(|p| p.to_string()).collect();
    let I = audit::basket(M, &P, 0);
    let mut modes = Map::new();
    for k in [0, 1] {
        modes.insert(k.to_string(), analyse(&I.with_k(k))?);
    }
    let pair: Vec<String> = match modes["1"]["panels"].as_array().unwrap().first() {
        Some(p) => p["members"].as_array().unwrap().iter().take(2).map(|x| x.as_str().unwrap().into()).collect(),
        None => I.V.iter().take(2).cloned().collect(),
    };
    let mut outages = vec![json!({"id": "none", "label": "No outage", "failed": []})];
    outages.extend(I.V.iter().map(|v| json!({"id": v, "label": names[v], "failed": [v]})));
    outages.push(json!({"id": "pair", "label": "Two-supplier stress test", "failed": pair}));
    for m in modes.values_mut() {
        for panel in m["panels"].as_array_mut().unwrap() {
            let T: Vec<String> = panel["members"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().into()).collect();
            let T = I.index(&T).unwrap();
            let o: Map<String, Value> = outages.iter().map(|o| {
                let F: Vec<String> = o["failed"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().into()).collect();
                (o["id"].as_str().unwrap().to_string(), coverage(&I, &T, &I.index(&F).unwrap()))
            }).collect();
            panel["outages"] = Value::Object(o);
        }
    }
    Ok(json!({"id": id, "title": title, "note": note, "products": I.P,
              "suppliers": I.V.iter().enumerate().map(|(v, s)| json!({"id": s, "name": names[s],
                  "qualifications": I.E.iter().map(|E| E.contains(&v)).collect::<Vec<_>>()})).collect::<Vec<_>>(),
              "eligible": I.V.len(), "product_sizes": I.E.iter().map(Vec::len).collect::<Vec<_>>(),
              "outages": outages, "modes": modes}))
}

/// Keys of a saved record the explorer shows; 0.2 and 0.3 campaigns name the exhaustion
/// check differently, and 0.3 adds the class count.
fn keep(x: &Value) -> &'static [&'static str] {
    if x.get("exhaustion").is_some() || x.get("quotient").is_some() {
        &["class", "rule", "size", "k", "P", "min", "essential", "infeasibility", "runs", "count_exact",
          "complete_routes_agree", "exhaustion", "orbits", "sensitivity", "quotient", "quotient_agrees"]
    } else {
        &["class", "rule", "size", "k", "P", "min", "essential", "infeasibility", "runs", "count_exact",
          "complete_routes_agree", "milp_exhaustion", "orbits", "sensitivity"]
    }
}

/// One saved record, its minimum, essential set and infeasibility witness re-certified from
/// the matrix. Without a complete run the count is the lower bound max_r count_r.
pub fn outcome(x: &Value, M: &Matrix) -> Result<Value, String> {
    let P: Vec<String> = x["P"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().into()).collect();
    let I = audit::basket(M, &P, x["k"].as_u64().unwrap() as usize);
    let mut sizes: Vec<usize> = I.E.iter().map(Vec::len).collect();
    sizes.sort_unstable();
    if json!(sizes) != x["stats"]["E_p_sizes"] || json!(I.V.len()) != x["stats"]["V"] {
        return Err(format!("saved record disagrees with the matrix: {P:?}"));
    }
    let m = &x["min"];
    let ok = match m["panel"].as_array() {
        Some(T) => {
            let T: Vec<String> = T.iter().map(|v| v.as_str().unwrap().into()).collect();
            m["status"] == "optimal" && I.index(&T).is_some_and(|T| certify::minimal(&I, &T)) && json!(T.len()) == m["size"]
        }
        None => m["status"] == "infeasible" && certify::infeasibility(&I).is_some(),
    };
    if !ok {
        return Err(format!("saved minimum fails its matrix certificate: {P:?}"));
    }
    if infeasibility_json(&I) != x["infeasibility"] || essential_json(&I) != x["essential"] {
        return Err(format!("saved essential set or witness disagrees: {P:?}"));
    }
    let mut out: Map<String, Value> = keep(x).iter().map(|k| (k.to_string(), x.get(*k).cloned().unwrap_or(Value::Null))).collect();
    out.insert("V".into(), json!(I.V.len()));
    out.insert("E_p".into(), json!(I.E.iter().map(Vec::len).collect::<Vec<_>>()));
    out.insert("count_lower_bound".into(), json!(x["runs"].as_array().unwrap().iter().map(|r| r["count"].as_u64().unwrap()).max()));
    Ok(Value::Object(out))
}

/// Average ranks, ties sharing the mean of their positions (scipy rankdata "average").
fn ranks(x: &[f64]) -> Vec<f64> {
    let mut ix: Vec<usize> = (0..x.len()).collect();
    ix.sort_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap());
    let mut r = vec![0.0; x.len()];
    let mut i = 0;
    while i < ix.len() {
        let mut j = i;
        while j + 1 < ix.len() && x[ix[j + 1]] == x[ix[i]] {
            j += 1;
        }
        for &t in &ix[i..=j] {
            r[t] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

/// Spearman's ρ: Pearson's r of the average ranks.
pub fn spearman(x: &[f64], y: &[f64]) -> f64 {
    let (a, b) = (ranks(x), ranks(y));
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(&b).map(|(u, v)| (u - ma) * (v - mb)).sum();
    let va: f64 = a.iter().map(|u| (u - ma).powi(2)).sum();
    let vb: f64 = b.iter().map(|v| (v - mb).powi(2)).sum();
    cov / (va * vb).sqrt()
}

/// Summaries and every record of a saved campaign, retaining its revision and limits. The
/// records are re-certified, not rerun.
pub fn evidence(R: &Value, M: &Matrix, names: &BTreeMap<String, String>) -> Result<Value, String> {
    let X = R["instances"].as_array().unwrap();
    let records: Vec<Value> = X.iter().map(|x| outcome(x, M)).collect::<Result<_, _>>()?;
    let mut used: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for x in &records {
        if let Some(T) = x["min"]["panel"].as_array() {
            used.extend(T.iter().map(|v| v.as_str().unwrap().to_string()));
        }
        if let Some(e) = x["essential"].as_object() {
            used.extend(e.keys().cloned());
        }
    }
    let feasible: Vec<&Value> = records.iter().filter(|x| x["min"]["status"] == "optimal").collect();
    let primary = X.iter().filter(|x| ["uniform", "overlap"].contains(&x["rule"].as_str().unwrap()) && x["k"] == 1);
    let mut methods: Vec<String> = Vec::new();
    for r in X.iter().flat_map(|x| x["runs"].as_array().unwrap()) {
        let m = r["method"].as_str().unwrap().to_string();
        if !methods.contains(&m) {
            methods.push(m);
        }
    }
    let routes: Vec<Value> = methods.iter().map(|m| {
        let runs: Vec<&Value> = X.iter().flat_map(|x| x["runs"].as_array().unwrap()).filter(|r| r["method"] == m.as_str()).collect();
        let first: Vec<Num> = runs.iter().filter(|r| !r["t_first"].is_null()).map(|r| Num::of(&r["t_first"])).collect();
        json!({"method": m, "complete": runs.iter().filter(|r| r["status"] == "complete").count(), "runs": runs.len(),
               "first_median_s": median(first.clone()).f(), "first_n": first.len()})
    }).collect();
    let (vx, cy): (Vec<f64>, Vec<f64>) = feasible.iter().map(|x| (x["V"].as_f64().unwrap(), x["count_lower_bound"].as_f64().unwrap())).unzip();
    Ok(json!({"instances": X.len(), "resilient": primary.clone().filter(|x| x["min"]["status"] == "optimal").count(),
              "primary": primary.count(), "routes": routes, "provenance": R["provenance"],
              "names": used.iter().map(|v| (v.clone(), json!(names[v]))).collect::<Map<_, _>>(),
              "checks": {"matrix_records": records.len(), "partial": records.iter().filter(|x| x["count_exact"].is_null()).count()},
              "spearman_V_count": round(spearman(&vx, &cy), 2), "feasible": feasible.len(),
              "records": records}))
}

/// The distinct supplier names used, as a map.
pub fn names(s: &audit::Snapshot) -> BTreeMap<String, String> {
    s.S.iter().map(|(v, x)| (v.clone(), x.name.clone())).collect()
}

/// Identity candidates of the snapshot.
pub fn identity_candidates(s: &audit::Snapshot) -> usize {
    s.issues.iter().filter(|(_, k, _)| *k == "identity_candidate").count()
}

/// Every public case; fails when a certificate does not pass.
pub fn cases(M: &Matrix, names: &BTreeMap<String, String>) -> Result<Vec<Value>, String> {
    CASES.iter().map(|(id, t, n, P)| case(M, names, id, t, n, P)).collect()
}

/// SHA-256 over the crate sources (path, then LF-normalised bytes) in path order.
pub fn code_sha256(root: &Path) -> Result<String, String> {
    let mut files: Vec<PathBuf> = ["Cargo.toml", "Cargo.lock", "build.rs"].map(PathBuf::from).to_vec();
    let mut stack = vec![PathBuf::from("src")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(root.join(&d)).map_err(|e| e.to_string())? {
            let p = d.join(e.map_err(|e| e.to_string())?.file_name());
            if root.join(&p).is_dir() { stack.push(p) } else if p.extension().is_some_and(|x| x == "rs") { files.push(p) }
        }
    }
    let name = |p: &PathBuf| p.to_string_lossy().replace('\\', "/");
    files.sort_by_key(name);
    let mut buf = Vec::new();
    for p in &files {
        buf.extend(name(p).as_bytes());
        let b = std::fs::read(root.join(p)).map_err(|e| format!("{}: {e}", p.display()))?;
        buf.extend(String::from_utf8_lossy(&b).replace("\r\n", "\n").as_bytes());
    }
    Ok(crate::data::sha256(&buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spearman_with_ties() {
        let r = spearman(&[1.0, 2.0, 2.0, 4.0], &[1.0, 3.0, 2.0, 4.0]);
        assert!((r - 0.9486832980505138).abs() < 1e-12, "{r}");
        assert_eq!(ranks(&[3.0, 1.0, 3.0]), vec![2.5, 1.0, 2.5]);
    }
}
