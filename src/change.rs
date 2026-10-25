//! Change between dated QBL snapshots (docs/DATA.md, "Snapshots").
//!
//! Suppliers are matched across workbooks by the identity key of `qbl::records` (name,
//! billing address, ZIP5), because supplier ids are sheet rows; products by their strict
//! name under the one catalog. Only the precise classes (bench::CLASSES) are compared: they
//! are the classes baskets are drawn from. A backup is a second approved supplier.

use std::collections::{BTreeMap, BTreeSet};

use crate::audit::{self, Matrix, Snapshot};
use crate::bench::{self, CLASSES};
use crate::certify;
use crate::json::{json, Value};
use crate::qbl::{self, Supplier};

/// Identity key of a supplier: (name, billing address, ZIP5), as `qbl::records` forms it.
pub type Key = (String, String, String);

pub fn key(s: &Supplier) -> Key {
    (qbl::clean(&s.name), s.address.clone(), s.zip5.clone())
}

/// Precise product -> its suppliers' keys (strict variant).
fn supply(s: &Snapshot) -> BTreeMap<String, BTreeSet<Key>> {
    audit::matrix(s, "strict").into_iter()
        .filter(|(_, (c, _))| CLASSES.contains(&c.as_str()))
        .map(|(p, (_, E))| (p, E.iter().map(|v| key(&s.S[v])).collect()))
        .collect()
}

fn split<T: Ord + Clone>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> Value {
    json!({"before": a.len(), "after": b.len(), "kept": a.intersection(b).count(),
           "entered": b.difference(a).count(), "left": a.difference(b).count()})
}

/// What changed from snapshot a to snapshot b.
///
/// OUTPUT  suppliers, products ... before, after, kept, entered, left (products: precise)
///         single ................ precise products with one supplier, before and after
///         gained, lost .......... kept products whose supplier count went 1 → ≥ 2, ≥ 2 → 1
///         approvals ............. qualifications on kept products added and removed
pub fn step(a: &Snapshot, b: &Snapshot) -> Value {
    let (Sa, Sb) = (supply(a), supply(b));
    let ids = |s: &Snapshot| -> BTreeSet<Key> { s.S.values().map(key).collect() };
    let names = |S: &BTreeMap<String, BTreeSet<Key>>| -> BTreeSet<String> { S.keys().cloned().collect() };
    let single = |S: &BTreeMap<String, BTreeSet<Key>>| S.values().filter(|E| E.len() == 1).count();
    let (mut gained, mut lost, mut added, mut removed) = (Vec::new(), Vec::new(), 0, 0);
    for (p, Ea) in &Sa {
        let Some(Eb) = Sb.get(p) else { continue };
        added += Eb.difference(Ea).count();
        removed += Ea.difference(Eb).count();
        if Ea.len() == 1 && Eb.len() >= 2 {
            gained.push(p.clone());
        } else if Ea.len() >= 2 && Eb.len() == 1 {
            lost.push(p.clone());
        }
    }
    json!({"suppliers": split(&ids(a), &ids(b)), "products": split(&names(&Sa), &names(&Sb)),
           "single": {"before": single(&Sa), "after": single(&Sb)},
           "gained": gained, "lost": lost, "approvals": {"added": added, "removed": removed}})
}

/// Feasibility of basket P at level k on matrix M, from the matrix certificate; "absent" when
/// a product is not in M.
pub fn status(M: &Matrix, P: &[String], k: usize) -> &'static str {
    if !P.iter().all(|p| M.contains_key(p)) {
        "absent"
    } else if certify::infeasibility(&audit::basket(M, P, k)).is_some() {
        "infeasible"
    } else {
        "feasible"
    }
}

/// Primary baskets (uniform ∪ overlap) of a seed on a matrix.
fn primary(M: &Matrix, seed: u64) -> Vec<Vec<String>> {
    bench::baskets(M, seed).into_iter().filter(|b| b.1 != "scarce").map(|b| b.3).collect()
}

/// The snapshot series: per snapshot its size and the k = 1 status of (a) the primary baskets
/// the campaign drew on the last snapshot and (b) those the same seed draws on its own
/// matrix; per consecutive pair, `step`.
///
/// INPUT  W .... [(name, date, snapshot)] in date order
pub fn series(W: &[(String, String, Snapshot)], seed: u64) -> Value {
    let M: Vec<Matrix> = W.iter().map(|w| audit::matrix(&w.2, "strict")).collect();
    let fixed = primary(M.last().expect("a snapshot"), seed);
    let tally = |m: &Matrix, B: &[Vec<String>]| {
        let s: Vec<&str> = B.iter().map(|P| status(m, P, 1)).collect();
        json!({"baskets": B.len(), "absent": s.iter().filter(|x| **x == "absent").count(),
               "feasible": s.iter().filter(|x| **x == "feasible").count()})
    };
    let snaps: Vec<Value> = W.iter().zip(&M).map(|((name, date, s), m)| {
        let precise = m.values().filter(|(c, _)| CLASSES.contains(&c.as_str()));
        json!({"name": name, "date": date, "suppliers": s.S.len(),
               "precise_products": precise.clone().count(),
               "single": precise.filter(|(_, E)| E.len() == 1).count(),
               "campaign_baskets": tally(m, &fixed), "own_baskets": tally(m, &primary(m, seed))})
    }).collect();
    let changed: Vec<Value> = fixed.iter().filter_map(|P| {
        let s: Vec<&str> = M.iter().map(|m| status(m, P, 1)).collect();
        s.windows(2).any(|w| w[0] != w[1]).then(|| json!({"P": P, "status": s}))
    }).collect();
    let steps: Vec<Value> = W.windows(2).map(|w| {
        let mut d = step(&w[0].2, &w[1].2);
        d["from"] = json!(w[0].0);
        d["to"] = json!(w[1].0);
        d
    }).collect();
    json!({"seed": seed, "k": 1, "snapshots": snaps, "steps": steps, "campaign_baskets_changed": changed})
}
