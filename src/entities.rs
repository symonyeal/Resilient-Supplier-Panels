//! Resolved identities and owner groups (docs/DATA.md, "Identity and ownership").
//!
//! data/catalog/identities.csv gives a verdict for every identity candidate of the frozen
//! snapshot: `same` (one entity at one site, written two ways: merged), `site` (another
//! plant of the same contracting entity: kept apart, and a member of an owner group) or
//! `unresolved`. data/catalog/owners.csv lists owner groups, one sourced row per member.
//! An owner scenario fails every member of a group together (MODEL, explicit F); with
//! scenarios only, product p fails iff E_p lies inside one failed group, so exposure is
//! read product by product and certified by `certify::infeasibility`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::audit::{self, Matrix, Snapshot};
use crate::bench::{self, CLASSES};
use crate::catalog::csv;
use crate::certify;
use crate::instance::Instance;
use crate::json::{json, Value};

pub const VERDICTS: [&str; 3] = ["same", "site", "unresolved"];

/// Reject incomplete recipient evidence instead of silently dropping dollars or links.
pub fn awards(A: &Value, s: &Snapshot) -> Result<(), String> {
    let links = A["links"].as_object().ok_or("award links must be an object")?;
    let recipients = A["recipients"].as_object().ok_or("award recipients must be an object")?;
    for (v, U) in links {
        if !s.S.contains_key(v) {
            return Err(format!("award link: unknown supplier {v}"));
        }
        for u in U.as_array().ok_or("award links must be lists")? {
            let u = u.as_str().ok_or("award UEI must be a string")?;
            if recipients.get(u).and_then(|r| r["amount"].as_f64()).is_none_or(|x| !x.is_finite()) {
                return Err(format!("award recipient {u}: missing finite amount"));
            }
        }
    }
    Ok(())
}

/// (first, candidate, verdict, evidence) of identities.csv.
pub type Pair = (String, String, String, String);

fn table(text: &str, need: &[&str]) -> Result<Vec<HashMap<String, String>>, String> {
    let rows = csv(text);
    let head = rows.first().ok_or("empty table")?;
    if let Some(c) = need.iter().find(|c| !head.iter().any(|h| h == *c)) {
        return Err(format!("missing column {c}"));
    }
    if rows[1..].iter().any(|r| r.len() != head.len()) {
        return Err("catalog row has the wrong number of columns".into());
    }
    Ok(rows[1..].iter().map(|r| head.iter().cloned().zip(r.iter().cloned()).collect()).collect())
}

/// The verdicts, checked against the snapshot: exactly one row per identity candidate.
pub fn identities(text: &str, s: &Snapshot) -> Result<Vec<Pair>, String> {
    let first: HashMap<usize, &String> = s.S.iter().map(|(sid, x)| (x.rows[0], sid)).collect();
    let want: BTreeSet<(String, String)> = s.issues.iter().filter(|(_, k, _)| *k == "identity_candidate")
        .map(|(r, _, d)| (d.clone(), first[r].clone())).collect();
    let P: Vec<Pair> = table(text, &["first", "candidate", "verdict", "evidence"])?.into_iter()
        .map(|r| (r["first"].clone(), r["candidate"].clone(), r["verdict"].clone(), r["evidence"].clone())).collect();
    if let Some(p) = P.iter().find(|p| !VERDICTS.contains(&p.2.as_str())) {
        return Err(format!("identities.csv: verdict {:?} for {}/{}", p.2, p.0, p.1));
    }
    if P.iter().any(|p| p.3.trim().is_empty()) {
        return Err("identities.csv: missing evidence".into());
    }
    let got: BTreeSet<(String, String)> = P.iter().map(|p| (p.0.clone(), p.1.clone())).collect();
    if got != want || got.len() != P.len() {
        return Err(format!("identities.csv: {} rows for {} identity candidates, or pairs differ", P.len(), want.len()));
    }
    Ok(P)
}

