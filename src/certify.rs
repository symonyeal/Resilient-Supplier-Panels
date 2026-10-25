//! Certificates read from the qualification matrix (E_p, k, F), never from H.
//!
//! Feasible:  |T ∩ E_p| ≥ k + 1 and T ∩ (E_p \ F) ≠ ∅ for every p and explicit F.
//! Minimal:   feasible, and every v ∈ T has a witness (p, F) with T ∩ (E_p \ F) = {v}.
//! Essential: v lies in every minimal panel  ⇔  V \ {v} is infeasible (monotonicity).
//! Complete:  no feasible T contains no member of the family (MODEL, Thm 3.3), decided by an
//!            exhaustive search written here, separately from solve.rs.
//!
//! T is a sorted index vector into V; a witness is (p, failed suppliers).

use std::collections::BTreeMap;

use crate::instance::Instance;

pub type Witness = (usize, Vec<usize>);

fn mask(I: &Instance, T: &[usize]) -> Vec<bool> {
    let mut m = vec![false; I.V.len()];
    for &v in T {
        m[v] = true;
    }
    m
}

fn meet(m: &[bool], S: &[usize]) -> usize {
    S.iter().filter(|&&v| m[v]).count()
}

/// T meets E_p minus F for every p and every F in the failure model (MODEL, Thm 2).
pub fn feasible(I: &Instance, T: &[usize]) -> bool {
    let m = mask(I, T);
    I.E.iter().all(|Ep| meet(&m, Ep) > I.k)
        && I.E.iter().all(|Ep| I.F.iter().all(|F| Ep.iter().any(|v| m[*v] && !F.contains(v))))
}

/// v -> (p, F) with T ∩ (E_p \ F) = {v}, or None when v is removable. The first product in
/// basket order supplies the witness; a tight product (|T ∩ E_p| = k + 1) fails the other k.
pub fn witnesses(I: &Instance, T: &[usize]) -> BTreeMap<usize, Option<Witness>> {
    let m = mask(I, T);
    let mut W = BTreeMap::new();
    for &v in T {
        let mut w = None;
        for (p, Ep) in I.E.iter().enumerate() {
            if !Ep.contains(&v) {
                continue;
            }
            let S: Vec<usize> = Ep.iter().copied().filter(|&u| m[u]).collect();
            if S.len() == I.k + 1 {
                w = Some((p, S.into_iter().filter(|&u| u != v).collect()));
                break;
            }
            let alone = |F: &Vec<usize>| {
                let R: Vec<usize> = Ep.iter().copied().filter(|u| m[*u] && !F.contains(u)).collect();
                R == [v]
            };
            if let Some(F) = I.F.iter().find(|F| alone(F)) {
                w = Some((p, F.clone()));
                break;
            }
        }
        W.insert(v, w);
    }
    W
}

/// T is feasible and every member has a witness (MODEL, Thm 3.1).
pub fn minimal(I: &Instance, T: &[usize]) -> bool {
    feasible(I, T) && witnesses(I, T).values().all(Option::is_some)
}

/// (p, F) with E_p \ F = ∅, F in the failure model; None if V is feasible.
pub fn infeasibility(I: &Instance) -> Option<Witness> {
    for (p, Ep) in I.E.iter().enumerate() {
        if Ep.len() <= I.k {
            return Some((p, Ep.clone()));
        }
        if let Some(F) = I.F.iter().find(|F| Ep.iter().all(|v| F.contains(v))) {
            return Some((p, F.clone()));
        }
    }
    None
}

