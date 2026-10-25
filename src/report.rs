//! Findings tables, rebuilt from saved result JSON only (never from a live run).
//!
//! Reads both the 0.2 Python campaigns (route `milp`, keys `milp_exhaustion`, `t_milp_min`)
//! and the 0.3 Rust campaigns (routes `cuts`, `mmcs`, keys `exhaustion`, `t_min`, `quotient`).
//! Numbers are formatted as Python formatted them, so a saved report rebuilds byte for byte.

use crate::json::{self, Value};

/// A Python number as `str()` shows it: ints plainly, floats by repr.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Num {
    I(i64),
    F(f64),
}

impl Num {
    pub fn of(v: &Value) -> Num {
        if v.is_f64() { Num::F(v.as_f64().unwrap()) } else { Num::I(v.as_i64().unwrap()) }
    }

    pub fn f(self) -> f64 {
        match self {
            Num::I(i) => i as f64,
            Num::F(x) => x,
        }
    }
}

impl std::fmt::Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Num::I(i) => write!(f, "{i}"),
            Num::F(x) => write!(f, "{}", json::float(*x)),
        }
    }
}

/// statistics.median: the middle value, or the mean of the middle two (then a float).
pub fn median(mut xs: Vec<Num>) -> Num {
    xs.sort_by(|a, b| a.f().partial_cmp(&b.f()).unwrap());
    let n = xs.len();
    if n % 2 == 1 { xs[n / 2] } else { Num::F((xs[n / 2 - 1].f() + xs[n / 2].f()) / 2.0) }
}

/// Python `format(x, ".{p}g")`.
pub fn g(x: f64, p: usize) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = format!("{:.*e}", p - 1, x);
    let (m, ex) = e.split_once('e').unwrap();
    let X: i32 = ex.parse().unwrap();
    let strip = |s: String| if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if X < -4 || X >= p as i32 {
        format!("{}e{}{:02}", strip(m.to_string()), if X < 0 { '-' } else { '+' }, X.abs())
    } else {
        strip(format!("{:.*}", (p as i32 - 1 - X) as usize, x))
    }
}

/// Python `str()` of a JSON value inside a container (strings quoted).
pub fn repr(v: &Value) -> String {
    match v {
        Value::String(s) => format!("'{s}'"),
        Value::Object(o) => format!("{{{}}}", o.iter().map(|(k, x)| format!("'{k}': {}", repr(x)))
            .collect::<Vec<_>>().join(", ")),
        Value::Array(a) => format!("[{}]", a.iter().map(repr).collect::<Vec<_>>().join(", ")),
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True".into() } else { "False".into() },
        Value::Number(_) => Num::of(v).to_string(),
    }
}

/// Python `str()` of a value at top level.
pub fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => repr(v),
    }
}

/// A run's output count; a stopped run gives a lower bound, never a count.
pub fn count(r: &Value) -> String {
    if r["status"] == "complete" {
        r["count"].to_string()
    } else {
        format!("≥{} ({})", r["count"], r["status"].as_str().unwrap())
    }
}

/// Essential suppliers come from the matrix witness (V \ {v} infeasible), so they need no
/// complete enumeration.
pub fn essential(rec: &Value) -> String {
    match rec["essential"].as_object() {
        None => "— (infeasible)".into(),
        Some(e) => format!("{} (matrix witness)", e.len()),
    }
}

fn sec(t: &Value) -> String {
    t.as_f64().map_or("—".into(), |t| g(t, 3))
}

/// Panels up to signature symmetry: from the stored complete family, else (0.3 campaigns)
/// from the class count, which equals it wherever both exist (MODEL, Theorem 4).
pub fn orbits(r: &Value) -> Option<&Value> {
    r.get("orbits").or_else(|| r.get("quotient").and_then(|q| q.get("orbits")))
}

/// A proved minimum size, or the solve status when no optimum was returned.
fn minimum(r: &Value) -> String {
    if r["status"] == "optimal" { r["size"].to_string() } else { r["status"].as_str().unwrap().into() }
}

/// Markdown table titled by its claim.
pub fn exhibit(title: &str, head: &[&str], rows: Vec<String>, note: Option<String>) -> Vec<String> {
    let mut out = vec![format!("### {title}"), String::new()];
    if let Some(n) = note {
        out.push(n);
        out.push(String::new());
    }
    out.push(format!("| {} |", head.join(" | ")));
    out.push(format!("|{}", "---|".repeat(head.len())));
    out.extend(rows);
    out.push(String::new());
    out
}