/// Owner group -> member supplier ids, checked: each id exists and its snapshot name is the
/// row's `name`, each supplier is in at most one group, each group has two members.
pub fn owners(text: &str, s: &Snapshot) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut G: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for r in table(text, &["group", "supplier", "name", "relation", "source"])? {
        let v = &r["supplier"];
        match s.S.get(v) {
            Some(x) if x.name == r["name"] => {}
            _ => return Err(format!("owners.csv: {v} is not {:?} in this snapshot", r["name"])),
        }
        if r["source"].is_empty() || !seen.insert(v.clone()) {
            return Err(format!("owners.csv: {v} without a source or in two groups"));
        }
        G.entry(r["group"].clone()).or_default().push(v.clone());
    }
    if let Some((g, _)) = G.iter().find(|(_, m)| m.len() < 2) {
        return Err(format!("owners.csv: group {g} has one member"));
    }
    Ok(G)
}

/// The instance with each supplier replaced by its representative (identity merge).
pub fn merge(I: &Instance, rep: &HashMap<&String, &String>) -> Instance {
    let E: Vec<Vec<String>> = I.E.iter().map(|E| {
        let s: BTreeSet<String> = E.iter().map(|&v| {
            let id = &I.V[v];
            rep.get(id).map_or(id.clone(), |d| (*d).clone())
        }).collect();
        s.into_iter().collect()
    }).collect();
    let V: BTreeSet<String> = E.iter().flatten().cloned().collect();
    let F: Vec<Vec<String>> = I.F.iter().map(|F| {
        F.iter().map(|&v| rep.get(&I.V[v]).map_or(I.V[v].clone(), |d| (*d).clone()))
            .filter(|v| V.contains(v)).collect::<BTreeSet<_>>().into_iter().collect()
    }).collect();
    Instance::new(V.into_iter().collect(), I.P.clone(), E, I.k as i64, F).expect("merged instance")
}

/// The basket at level k with every owner group meeting its support as a scenario.
pub fn with_owners(M: &Matrix, P: &[String], k: usize, G: &BTreeMap<String, Vec<String>>) -> Instance {
    let I = audit::basket(M, P, k);
    let F: Vec<Vec<String>> = G.values()
        .map(|m| m.iter().filter(|v| I.V.contains(v)).cloned().collect::<Vec<_>>())
        .filter(|f| !f.is_empty()).collect();
    Instance::new(I.V.clone(), I.P.clone(), P.iter().map(|p| M[p].1.iter().cloned().collect()).collect(), k as i64, F)
        .expect("owner scenarios lie in V")
}

/// Entity groups: the components of the owner groups joined with the `same` and `site`
/// pairs (each pair is one contracting entity). A component is named by its first owner
/// group, else by its first member.
pub fn groups(pairs: &[Pair], G: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
    // union-find; a root is the smallest id of its component
    fn root(up: &mut BTreeMap<String, String>, v: &str) -> String {
        let mut r = v.to_string();
        loop {
            let p = up.entry(r.clone()).or_insert_with(|| r.clone()).clone();
            if p == r {
                return r;
            }
            r = p;
        }
    }
    fn join(up: &mut BTreeMap<String, String>, a: &str, b: &str) {
        let (x, y) = (root(up, a), root(up, b));
        if x != y {
            let (lo, hi) = if x < y { (x, y) } else { (y, x) };
            up.insert(hi, lo);
        }
    }
    let mut up: BTreeMap<String, String> = BTreeMap::new();
    for m in G.values() {
        for v in &m[1..] {
            join(&mut up, &m[0], v);
        }
    }
    for p in pairs.iter().filter(|p| p.2 != "unresolved") {
        join(&mut up, &p.0, &p.1);
    }
    let mut C: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for v in up.keys().cloned().collect::<Vec<_>>() {
        let r = root(&mut up, &v);
        C.entry(r).or_default().push(v);
    }
    C.into_values().filter(|m| m.len() >= 2).map(|m| {
        let name = G.iter().find(|(_, g)| g.iter().any(|v| m.contains(v))).map_or_else(|| format!("entity {}", m[0]), |(g, _)| g.clone());
        (name, m)
    }).collect()
}