/// v -> (p, F) with E_p \ F = {v}; None if the instance is infeasible. The first product
/// in basket order supplies each witness.
pub fn essential(I: &Instance) -> Option<BTreeMap<usize, Witness>> {
    if infeasibility(I).is_some() {
        return None;
    }
    let mut W = BTreeMap::new();
    for (p, Ep) in I.E.iter().enumerate() {
        if Ep.len() == I.k + 1 {
            for &v in Ep {
                W.entry(v).or_insert_with(|| (p, Ep.iter().copied().filter(|&u| u != v).collect()));
            }
        }
        for F in &I.F {
            let R: Vec<usize> = Ep.iter().copied().filter(|v| !F.contains(v)).collect();
            if let [v] = R[..] {
                W.entry(v).or_insert_with(|| (p, F.clone()));
            }
        }
    }
    Some(W)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Completeness {
    Complete,
    /// A minimal panel the family does not list.
    Missing(Vec<usize>),
    /// The family has a member that is not a minimal panel.
    Invalid,
}

/// Is every minimal panel in `family`? Searches for a feasible T containing no member.
///
/// Each node takes the violated row with fewest candidates and branches over them, a
/// candidate tried and abandoned being forbidden to its later siblings. If a minimal
/// feasible T′ avoids the family, the branch following T′ \ T reaches T′ or a feasible
/// subset of it, so exhaustion proves completeness. A T found is shrunk to a minimal panel.
pub fn complete(I: &Instance, family: &[Vec<usize>]) -> Completeness {
    if !family.iter().all(|S| minimal(I, S)) {
        return Completeness::Invalid;
    }
    // rows: (members, demand); demand k + 1 for E_p, 1 for each E_p \ F
    let mut rows: Vec<(Vec<usize>, usize)> = Vec::new();
    for Ep in &I.E {
        rows.push((Ep.clone(), I.k + 1));
        for F in &I.F {
            rows.push((Ep.iter().copied().filter(|v| !F.contains(v)).collect(), 1));
        }
    }
    let mut by_v: Vec<Vec<usize>> = vec![Vec::new(); I.V.len()];
    for (j, S) in family.iter().enumerate() {
        for &v in S {
            by_v[v].push(j);
        }
    }
    let mut s = Search { rows, family, by_v, held: vec![0; family.len()], T: vec![false; I.V.len()],
                         barred: vec![false; I.V.len()] };
    if family.iter().any(|S| S.is_empty()) {
        return Completeness::Complete;
    }
    match s.go() {
        None => Completeness::Complete,
        Some(mut T) => {
            for v in T.clone() {                         // shrink to a minimal panel
                let U: Vec<usize> = T.iter().copied().filter(|&u| u != v).collect();
                if feasible(I, &U) {
                    T = U;
                }
            }
            Completeness::Missing(T)
        }
    }
}

struct Search<'a> {
    rows: Vec<(Vec<usize>, usize)>,
    family: &'a [Vec<usize>],
    by_v: Vec<Vec<usize>>,
    /// held[j] = |S_j ∩ T|; S_j ⊆ T iff held[j] = |S_j|.
    held: Vec<usize>,
    T: Vec<bool>,
    barred: Vec<bool>,
}

impl Search<'_> {
    /// Adds v; false when T now contains a member of the family.
    fn add(&mut self, v: usize) -> bool {
        self.T[v] = true;
        let mut ok = true;
        for &j in &self.by_v[v] {
            self.held[j] += 1;
            ok &= self.held[j] < self.family[j].len();
        }
        ok
    }

    fn drop(&mut self, v: usize) {
        self.T[v] = false;
        for &j in &self.by_v[v] {
            self.held[j] -= 1;
        }
    }

    fn go(&mut self) -> Option<Vec<usize>> {
        let mut pick: Option<Vec<usize>> = None;
        for (R, d) in &self.rows {
            let have = R.iter().filter(|&&v| self.T[v]).count();
            if have >= *d {
                continue;
            }
            let cand: Vec<usize> = R.iter().copied().filter(|&v| !self.T[v] && !self.barred[v]).collect();
            if cand.len() < d - have {
                return None;
            }
            if pick.as_ref().is_none_or(|p| cand.len() < p.len()) {
                pick = Some(cand);
            }
        }
        let Some(cand) = pick else {
            return Some((0..self.T.len()).filter(|&v| self.T[v]).collect());
        };
        let mut barred = Vec::new();
        let mut found = None;
        for v in cand {
            if self.add(v) {
                found = self.go();
            }
            self.drop(v);
            if found.is_some() {
                break;
            }
            self.barred[v] = true;
            barred.push(v);
        }
        for v in barred {
            self.barred[v] = false;
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::tests::inst;

    #[test]
    fn hand_example_witnesses() {
        // E = ab, bc, cd; T = bd is minimal; a is redundant in abd
        let I = inst("abcd", &["ab", "bc", "cd"], 0, &[]);
        assert!(minimal(&I, &[1, 3]));
        let W = witnesses(&I, &[0, 1, 3]);
        assert_eq!(W[&0], None);
        assert_eq!(essential(&I).unwrap().len(), 0);
        assert_eq!(essential(&I.with_k(1)).unwrap().len(), 4);
    }

    #[test]
    fn empty_edge_is_infeasible() {
        let I = inst("ab", &["a", ""], 0, &[]);
        assert_eq!(infeasibility(&I), Some((1, vec![])));
        assert_eq!(essential(&I), None);
    }

    #[test]
    fn completeness_detects_missing_and_invalid() {
        let I = inst("ab", &["ab"], 0, &[]);
        assert_eq!(complete(&I, &[vec![0], vec![1]]), Completeness::Complete);
        assert_eq!(complete(&I, &[vec![0]]), Completeness::Missing(vec![1]));
        assert_eq!(complete(&I, &[vec![0, 1]]), Completeness::Invalid);
        let J = inst("", &[], 0, &[]);
        assert_eq!(complete(&J, &[]), Completeness::Missing(vec![]));
        assert_eq!(complete(&J, &[vec![]]), Completeness::Complete);
    }

    #[test]
    fn unknown_supplier_is_never_feasible() {
        let I = inst("abc", &["abc"], 0, &[]);
        assert!(I.index(&["a".into(), "unknown".into()]).is_none());
    }
}
