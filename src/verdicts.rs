//! Pre-registered hypotheses H1–H5 and kill conditions K1–K7 (docs/findings/hypotheses.md),
//! scored from saved results and the snapshot audit only.

use crate::audit::round;
use crate::json::{json, Map, Value};
use crate::report::{median, Num};

const PRECISE: [&str; 4] = ["material", "naics", "program", "description"];
const VARIANTS: [&str; 2] = ["merged", "identity_candidates_merged"];

fn run<'a>(x: &'a Value, m: &str) -> &'a Value {
    x["runs"].as_array().unwrap().iter().find(|r| r["method"] == m).unwrap()
}

fn total(x: &Value, m: &str) -> f64 {
    let r = run(x, m);
    if r["status"] == "complete" { r["t_total"].as_f64().unwrap() } else { f64::INFINITY }
}

fn first(x: &Value, m: &str) -> f64 {
    run(x, m)["t_first"].as_f64().unwrap_or(f64::INFINITY)
}

/// The scores in their registered order. `A` is the audit of the snapshot the results name.
pub fn score(R: &Value, A: &Value) -> Vec<(String, Value)> {
    let X: Vec<&Value> = R["instances"].as_array().unwrap().iter().collect();
    let size = |x: &Value| x["min"]["size"].as_i64();
    let prim: Vec<&Value> = X.iter().copied().filter(|x| ["uniform", "overlap"].contains(&x["rule"].as_str().unwrap())).collect();
    let feas: Vec<&Value> = prim.iter().copied().filter(|x| size(x).is_some()).collect();
    let exact: Vec<&Value> = feas.iter().copied().filter(|x| !x["count_exact"].is_null()).collect();
    let sp = &A["suppliers_per_product"];
    let n3: u64 = PRECISE.iter().map(|c| sp[c][">=3"].as_u64().unwrap()).sum();
    let n: u64 = PRECISE.iter().map(|c| sp[c].as_object().unwrap().values().map(|v| v.as_u64().unwrap()).sum::<u64>()).sum();
    let small1: Vec<&Value> = prim.iter().copied().filter(|x| x["k"] == 1 && x["size"].as_u64().unwrap() <= 8).collect();
    let ok1 = small1.iter().filter(|x| size(x).is_some()).count();
    let many = exact.iter().filter(|x| x["count_exact"].as_u64().unwrap() >= 2).count();
    let ess_sub = feas.iter().filter(|x| (x["essential"].as_object().unwrap().len() as i64) < size(x).unwrap()).count();
    let ratio: Vec<f64> = X.iter().filter_map(|x| x.get("thesis").filter(|t| t.get("status").and_then(Value::as_str) == Some("dual")))
        .map(|t| t["tree_nodes"].as_f64().unwrap() / t["dag_nodes"].as_f64().unwrap()).collect();
    let comp: Vec<f64> = exact.iter().filter(|x| x["k"] == 1 && x.get("orbits").and_then(Value::as_u64).is_some_and(|o| o > 0))
        .map(|x| x["count_exact"].as_f64().unwrap() / x["orbits"].as_f64().unwrap()).collect();
    let runs: Vec<&Value> = prim.iter().flat_map(|x| x["runs"].as_array().unwrap()).collect();
    let stopped = runs.iter().filter(|r| ["timeout", "cap"].contains(&r["status"].as_str().unwrap())).count();
    let old = run(X[0], "berge").is_object() && X[0]["runs"].as_array().unwrap().iter().any(|r| r["method"] == "milp");
    let (cut, cut_name) = if old { ("milp", "MILP") } else { ("cuts", "the cut route") };
    let fkb_slowest = X.iter().filter(|x| total(x, "fkb") > total(x, "berge").min(total(x, cut))).count();
    let beats: Vec<&&Value> = X.iter().filter(|x| total(x, "fkb") < total(x, "berge")).collect();
    let late: Vec<&&Value> = X.iter().filter(|x| total(x, "berge") > 0.1 && size(x).is_some()).collect();
    let (mut h2v, mut dirn, mut flip, mut dmin) = (Map::new(), Map::new(), Map::new(), Map::new());
    for v in VARIANTS {
        let sf = |x: &Value| x.get("sensitivity").and_then(|s| s.get(v)).and_then(|s| s.get("feasible")).and_then(Value::as_bool);
        if prim.is_empty() || prim.iter().any(|x| sf(x).is_none() || !["optimal", "infeasible"].contains(&x["min"]["status"].as_str().unwrap())) {
            for m in [&mut h2v, &mut dirn, &mut flip, &mut dmin] {
                m.insert(v.into(), json!("not measured"));
            }
            continue;
        }
        h2v.insert(v.into(), json!(small1.iter().filter(|x| sf(x) == Some(true)).count()));
        dirn.insert(v.into(), json!({
            "infeasible->feasible": prim.iter().filter(|x| sf(x) == Some(true) && size(x).is_none()).count(),
            "feasible->infeasible": prim.iter().filter(|x| sf(x) == Some(false) && size(x).is_some()).count()}));
        flip.insert(v.into(), json!(prim.iter().filter(|x| sf(x) != Some(size(x).is_some())).count()));
        dmin.insert(v.into(), json!(prim.iter().filter(|x| sf(x) == Some(true) && size(x).is_some()
            && x["sensitivity"][v]["min"].as_i64() != size(x)).count()));
    }
    let tmin: Vec<Num> = X.iter().map(|x| Num::of(if x.get("t_min").is_some() { &x["t_min"] } else { &x["t_milp_min"] })).collect();
    let tmax = tmin.iter().map(|t| t.f()).fold(f64::NEG_INFINITY, f64::max);
    let rival = if old { "MILP minimum" } else { "minimum" };
    let num = |x: Num| match x { Num::I(i) => json!(i), Num::F(f) => json!(f) };
    vec![
        ("H1 precise products with >=3 suppliers".into(), json!(format!("{n3}/{n} = {:.3} (threshold 0.25)", n3 as f64 / n as f64))),
        ("H2 primary size<=8 baskets k=1 feasible".into(), json!(format!("{ok1}/{} (threshold 0.5)", small1.len()))),
        ("H3 feasible primary cases with exact |Tr|>=2".into(), json!(format!("{many}/{} (threshold 0.5); exact count unknown in {}",
                                                                            exact.len(), feas.len() - exact.len()))),
        ("H3 essential set strictly smaller than minimum".into(), json!(format!("{ess_sub}/{}", feas.len()))),
        ("H4 tree/DAG >= 2".into(), json!(if ratio.is_empty() { "no certified instance".to_string() } else {
            format!("{}/{}; median {:.2}", ratio.iter().filter(|&&r| r >= 2.0).count(), ratio.len(),
                    median(ratio.iter().map(|&r| Num::F(r)).collect()).f()) })),
        ("H4 signature compression >= 2 on k=1".into(), json!(format!("{}/{}", comp.iter().filter(|&&c| c >= 2.0).count(), comp.len()))),
        ("H5 / K7 feasibility verdict flips (primary)".into(), Value::Object(flip)),
        ("H5 / K7 flip directions".into(), Value::Object(dirn)),
        ("H5 / K7 H2 numerator under variant (of the same baskets)".into(), Value::Object(h2v)),
        ("H5 / K7 minimum changes where both feasible (primary)".into(), Value::Object(dmin)),
        ("K1 precise incidence share".into(), A["incidences_precise_share"].clone()),
        ("K3 size<=8 k=1 feasible share".into(), if small1.is_empty() { json!("not measured") } else {
            json!(round(ok1 as f64 / small1.len() as f64, 3)) }),
        ("K4 median exact |Tr| over feasible primary".into(), if exact.is_empty() { json!("not measured") } else {
            num(median(exact.iter().map(|x| Num::of(&x["count_exact"])).collect())) }),
        ("K5 primary runs stopped (timeout or cap)".into(), json!(format!("{stopped}/{}", runs.len()))),
        (format!("K6 instances where FK-B is slower than the faster of Berge and {cut_name}"), json!(format!("{fkb_slowest}/{}", X.len()))),
        ("K6 instances where FK-B beats Berge".into(), json!(if beats.is_empty() { "0".to_string() } else {
            format!("{}/{}; slowest Berge among them {:.4} s", beats.len(), X.len(),
                    beats.iter().map(|x| total(x, "berge")).fold(f64::NEG_INFINITY, f64::max)) })),
        ("first output: Berge > 0.1 s, FK-B first faster".into(), json!(format!("{}/{}",
            late.iter().filter(|x| first(x, "fkb") < first(x, "berge")).count(), late.len()))),
        (format!("rival: median {rival} time (s)"), json!(round(median(tmin).f(), 4))),
        (format!("rival: max {rival} time (s)"), json!(round(tmax, 4))),
    ]
}
