//! Benchmark suite on the frozen snapshot (docs/BENCHMARKS.md).
//!
//! Baskets: per granularity class, size s ∈ SIZES, rule ∈ {uniform, overlap, scarce}, seed
//! (Python's Mersenne Twister, so seeds give the 0.2 baskets). Each enumeration runs on its
//! own thread under a deadline and an output cap; a stopped run is partial and reported as
//! censored, a panicked one as an error.

use std::collections::{BTreeSet, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::audit::{self, round, Matrix, Snapshot};
use crate::certify::{self, Completeness};
use crate::instance::{decode, encode, hypergraph, Instance};
use crate::json::{json, Map, Value};
use crate::pyrand::Random;
use crate::run::{Clock, Stop};
use crate::solve::{self, Minimum};
use crate::thesis::{berge, fka, fkb, Mbf};
use crate::{mmcs, quotient};

pub const SIZES: [usize; 5] = [3, 5, 8, 12, 20];
pub const CLASSES: [&str; 4] = ["description", "program", "naics", "material"];
pub const ROUTES: [&str; 4] = ["berge", "fkb", "cuts", "mmcs"];

/// Limits of a campaign (docs/BENCHMARKS.md); they change only there, before a run.
#[derive(Clone, Debug)]
pub struct Limits {
    pub timeout_s: f64,
    pub cap: usize,
    pub K: usize,
    pub fka_nodes: usize,
    pub fka_max_family: usize,
    pub exhaustive_support: usize,
    pub repeat_below_s: f64,
    pub repeats: usize,
    /// Berge's working family, in sets.
    pub berge_sets: usize,
}

pub const LIMITS: Limits = Limits { timeout_s: 10.0, cap: 10_000, K: 10, fka_nodes: 200_000,
                                    fka_max_family: 2_000, exhaustive_support: 18,
                                    repeat_below_s: 1.0, repeats: 3, berge_sets: 10_000_000 };

impl Limits {
    pub fn json(&self) -> Value {
        json!({"timeout_s": self.timeout_s, "cap": self.cap, "K": self.K, "fka_nodes": self.fka_nodes,
               "fka_max_family": self.fka_max_family, "exhaustive_support": self.exhaustive_support,
               "repeat_below_s": self.repeat_below_s, "repeats": self.repeats,
               "berge_sets": self.berge_sets})
    }
}

pub type Basket = (&'static str, &'static str, usize, Vec<String>);

/// [(class, rule, s, P)] for one seed, drawn as bench.baskets drew them.
pub fn baskets(M: &Matrix, seed: u64) -> Vec<Basket> {
    let mut rng = Random::new(seed);
    let mut out = Vec::new();
    for c in CLASSES {
        let prods: Vec<String> = M.iter().filter(|(_, (cl, _))| cl == c).map(|(p, _)| p.clone()).collect();
        let starts: Vec<String> = prods.iter().filter(|p| M[*p].1.len() >= 2).cloned().collect();
        for s in SIZES {
            if prods.len() < s {
                continue;
            }
            let mut P = rng.sample(&prods, s);
            P.sort();
            out.push((c, "uniform", s, P));
            if starts.is_empty() {
                continue;
            }
            let mut P = vec![rng.choice(&starts).clone()];
            while P.len() < s {                          // greedy Jaccard to the union
                let U: BTreeSet<&String> = P.iter().flat_map(|p| &M[p].1).collect();
                let mut rest: Vec<&String> = prods.iter().filter(|q| !P.contains(q)).collect();
                rng.shuffle(&mut rest);
                let jac = |q: &String| {
                    let E = &M[q].1;
                    let both = E.iter().filter(|v| U.contains(v)).count();
                    both as f64 / (E.len() + U.len() - both) as f64
                };
                let mut best = rest[0];
                for q in &rest[1..] {
                    if jac(q) > jac(best) {
                        best = q;
                    }
                }
                P.push(best.clone());
            }
            P.sort();
            out.push((c, "overlap", s, P));
        }
        let scarce: Vec<String> = prods.iter().filter(|p| M[*p].1.len() <= 2).cloned().collect();
        if scarce.len() >= 3 {
            let mut P = rng.sample(&scarce, 3);
            P.sort();
            out.push((c, "scarce", 3, P));
        }
    }
    out
}

/// Runs a route to exhaustion or a stop; `emit` returns false to stop the stream. Berge
/// emits only after its fold has finished.
pub fn route(method: &str, I: &Instance, clock: &Clock, L: &Limits,
             emit: &mut dyn FnMut(Vec<usize>) -> bool) -> Result<(), Stop> {
    if method == "cuts" {
        return solve::cut_stream(I, clock, emit);
    }
    let n = I.support().len();
    crate::with_width!(n, W => {
        let S = hypergraph::<W>(I);
        let mut out = |t: crate::bits::Bits<W>| emit(decode(&S.labels, &t));
        match method {
            "berge" => {
                for t in berge::transversal(&S.H, clock, L.berge_sets)? {
                    out(t);
                }
                Ok(())
            }
            "fkb" => fkb::dualise(&S.H, clock, &mut out).map(|_| ()),
            "mmcs" => mmcs::transversals(&S.H, clock, &mut out),
            _ => panic!("unknown route {method}"),
        }
    })
}

/// One enumeration of Tr(H) under the limits.
///
/// OUTPUT  panels . the first ≤ cap panels received
///         r ...... status complete | timeout | cap | memory | error; count = panels
///                  received, stored = panels kept. Berge emits only after it has finished,
///                  so it is never stopped at the cap: its count is exact, its storage capped.
pub fn run(method: &str, I: &Instance, timeout: f64, cap: usize, L: &Limits) -> (Vec<Vec<usize>>, Value) {
    assert!(timeout.is_finite() && timeout > 0.0, "timeout must be finite and positive");
    let (mut panels, mut times, mut n) = (Vec::new(), Vec::new(), 0usize);
    let clock = Clock::limit(timeout);
    let res = catch_unwind(AssertUnwindSafe(|| {
        route(method, I, &clock, L, &mut |T| {
            n += 1;
            if n <= cap {
                panels.push(T);
                times.push(clock.elapsed());
            }
            n <= cap || method == "berge"
        })
    }));
    let t_end = clock.elapsed();
    let status = match &res {
        Ok(Ok(())) => "complete",
        Ok(Err(s)) => s.name(),
        Err(_) => "error",
    };
    let t = |i: usize| times.get(i).map_or(Value::Null, |&x| json!(x));
    let r = json!({"method": method, "status": status, "count": n, "stored": panels.len(),
                   "t_first": t(0), "t_K": if L.K >= 1 { t(L.K - 1) } else { Value::Null },
                   "t_total": if status == "complete" { json!(t_end) } else { Value::Null }});
    (panels, r)
}

fn timed(method: &str, I: &Instance, L: &Limits) -> f64 {
    let clock = Clock::unlimited();
    route(method, I, &clock, L, &mut |_| true).expect("a run already known to finish");
    clock.elapsed()
}

/// Structure, and the signature classes (suppliers with equal membership in every E_p and F).
pub fn stats(I: &Instance) -> (Value, Vec<Vec<usize>>) {
    let n = I.support().len();
    let edges = crate::with_width!(n, W => hypergraph::<W>(I).H.len());
    let mut sizes: Vec<usize> = I.E.iter().map(Vec::len).collect();
    sizes.sort_unstable();
    let mut deg: Vec<usize> = (0..I.V.len()).map(|v| I.E.iter().filter(|E| E.contains(&v)).count()).collect();
    deg.sort_unstable();
    let sig = quotient::classes(I);
    let inc: usize = sizes.iter().sum();
    let binom = |n: usize, r: usize| (0..r).fold(1u128, |a, i| a * (n - i) as u128 / (i + 1) as u128);
    let before: u128 = I.E.iter().map(|E| binom(E.len(), I.k.min(E.len())) + I.F.len() as u128).sum();
    (json!({"P": I.P.len(), "V": I.V.len(), "incidences": inc,
            "density": round(inc as f64 / (I.P.len() * I.V.len()).max(1) as f64, 4),
            "E_p_sizes": sizes,
            "supplier_degree_min_median_max": if deg.is_empty() { json!([]) } else {
                json!([deg[0], deg[deg.len() / 2], deg[deg.len() - 1]]) },
            "H_edges_after_reduction": edges,
            "H_edges_before_reduction": before as u64,
            "signature_classes": sig.len(),
            "largest_class": sig.iter().map(Vec::len).max().unwrap_or(0)}), sig)
}

/// |C| up to permutations within signature classes: the number of distinct class multisets.
pub fn orbits(C: &[Vec<usize>], sig: &[Vec<usize>]) -> usize {
    let mut cls = vec![0; sig.iter().flatten().max().map_or(0, |m| m + 1)];
    for (i, vs) in sig.iter().enumerate() {
        for &v in vs {
            cls[v] = i;
        }
    }
    C.iter().map(|T| {
        let mut m: Vec<usize> = T.iter().map(|&v| cls[v]).collect();
        m.sort_unstable();
        m
    }).collect::<HashSet<_>>().len()
}

/// FK-A decides C = Tr(H) (duality certificate); tree nodes against fk_a_memo's distinct
/// subproblems on the same recursion.
pub fn thesis_check(I: &Instance, C: &[Vec<usize>], nodes: usize) -> Value {
    let n = I.support().len();
    crate::with_width!(n, W => {
        let S = hypergraph::<W>(I);
        let Cb = Mbf::new(S.H.n, C.iter().map(|T| encode(&S.labels, T)).collect());
        let t0 = Clock::unlimited();
        let Some((v, tau)) = fka::fk_a(&Cb, &S.H, nodes) else { return json!({"status": "node_limit"}) };
        let t_tree = t0.elapsed();
        let t1 = Clock::unlimited();
        let (dual, delta, _) = fka::fk_a_memo(&Cb, &S.H);
        json!({"status": if v.dual { "dual" } else { "not_dual" }, "tree_nodes": tau,
               "dag_nodes": delta, "memo_agrees": dual == v.dual, "t_tree": t_tree,
               "t_memo": t1.elapsed()})
    })
}

/// Labels of an index set, sorted.
fn lab(I: &Instance, T: &[usize]) -> Value {
    json!(I.labels(T))
}

pub fn witness(I: &Instance, w: &certify::Witness) -> Value {
    json!([I.P[w.0], I.labels(&w.1)])
}

pub fn essential_json(I: &Instance) -> Value {
    match certify::essential(I) {
        None => Value::Null,
        Some(W) => {
            let mut m: Vec<(String, Value)> = W.iter().map(|(v, w)| (I.V[*v].clone(), witness(I, w))).collect();
            m.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(m.into_iter().collect())
        }
    }
}

pub fn infeasibility_json(I: &Instance) -> Value {
    certify::infeasibility(I).map_or(Value::Null, |w| witness(I, &w))
}

pub fn minimum_status(m: &Minimum) -> String {
    match m {
        Minimum::Optimal(_) => "optimal".into(),
        Minimum::Infeasible => "infeasible".into(),
        Minimum::Stopped(s) => s.name().into(),
    }
}

/// Minimum, certificates, every route under the limits, checks of complete families, and
/// the exact count by signature classes.
pub fn instance_record(I: &Instance, L: &Limits) -> Value {
    let t0 = Clock::unlimited();
    let (st, sig) = stats(I);
    let mut rec = Map::new();
    rec.insert("stats".into(), st);
    rec.insert("t_build".into(), json!(t0.elapsed()));
    let t0 = Clock::unlimited();
    let m = solve::minimum(I, &[], &Clock::limit(L.timeout_s));
    rec.insert("t_min".into(), json!(t0.elapsed()));
    let t0 = Clock::unlimited();
    let infeasible = certify::infeasibility(I);
    rec.insert("infeasibility".into(), infeasibility_json(I));
    let T = match &m { Minimum::Optimal(T) => Some(T.clone()), _ => None };
    rec.insert("min".into(), json!({"status": minimum_status(&m), "size": T.as_ref().map(Vec::len),
                                    "panel": T.as_ref().map(|T| lab(I, T)),
                                    "certified_minimal": T.as_ref().is_some_and(|T| certify::minimal(I, T))}));
    let ess = certify::essential(I);
    rec.insert("essential".into(), essential_json(I));
    rec.insert("t_certify_min".into(), json!(t0.elapsed()));
    rec.insert("solver_agrees_with_witness".into(), match m {
        Minimum::Stopped(_) => Value::Null,
        _ => json!(matches!(m, Minimum::Infeasible) == infeasible.is_some()),
    });
    let mut runs = Vec::new();
    let mut full: Vec<HashSet<Vec<usize>>> = Vec::new();
    for method in ROUTES {
        let (panels, mut r) = run(method, I, L.timeout_s, L.cap, L);
        let complete = r["status"] == "complete";
        if complete && r["t_total"].as_f64().unwrap() < L.repeat_below_s {
            let mut reps = vec![r["t_total"].clone()];
            reps.extend((1..L.repeats).map(|_| json!(timed(method, I, L))));
            r["t_total_repeats"] = json!(reps);
        }
        let t0 = Clock::unlimited();
        r["all_certified_minimal"] = json!(panels.iter().all(|T| certify::minimal(I, T)));
        r["t_certify"] = json!(t0.elapsed());
        if complete && r["stored"] == r["count"] {
            full.push(panels.into_iter().collect());
        }
        runs.push(r);
    }
    let exact = runs.iter().find(|r| r["status"] == "complete").map(|r| r["count"].clone());
    rec.insert("count_exact".into(), exact.unwrap_or(Value::Null));
    rec.insert("complete_routes_agree".into(),
               if full.is_empty() { Value::Null } else { json!(full.iter().all(|C| *C == full[0])) });
    let t0 = Clock::unlimited();
    let q = quotient::count(I, &Clock::limit(L.timeout_s));
    let tq = t0.elapsed();
    rec.insert("quotient".into(), match &q {
        Ok(c) => json!({"status": "complete", "count": count_json(c.panels), "orbits": c.orbits,
                        "reduced_panels": c.reduced, "t": tq}),
        Err(s) => json!({"status": s.name(), "t": tq}),
    });
    if let Some(C) = full.first() {
        let mut C: Vec<Vec<usize>> = C.iter().cloned().collect();
        C.sort();
        let t0 = Clock::unlimited();
        rec.insert("exhaustion".into(), json!(match certify::complete(I, &C) {
            Completeness::Complete => "complete",
            Completeness::Missing(_) => "missing",
            Completeness::Invalid => "invalid",
        }));
        rec.insert("t_exhaustion".into(), json!(t0.elapsed()));
        if I.support().len() <= L.exhaustive_support {
            let mut X = solve::exhaustive(I);
            X.sort();
            rec.insert("exhaustive_agrees".into(), json!(X == C));
        }
        let o = orbits(&C, &sig);
        rec.insert("orbits".into(), json!(o));
        if let Ok(c) = &q {
            rec.insert("quotient_agrees".into(), json!(c.panels == C.len() as u128 && c.orbits == o));
        }
        rec.insert("thesis".into(), if !C.is_empty() && C.len() <= L.fka_max_family {
            thesis_check(I, &C, L.fka_nodes) } else { Value::Null });
        if let Some(e) = &ess {
            let inter: BTreeSet<usize> = I.support().into_iter().filter(|v| C.iter().all(|T| T.contains(v))).collect();
            rec.insert("essential_matches_family".into(), json!(inter == e.keys().copied().collect()));
        }
    }
    rec.insert("runs".into(), json!(runs));
    Value::Object(rec)
}

pub fn count_json(c: u128) -> Value {
    u64::try_from(c).map_or_else(|_| json!(c.to_string()), |x| json!(x))
}

/// Sensitivity variant: feasibility and minimum only.
pub fn sens(J: &Instance, L: &Limits) -> Value {
    let m = solve::minimum(J, &[], &Clock::limit(L.timeout_s));
    json!({"min": match &m { Minimum::Optimal(T) => json!(T.len()), _ => Value::Null },
           "feasible": match m { Minimum::Stopped(_) => Value::Null, _ => json!(matches!(m, Minimum::Optimal(_))) },
           "status": minimum_status(&m)})
}

/// Every basket of `seed` at k ∈ {0, 1}; sensitivity variants by one minimum solve each.
/// `log` receives one line per instance.
pub fn suite(seed: u64, L: &Limits, s: &Snapshot, provenance: Value, variants: bool,
             log: &mut dyn FnMut(&str)) -> Value {
    let t0 = Clock::unlimited();
    let M = audit::matrix(s, "strict");
    let MM = audit::matrix(s, "merged");
    let merged = crate::catalog::merged_of(&s.D);
    let first: std::collections::HashMap<usize, &String> = s.S.iter().map(|(sid, x)| (x.rows[0], sid)).collect();
    let rep: std::collections::HashMap<&String, &String> = s.issues.iter()
        .filter(|(_, k, _)| *k == "identity_candidate").map(|(r, _, d)| (first[r], d)).collect();
    let t_matrix = t0.elapsed();
    let t_start = Clock::unlimited();
    let mut instances = Vec::new();
    for (c, rule, size, P) in baskets(&M, seed) {
        for k in [0, 1] {
            let I = audit::basket(&M, &P, k);
            let mut rec = json!({"class": c, "rule": rule, "size": size, "k": k, "P": P, "variant": "strict"});
            for (key, v) in instance_record(&I, L).as_object().unwrap() {
                rec[key] = v.clone();
            }
            if variants {
                let Ec: Vec<Vec<String>> = I.E.iter().map(|E| {
                    let s: BTreeSet<String> = E.iter().map(|&v| {
                        let id = &I.V[v];
                        rep.get(id).map_or(id.clone(), |d| (*d).clone())
                    }).collect();
                    s.into_iter().collect()
                }).collect();
                let Vc: BTreeSet<String> = Ec.iter().flatten().cloned().collect();
                let Ic = Instance::new(Vc.into_iter().collect(), I.P.clone(), Ec, k as i64, vec![]).unwrap();
                let Pm: BTreeSet<String> = P.iter().map(|p| merged[p].clone()).collect();
                let Im = audit::basket(&MM, &Pm.into_iter().collect::<Vec<_>>(), k);
                rec["sensitivity"] = json!({"merged": sens(&Im, L), "identity_candidates_merged": sens(&Ic, L)});
            }
            let runs: Vec<String> = rec["runs"].as_array().unwrap().iter()
                .map(|r| format!("{}:{}/{}", r["method"].as_str().unwrap(), r["status"].as_str().unwrap(), r["count"]))
                .collect();
            log(&format!("{c:11} {rule:7} s={size:2} k={k} V={:3} min={} {} quotient={}", I.V.len(),
                         rec["min"]["size"], runs.join(" "), rec["quotient"]["count"]));
            instances.push(rec);
        }
    }
    json!({"provenance": provenance, "t_matrix": t_matrix, "instances": instances,
           "t_suite": t_start.elapsed()})
}