fn basket_id(r: &Value) -> String {
    format!("{}/{}/{}", r["class"].as_str().unwrap(), r["rule"].as_str().unwrap(), r["size"])
}

fn min_count(r: &Value) -> String {
    let lead = |s: &String| s.trim_start_matches('≥').split(' ').next().unwrap().parse::<i64>().unwrap();
    let mut best: Option<String> = None;
    for u in r["runs"].as_array().unwrap() {
        let c = count(u);
        if best.as_ref().is_none_or(|b| lead(&c) > lead(b)) {
            best = Some(c);
        }
    }
    best.unwrap()
}

pub fn tables(res: &Value) -> String {
    let X = res["instances"].as_array().unwrap();
    let by = |b: &str, k: u64| X.iter().find(|r| basket_id(r) == b && r["k"] == k).unwrap();
    let mut ids: Vec<String> = X.iter().map(basket_id).collect();
    ids.sort_by_key(|s| {
        let p: Vec<&str> = s.split('/').collect();
        (p[0].to_string(), p[1].to_string(), p[2].parse::<i64>().unwrap())
    });
    ids.dedup();
    let primary: Vec<&String> = ids.iter().filter(|b| ["uniform", "overlap"].contains(&b.split('/').nth(1).unwrap())).collect();
    let is_primary = |b: &String| primary.contains(&b);
    let mut out = Vec::new();

    // Exhibit 1: minimum panel, k = 0 against k = 1
    let (mut rows, mut inc) = (Vec::new(), Vec::new());
    for b in &ids {
        let (r0, r1) = (by(b, 0), by(b, 1));
        if let (Some(m0), Some(m1)) = (r0["min"]["size"].as_i64(), r1["min"]["size"].as_i64()) {
            if is_primary(b) {
                inc.push(Num::I(m1 - m0));
            }
        }
        let w = &r1["infeasibility"];
        rows.push(format!("| {b} | {} | {} | {} | {} |", r0["stats"]["V"], minimum(&r0["min"]), minimum(&r1["min"]),
                          if w.is_null() { String::new() } else { w[0].as_str().unwrap().to_string() }));
    }
    let feas = primary.iter().filter(|b| !by(b, 1)["min"]["size"].is_null()).count();
    let unknown = primary.iter().filter(|b| !["optimal", "infeasible"].contains(&by(b, 1)["min"]["status"].as_str().unwrap())).count();
    let note = if unknown > 0 { format!(" ({unknown} unresolved)") } else { String::new() };
    let dash = |x: Option<String>| x.unwrap_or("—".into());
    let head = format!("k = 1 is feasible for {feas} of {} primary baskets{note}; where feasible it adds a median of {} suppliers (range {}–{})",
                       primary.len(), dash((!inc.is_empty()).then(|| median(inc.clone()).to_string())),
                       dash(inc.iter().map(|x| x.f() as i64).min().map(|x| x.to_string())),
                       dash(inc.iter().map(|x| x.f() as i64).max().map(|x| x.to_string())));
    out.extend(exhibit(&head, &["basket (class/rule/size)", "eligible suppliers", "min panel k=0", "min panel k=1",
                                 "k=1 blocked by (single-supplier product)"], rows, None));

    // Exhibit 2: alternatives and essential suppliers
    let (mut rows, mut many) = (Vec::new(), 0);
    for b in &ids {
        for k in [0, 1] {
            let r = by(b, k);
            if r["min"]["size"].is_null() {
                continue;
            }
            let c = &r["count_exact"];
            many += (c.as_i64().is_some_and(|c| c >= 2) && is_primary(b)) as usize;
            rows.push(format!("| {b} | {k} | {} | {} | {} | {} |", r["min"]["size"],
                              if c.is_null() { min_count(r) } else { c.to_string() },
                              orbits(r).map_or("—".into(), |o| o.to_string()), essential(r)));
        }
    }
    out.extend(exhibit(&format!("{many} feasible primary basket/k cases have at least 2 minimal panels (complete count)"),
                       &["basket", "k", "min size", "minimal panels", "up to signature symmetry", "essential suppliers"],
                       rows, None));

    // Exhibit 3: routes
    let mut rows = Vec::new();
    let mut stat: Vec<(String, Vec<bool>)> = Vec::new();
    for b in &ids {
        for k in [0, 1] {
            let r = by(b, k);
            for run in r["runs"].as_array().unwrap() {
                let m = run["method"].as_str().unwrap();
                let c = run["status"] == "complete";
                match stat.iter_mut().find(|(n, _)| n == m) {
                    Some((_, v)) => v.push(c),
                    None => stat.push((m.into(), vec![c])),
                }
            }
            if r["min"]["size"].is_null() {
                continue;
            }
            let cells: Vec<String> = r["runs"].as_array().unwrap().iter().map(|u| format!("{} · {} / {} / {}",
                count(u), sec(&u["t_first"]), sec(&u["t_K"]), sec(&u["t_total"]))).collect();
            rows.push(format!("| {b} | {k} | {} |", cells.join(" | ")));
        }
    }
    let done: Vec<String> = stat.iter().map(|(m, v)| format!("{m} {}/{}", v.iter().filter(|&&c| c).count(), v.len())).collect();
    let mut head = vec!["basket", "k"];
    head.extend(stat.iter().map(|(m, _)| m.as_str()));
    out.extend(exhibit(&format!("Complete runs within the limit: {}", done.join(", ")), &head, rows,
                       Some("Each cell: count · time to first / to K=10 / total (s). '≥n (timeout)' is censored.".into())));

    // Exhibit 4: thesis tree against memoised DAG
    let (mut rows, mut ratios) = (Vec::new(), Vec::new());
    for b in &ids {
        for k in [0, 1] {
            let t = &by(b, k)["thesis"];
            if t.get("status").and_then(Value::as_str) != Some("dual") {
                continue;
            }
            let ratio = t["tree_nodes"].as_f64().unwrap() / t["dag_nodes"].as_f64().unwrap();
            ratios.push(Num::F(ratio));
            rows.push(format!("| {b} | {k} | {} | {} | {ratio:.2} | {} | {} |", t["tree_nodes"], t["dag_nodes"],
                              sec(&t["t_tree"]), sec(&t["t_memo"])));
        }
    }
    if !ratios.is_empty() {
        let n = ratios.len();
        out.extend(exhibit(&format!("FK-A memoisation: median tree/DAG node ratio {:.2} over {n} certified instances",
                                    median(ratios).f()),
                           &["basket", "k", "FK-A tree nodes", "distinct subproblems", "ratio", "t tree (s)", "t memo (s)"],
                           rows, None));
    }

    // Exhibit 5: sensitivity
    let (mut rows, mut flips, mut unknown) = (Vec::new(), 0, 0);
    for b in &ids {
        for k in [0, 1] {
            let r = by(b, k);
            let Some(s) = r.get("sensitivity").filter(|s| s.as_object().is_some_and(|o| !o.is_empty())) else { continue };
            let base = !r["min"]["size"].is_null();
            let resolved = ["optimal", "infeasible"].contains(&r["min"]["status"].as_str().unwrap());
            let mut cells = Vec::new();
            for name in ["merged", "identity_candidates_merged"] {
                let v = &s[name];
                if !v["feasible"].is_null() && resolved {
                    flips += (v["feasible"].as_bool() != Some(base) && is_primary(b)) as usize;
                } else {
                    unknown += is_primary(b) as usize;
                }
                cells.push(if v["feasible"] == true { v["min"].to_string() } else { v["status"].as_str().unwrap().to_string() });
            }
            rows.push(format!("| {b} | {k} | {} | {} |", minimum(&r["min"]), cells.join(" | ")));
        }
    }
    if rows.is_empty() {
        out.extend(["Sensitivity variants were not measured.".to_string(), String::new()]);
    } else {
        let note = (unknown > 0).then(|| format!("{unknown} primary comparisons unresolved."));
        out.extend(exhibit(&format!("Feasibility verdict changes in {flips} primary cases under the two sensitivity variants"),
                           &["basket", "k", "min (strict)", "min (merged products)", "min (identity candidates merged)"],
                           rows, note));
    }

    // Exhibit 6 (0.3 campaigns): exact counts by signature classes
    if X.iter().any(|r| r.get("quotient").is_some()) {
        let mut rows = Vec::new();
        let (mut exact, mut counted, mut equal, mut stored, mut same) = (0, 0, 0, 0, 0);
        for b in &ids {
            for k in [0, 1] {
                let r = by(b, k);
                let q = &r["quotient"];
                exact += (q["status"] == "complete") as usize;
                let enumerated = (q["status"] == "complete" && !r["count_exact"].is_null()).then(|| q["count"] == r["count_exact"]);
                counted += enumerated.is_some() as usize;
                equal += (enumerated == Some(true)) as usize;
                if let Some(a) = r.get("quotient_agrees").and_then(Value::as_bool) {
                    stored += 1;
                    same += a as usize;
                }
                if r["min"]["size"].is_null() {
                    continue;
                }
                rows.push(format!("| {b} | {k} | {} | {} | {} | {} |", show(&q["count"]), q["orbits"],
                                  if r["count_exact"].is_null() { min_count(r) } else { r["count_exact"].to_string() },
                                  enumerated.map_or("—", |a| if a { "yes" } else { "no" })));
            }
        }
        out.extend(exhibit(&format!("Exact counts by signature classes on {exact} of {} instances: equal to the enumerated count on {equal} of {counted}",
                                    X.len()),
                           &["basket", "k", "minimal panels (classes)", "class vectors", "enumerated", "equal"], rows,
                           Some(format!("Count and class vectors also equal those of the stored complete family on {same} of {stored}."))));
    }
    out.join("\n")
}

