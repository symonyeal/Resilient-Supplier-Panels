//! Qualification gaps: which new approvals would make the most sampled baskets resilient.
//!
//! Without explicit scenarios a basket is feasible at level k iff each product alone is,
//! since T = V meets every E_p in min(|E_p|, k + 1) suppliers; so the blockers of a basket are
//! its products p with |E_p| ≤ k, each certified by `certify::infeasibility` on the basket {p}.
//! Lifting p takes k + 1 − |E_p| new approvals. The population is the campaign's primary
//! baskets (uniform ∪ overlap) over seeds 1..=n. Under the uniform rule the blockers of a
//! class are exchangeable, so differences between products come from the overlap rule; the
//! exact uniform share is stated separately.

use std::collections::BTreeMap;

use crate::audit::{self, Matrix};
use crate::bench::{self, CLASSES, SIZES};
use crate::certify;
use crate::json::{json, Map, Value};
use crate::quotient::binom;

/// The blockers of each primary basket of seeds 1..=n at level k.
fn population(M: &Matrix, n: u64, k: usize) -> Vec<Vec<String>> {
    let blocks = |p: &String| certify::infeasibility(&audit::basket(M, std::slice::from_ref(p), k)).is_some();
    let mut memo: BTreeMap<String, bool> = BTreeMap::new();
    (1..=n).flat_map(|seed| bench::baskets(M, seed)).filter(|b| b.1 != "scarce")
        .map(|b| b.3.into_iter().filter(|p| *memo.entry(p.clone()).or_insert_with(|| blocks(p))).collect())
        .collect()
}

/// Ranked gaps of the strict matrix M.
///
/// OUTPUT  population .. baskets, resilient now
///         products .... per blocker: class, |E_p|, approvals needed, baskets it blocks,
///                       baskets it alone blocks; ordered by the last, then the second
///         greedy ...... products lifted in turn, each the one completing most baskets
///                       (ties: most blocked, then name), with cumulative approvals and
///                       resilient baskets, until every basket is resilient
///         uniform ..... per class: products, blockers, and for each basket size s the
///                       exact share C(n − u, s) / C(n, s) of resilient uniform baskets
pub fn gaps(M: &Matrix, n: u64, k: usize) -> Value {
    let B = population(M, n, k);
    let need = |p: &str| k + 1 - M[p].1.len();
    let (mut blocked, mut alone): (BTreeMap<&str, usize>, BTreeMap<&str, usize>) = Default::default();
    for b in &B {
        for p in b {
            *blocked.entry(p).or_default() += 1;
        }
        if let [p] = b.as_slice() {
            *alone.entry(p).or_default() += 1;
        }
    }
    let mut ranked: Vec<&str> = blocked.keys().copied().collect();
    ranked.sort_by_key(|p| (std::cmp::Reverse((alone.get(p).copied().unwrap_or(0), blocked[p])), *p));
    let products: Vec<Value> = ranked.iter().map(|p| json!({
        "product": p, "class": M[*p].0, "suppliers": M[*p].1.len(), "approvals": need(p),
        "blocks": blocked[p], "alone": alone.get(p).copied().unwrap_or(0)})).collect();

    // greedy lifting; open[i] = blockers of basket i still unlifted
    let mut open: Vec<Vec<&str>> = B.iter().map(|b| b.iter().map(String::as_str).collect()).collect();
    let mut done = open.iter().filter(|b| b.is_empty()).count();
    let (mut cost, mut greedy) = (0, Vec::new());
    while done < open.len() {
        let mut score: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for b in open.iter().filter(|b| !b.is_empty()) {
            for p in b {
                let e = score.entry(p).or_default();
                e.0 += (b.len() == 1) as usize;
                e.1 += 1;
            }
        }
        // the best score; BTreeMap order makes the first maximum the earliest name
        let (p, s) = score.iter().fold(None, |best: Option<(&str, (usize, usize))>, (p, s)| match best {
            Some((_, t)) if t >= *s => best,
            _ => Some((p, *s)),
        }).expect("an open basket has a blocker");
        for b in open.iter_mut() {
            b.retain(|q| *q != p);
        }
        done += s.0;
        cost += need(p);
        greedy.push(json!({"product": p, "approvals": cost, "resilient": done,
                           "share": audit::round(done as f64 / open.len() as f64, 4)}));
    }

    let uniform: Vec<Value> = CLASSES.iter().map(|c| {
        let ps: Vec<&String> = M.iter().filter(|(_, (cl, _))| cl == c).map(|(p, _)| p).collect();
        let u = ps.iter().filter(|p| M[**p].1.len() <= k).count();
        let share: Map<String, Value> = SIZES.iter().filter(|&&s| s <= ps.len()).map(|&s| {
            (s.to_string(), json!(audit::round(binom(ps.len() - u, s) as f64 / binom(ps.len(), s) as f64, 6)))
        }).collect();
        json!({"class": c, "products": ps.len(), "blockers": u, "share": share})
    }).collect();
    json!({"k": k, "seeds": n, "population": {"baskets": B.len(), "resilient": B.iter().filter(|b| b.is_empty()).count()},
           "products": products, "greedy": greedy, "uniform": uniform})
}
