//! Qualification matrices of the snapshot and the Stage 0 audit of what they support
//! (docs/DATA.md).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::catalog::{self, Entry};
use crate::instance::Instance;
use crate::json::{json, Map, Value};
use crate::qbl::{self, Issue, Row, Supplier};

/// product -> (granularity class, E_p).
pub type Matrix = BTreeMap<String, (String, BTreeSet<String>)>;

pub struct Snapshot {
    pub S: BTreeMap<String, Supplier>,
    pub Q: Vec<(String, String, usize)>,
    pub issues: Vec<Issue>,
    pub D: BTreeMap<String, Entry>,
}

/// Records, qualifications, issues and the product dictionary of a workbook.
pub fn snapshot(rows: &[Row], mappings: &str) -> Snapshot {
    let (S, Q, issues) = qbl::records(rows);
    let tokens: BTreeSet<String> = Q.iter().map(|q| q.1.clone()).collect();
    let D = catalog::load(&tokens, mappings);
    Snapshot { S, Q, issues, D }
}

/// The qualification matrix of a variant ("strict" or "merged"); a product's class is that
/// of the first qualification naming it.
pub fn matrix(s: &Snapshot, variant: &str) -> Matrix {
    let mut M: Matrix = BTreeMap::new();
    for (sid, t, _) in &s.Q {
        let e = &s.D[t];
        for p in if variant == "strict" { &e.strict } else { &e.merged } {
            M.entry(p.clone()).or_insert_with(|| (e.class.clone(), BTreeSet::new())).1.insert(sid.clone());
        }
    }
    M
}

/// Instance for basket P with every eligible supplier: V = ∪ E_p.
pub fn basket(M: &Matrix, P: &[String], k: usize) -> Instance {
    let E: Vec<Vec<String>> = P.iter().map(|p| M[p].1.iter().cloned().collect()).collect();
    let V: BTreeSet<String> = E.iter().flatten().cloned().collect();
    Instance::new(V.into_iter().collect(), P.to_vec(), E, k as i64, Vec::new()).expect("matrix basket")
}

fn hist(xs: impl Iterator<Item = usize>, top: usize) -> Value {
    let mut h = vec![0usize; top + 1];
    for x in xs {
        h[x.min(top)] += 1;
    }
    let mut m = Map::new();
    for (n, c) in h.into_iter().enumerate() {
        m.insert(if n < top { n.to_string() } else { format!(">={top}") }, json!(c));
    }
    Value::Object(m)
}

/// Python `round(x, n)`: the float nearest the correctly rounded decimal.
pub fn round(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap()
}

/// Stage 0 questions (docs/DATA.md): identities, tokens, granularity, supplier counts per
/// product and per supplier, repeated signatures, share of precise incidences.
pub fn audit(rows: &[Row], s: &Snapshot) -> Value {
    let M = matrix(s, "strict");
    let mut kinds = Map::new();
    for (_, k, _) in &s.issues {
        let n = kinds.get(*k).and_then(Value::as_u64).unwrap_or(0);
        kinds.insert(k.to_string(), json!(n + 1));
    }
    let count = |k: &str| kinds.get(k).and_then(Value::as_u64).unwrap_or(0) as usize;
    let mut by_class: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (c, E) in M.values() {
        by_class.entry(c).or_default().push(E.len());
    }
    let mut deg: HashMap<&str, usize> = HashMap::new();
    for (_, E) in M.values() {
        for v in E {
            *deg.entry(v).or_default() += 1;
        }
    }
    let mut sig: HashMap<Vec<&str>, usize> = HashMap::new();
    for v in s.S.keys() {
        let ps: Vec<&str> = M.iter().filter(|(_, (c, E))| E.contains(v) && c != "broad")
            .map(|(p, _)| p.as_str()).collect();
        if !ps.is_empty() {
            *sig.entry(ps).or_default() += 1;
        }
    }
    let mut cls = Map::new();
    for e in s.D.values() {
        let n = cls.get(&e.class).and_then(Value::as_u64).unwrap_or(0);
        cls.insert(e.class.clone(), json!(n + 1));
    }
    let mut inc: HashMap<&str, usize> = HashMap::new();
    for (_, t, _) in &s.Q {
        *inc.entry(s.D[t].class.as_str()).or_default() += 1;
    }
    let precise: usize = ["material", "naics", "program", "description"].iter()
        .map(|c| inc.get(c).copied().unwrap_or(0)).sum();
    let spp: Map<String, Value> = by_class.iter().map(|(c, v)| (c.to_string(), hist(v.iter().copied(), 3)))
        .collect();
    json!({
        "rows": rows.len(),
        "rows_usable": rows.len() - count("missing_name") - count("missing_product"),
        "issues": kinds,
        "suppliers": s.S.len(),
        "multiline_cells": rows.iter().filter(|r| qbl::strip(r.get("Product")).contains('\n')).count(),
        "tokens": s.D.len(),
        "tokens_by_class": cls,
        "multi_product_tokens": s.D.values().filter(|e| e.strict.len() > 1).count(),
        "mapped_by_judgement": s.D.values().filter(|e| e.justification.contains("judgement")).count(),
        "products": M.len(),
        "suppliers_per_product": spp,
        "products_per_supplier": hist(s.S.keys().map(|v| deg.get(v.as_str()).copied().unwrap_or(0)), 5),
        "signatures_nonbroad": sig.len(),
        "signatures_repeated": sig.values().filter(|&&n| n > 1).count(),
        "suppliers_in_repeated_signatures": sig.values().filter(|&&n| n > 1).sum::<usize>(),
        "incidences": s.Q.len(),
        "incidences_precise_share": round(precise as f64 / s.Q.len() as f64, 3),
    })
}