/// The report text for a saved campaign; `score` supplies the pre-registered tests.
pub fn markdown(res: &Value, name: &str, score: Vec<(String, Value)>) -> String {
    let p = &res["provenance"];
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let lang = if p.get("python").is_some() { format!("Python {}", s("python")) } else { format!("Rust {}", s("rustc")) };
    let mut md = format!("# Results: seed {}\n\nGenerated by `srl report {name}` from saved results only.\n\n\
        Source: QBL snapshot `{}…`; code `{}`{}; thesis `{}`; limits {} s, cap {}; {}, {}, {lang}. \
        Suite wall time {:.0} s. Assumptions: strict mapping, all eligible suppliers, k-failure model; \
        qualification only (no capacity, price or lead time).\n\n{}",
        p["seed"], &s("data_sha256")[..12], &s("code_rev")[..7], if p["dirty"] == true { " (dirty)" } else { "" },
        &s("thesis_rev")[..7], Num::of(&p["limits"]["timeout_s"]), p["limits"]["cap"], s("cpu"), s("os"),
        res["t_suite"].as_f64().unwrap(), tables(res));
    md += &exhibit("Pre-registered tests (docs/findings/hypotheses.md)", &["test", "result"],
                   score.iter().map(|(k, v)| format!("| {k} | {} |", show(v))).collect(), None).join("\n");
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json;

    #[test]
    fn python_g() {
        for (x, s) in [(0.000174, "0.000174"), (1.2e-5, "1.2e-05"), (6.7, "6.7"), (1234.5, "1.23e+03"),
                       (0.1, "0.1"), (100.0, "100"), (999.6, "1e+03"), (0.0009995, "0.000999")] {
            assert_eq!(g(x, 3), s, "{x}");
        }
    }

    #[test]
    fn partial_counts_are_never_exact() {
        assert_eq!(count(&json!({"status": "complete", "count": 12000, "stored": 10000})), "12000");
        assert_eq!(count(&json!({"status": "timeout", "count": 41})), "≥41 (timeout)");
        assert_eq!(count(&json!({"status": "cap", "count": 10001})), "≥10001 (cap)");
        assert_eq!(essential(&json!({"essential": {"a": ["p", []]}})), "1 (matrix witness)");
        assert_eq!(essential(&json!({"essential": null})), "— (infeasible)");
    }

    #[test]
    fn python_str() {
        assert_eq!(show(&json!({"merged": 4, "x": "not measured", "d": {"a": 0}})),
                   "{'merged': 4, 'x': 'not measured', 'd': {'a': 0}}");
        assert_eq!(median(vec![Num::I(10), Num::I(12)]).to_string(), "11.0");
        assert_eq!(median(vec![Num::I(3), Num::I(1), Num::I(2)]).to_string(), "2");
    }
}
