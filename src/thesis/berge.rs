//! Tr by Berge's method (fka `transversal`, berge.m): fold in one term at a time, shortest
//! first, from Tr(∅) = {∅}; Tr({∅}) = ∅ falls out of the fold.
//!
//! Step on term t, from the antichain Tr_i:
//!   K = {s ∈ Tr_i : s ∩ t ≠ ∅}        kept unchanged (each is still minimal);
//!   s ∪ {v}, s ∈ Tr_i \ K, v ∈ t     minimal iff it contains no k ∈ K with v ∈ k.
//! No other set of Tr_{i+1} can lie inside s ∪ {v}, and no two candidates coincide, so this is
//! the thesis' `minimal_elements(nxt)` without the quadratic pass.

use super::Mbf;
use crate::bits::Bits;
use crate::run::{Clock, Stop};

/// Tr(H) in canonical order, or the reason the fold stopped: the clock, or a working set
/// above `cap` sets.
pub fn transversal<const W: usize>(H: &Mbf<W>, clock: &Clock, cap: usize)
                                   -> Result<Vec<Bits<W>>, Stop> {
    let mut order = H.edges.clone();
    order.sort_by_key(|t| t.len());                      // stable: canonical within a size
    let mut Tr: Vec<Bits<W>> = vec![Bits::EMPTY];
    for t in &order {
        if t.is_empty() {
            return Ok(Vec::new());
        }
        let (K, rest): (Vec<_>, Vec<_>) = Tr.into_iter().partition(|s| s.meets(t));
        let mut by_v: Vec<Vec<usize>> = vec![Vec::new(); H.n];   // v -> kept sets holding v
        for (i, k) in K.iter().enumerate() {
            for v in k.and(t).iter() {
                by_v[v].push(i);
            }
        }
        let mut next = K.clone();
        for (j, s) in rest.iter().enumerate() {
            if j % 4096 == 0 && clock.expired() {
                return Err(Stop::Timeout);
            }
            for v in t.iter() {
                let c = s.with(v);
                if !by_v[v].iter().any(|&i| K[i].sub(&c)) {
                    next.push(c);
                }
            }
            if next.len() > cap {
                return Err(Stop::Memory);
            }
        }
        Tr = next;
    }
    Tr.sort_by(|a, b| a.lex(b));
    Ok(Tr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: usize, e: &[&[usize]]) -> Mbf<1> {
        Mbf::new(n, e.iter().map(|t| Bits::of(t.iter().copied())).collect())
    }

    #[test]
    fn degenerate_conventions() {
        let c = Clock::unlimited();
        assert_eq!(transversal(&h(2, &[]), &c, usize::MAX).unwrap(), vec![Bits::EMPTY]);
        assert!(transversal(&h(2, &[&[]]), &c, usize::MAX).unwrap().is_empty());
    }

    #[test]
    fn path() {
        // ab, bc, cd -> ac, bc, bd
        let T = transversal(&h(4, &[&[0, 1], &[1, 2], &[2, 3]]), &Clock::unlimited(), usize::MAX);
        let T: Vec<Vec<usize>> = T.unwrap().iter().map(|t| t.to_vec()).collect();
        assert_eq!(T, vec![vec![0, 2], vec![1, 2], vec![1, 3]]);
    }
}