/// Identity verdicts against the K7 sensitivity, owner exposure, and award dollars.
///
/// INPUT   A ............ data/catalog/usaspending.json: links (supplier -> UEIs) and recipients
/// OUTPUT  identities ... verdict counts; over the primary baskets of seed 1 at k ∈ {0, 1},
///                        the feasibility flips with every candidate merged (K7) and with
///                        the `same` pairs only
///         owners ....... per entity group (`groups`): members, precise products whose every
///                        supplier is in the group (a backup inside one owner), AMS award
///                        dollars of its linked UEIs; over seeds 1..=n, primary baskets
///                        feasible at k = 1, those still feasible when each group may fail,
///                        and the groups that break them
///         awards ....... suppliers linked to a UEI, their distinct UEIs and award dollars
pub fn report(s: &Snapshot, pairs: &[Pair], G: &BTreeMap<String, Vec<String>>, A: &Value, n: u64) -> Value {
    let M = audit::matrix(s, "strict");
    let all: HashMap<&String, &String> = pairs.iter().map(|p| (&p.1, &p.0)).collect();
    let same: HashMap<&String, &String> = pairs.iter().filter(|p| p.2 == "same").map(|p| (&p.1, &p.0)).collect();
    let feasible = |I: &Instance| certify::infeasibility(I).is_none();
    let primary = |seed| bench::baskets(&M, seed).into_iter().filter(|b| b.1 != "scarce").map(|b| b.3);
    let mut flips = json!({"all_candidates": 0, "same_only": 0});
    let mut flipped = Vec::new();
    for P in primary(1) {
        for k in [0, 1] {
            let I = audit::basket(&M, &P, k);
            let (a, b) = (feasible(&merge(&I, &all)), feasible(&merge(&I, &same)));
            for (key, f) in [("all_candidates", a), ("same_only", b)] {
                if f != feasible(&I) {
                    flips[key] = json!(flips[key].as_u64().unwrap() + 1);
                }
            }
            if a != feasible(&I) || b != feasible(&I) {
                flipped.push(json!({"P": P, "k": k, "strict": feasible(&I), "all_candidates": a, "same_only": b}));
            }
        }
    }
    let verdicts: BTreeMap<&str, usize> = VERDICTS.iter().map(|v| (*v, pairs.iter().filter(|p| p.2 == *v).count())).collect();

    let links = |v: &String| -> Vec<String> {
        A["links"][v.as_str()].as_array().map_or(Vec::new(), |u| u.iter().filter_map(|x| x.as_str().map(String::from)).collect())
    };
    let dollars = |U: &BTreeSet<String>| -> f64 { U.iter().filter_map(|u| A["recipients"][u.as_str()]["amount"].as_f64()).sum() };
    let E = groups(pairs, G);
    let inside = |P: &BTreeSet<String>| E.iter().find(|(_, m)| P.len() >= 2 && P.iter().all(|v| m.contains(v))).map(|(g, _)| g);
    let owners: Vec<Value> = E.iter().map(|(g, m)| {
        let held: Vec<&String> = M.iter().filter(|(_, (c, P))| CLASSES.contains(&c.as_str()) && inside(P) == Some(g))
            .map(|(p, _)| p).collect();
        let U: BTreeSet<String> = m.iter().flat_map(links).collect();
        json!({"group": g, "members": m, "backup_inside": held, "ueis": U, "award_dollars": audit::round(dollars(&U), 2)})
    }).collect();
    let (mut base, mut kept, mut breaks) = (0usize, 0usize, BTreeMap::<&String, usize>::new());
    for seed in 1..=n {
        for P in primary(seed) {
            if !feasible(&audit::basket(&M, &P, 1)) {
                continue;
            }
            base += 1;
            if feasible(&with_owners(&M, &P, 1, &E)) {
                kept += 1;
            } else {
                for g in P.iter().filter_map(|p| inside(&M[p].1)).collect::<BTreeSet<_>>() {
                    *breaks.entry(g).or_default() += 1;
                }
            }
        }
    }
    let linked: Vec<&String> = s.S.keys().filter(|v| !links(v).is_empty()).collect();
    let U: BTreeSet<String> = linked.iter().flat_map(|v| links(v)).collect();
    json!({"identities": {"verdicts": verdicts, "flips": flips, "flipped": flipped},
           "owners": {"groups": owners, "seeds": n,
                      "baskets": {"feasible_k1": base, "feasible_with_owner_scenarios": kept},
                      "breaks": breaks},
           "awards": {"source": A["source"], "retrieved_utc": A["retrieved_utc"], "awards": A["awards"],
                      "link_rule": A["link_rule"],
                      "amount_scope": "Full Award Amount per distinct candidate recipient UEI; no allocation to QBL suppliers or plants, no unit prices or capacity",
                      "suppliers": s.S.len(), "linked": linked.len(), "ueis": U.len(),
                      "dollars": audit::round(dollars(&U), 2)}})
}
