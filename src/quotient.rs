//! Exact |Tr(H)| through supplier signature classes (docs/MODEL.md, Theorem 4).
//!
//! A class is a maximal set of suppliers with equal membership in every E_p and explicit F.
//! Permuting a class fixes the matrix, so feasibility and minimality of T depend only on
//! m_c = |T ∩ c|, and a minimal vector has m_c ≤ r_c = min(|c|, k + 1). The reduced instance
//! keeps r_c copies of each class; its minimal panels give exactly the minimal vectors M, and
//!     |Tr(H)| = Σ_{m ∈ M} Π_c C(|c|, m_c).
//! |M| is the number of panels up to signature symmetry (bench `orbits`).

use std::collections::{BTreeMap, HashSet};

use crate::bits::Bits;
use crate::instance::{hypergraph, Instance};
use crate::mmcs;
use crate::run::{Clock, Stop};

/// Signature classes of V: the members of each, in V order, classes in order of first member.
pub fn classes(I: &Instance) -> Vec<Vec<usize>> {
    let mut by: BTreeMap<Vec<bool>, usize> = BTreeMap::new();
    let mut out: Vec<Vec<usize>> = Vec::new();
    for v in 0..I.V.len() {
        let sig: Vec<bool> = I.E.iter().chain(&I.F).map(|S| S.contains(&v)).collect();
        let j = *by.entry(sig).or_insert_with(|| {
            out.push(Vec::new());
            out.len() - 1
        });
        out[j].push(v);
    }
    out
}

pub struct Count {
    /// |Tr(H)|.
    pub panels: u128,
    /// |M|, the panels up to signature symmetry.
    pub orbits: usize,
    /// Minimal panels of the reduced instance enumerated.
    pub reduced: usize,
}

/// C(n, r), exact: a product of r consecutive integers is divisible by r!; 0 when r > n.
pub fn binom(n: usize, r: usize) -> u128 {
    if r > n { 0 } else { (0..r).fold(1u128, |a, i| a * (n - i) as u128 / (i + 1) as u128) }
}

/// The count, or the stop reason (clock, or a product above u128).
pub fn count(I: &Instance, clock: &Clock) -> Result<Count, Stop> {
    let cls: Vec<Vec<usize>> = classes(I).into_iter()
        .filter(|c| I.E.iter().any(|Ep| Ep.contains(&c[0])))
        .collect();
    let r: Vec<usize> = cls.iter().map(|c| c.len().min(I.k + 1)).collect();
    // reduced instance: the first r_c members of each class stand for it
    let keep: Vec<usize> = cls.iter().zip(&r).flat_map(|(c, &r)| c[..r].iter().copied()).collect();
    let mut of = vec![usize::MAX; I.V.len()];
    for (j, c) in cls.iter().enumerate() {
        for &v in c {
            of[v] = j;
        }
    }
    let sub = |S: &Vec<usize>| S.iter().copied().filter(|v| keep.contains(v)).collect();
    let J = Instance { V: I.V.clone(), P: I.P.clone(), E: I.E.iter().map(sub).collect(), k: I.k,
                       F: I.F.iter().map(sub).collect() };
    crate::with_width!(J.support().len(), W => {
        let S = hypergraph::<W>(&J);
        let mut M: HashSet<Vec<u8>> = HashSet::new();
        let mut panels = 0u128;
        let mut reduced = 0usize;
        let mut overflow = false;
        mmcs::transversals(&S.H, clock, &mut |t: Bits<W>| {
            reduced += 1;
            let mut m = vec![0u8; cls.len()];
            for i in t.iter() {
                m[of[S.labels[i]]] += 1;
            }
            let w = m.iter().enumerate()
                .try_fold(1u128, |a, (c, &mc)| a.checked_mul(binom(cls[c].len(), mc as usize)));
            if M.insert(m) {
                match w.and_then(|w| panels.checked_add(w)) {
                    Some(x) => panels = x,
                    None => overflow = true,
                }
            }
            !overflow
        }).map_err(|s| if overflow { Stop::Cap } else { s })?;
        Ok(Count { panels, orbits: M.len(), reduced })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::tests::inst;
    use crate::solve::exhaustive;

    #[test]
    fn count_matches_exhaustive_with_scenarios() {
        // all pairs except ab are feasible; two orbits (cross-class pairs, cd)
        let I = inst("abcd", &["abcd"], 1, &["ab"]);
        let c = count(&I, &Clock::unlimited()).unwrap();
        assert_eq!((c.panels, c.orbits), (exhaustive(&I).len() as u128, 2));
        let J = inst("abcdef", &["abc", "cdef", "ef"], 1, &[]);
        assert_eq!(count(&J, &Clock::unlimited()).unwrap().panels, exhaustive(&J).len() as u128);
    }

    #[test]
    fn binomials() {
        assert_eq!(binom(50, 0), 1);
        assert_eq!(binom(50, 2), 1225);
    }
}
