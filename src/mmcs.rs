//! Tr(H) by MMCS (Murakami and Uno 2014, "Efficient algorithms for dualizing large-scale
//! hypergraphs", Discrete Applied Mathematics 170), streaming each minimal transversal.
//!
//! State: S the current set, CAND the vertices that may still be added,
//! crit(u) = #{e ∈ H : e ∩ S = {u}}, uncov = {e : e ∩ S = ∅}.
//! Step: if uncov = ∅, output S. Otherwise take e ∈ uncov with |e ∩ CAND| least,
//! C = e ∩ CAND, CAND ← CAND \ C; for each v ∈ C in turn, recurse on S + v when every
//! u ∈ S + v keeps crit(u) > 0, then return v to CAND. A set is reached only from its
//! last-added branch, so each member of Tr(H) is output once; every S visited extends to
//! one, so the delay is polynomial.

use crate::bits::Bits;
use crate::run::{Clock, Stop};
use crate::thesis::Mbf;

struct Mmcs<'a, const W: usize> {
    H: &'a [Bits<W>],
    by_v: Vec<Vec<usize>>,
    hit: Vec<usize>,
    crit: Vec<usize>,
    S: Bits<W>,
    clock: &'a Clock,
    nodes: usize,
}

impl<const W: usize> Mmcs<'_, W> {
    fn add(&mut self, v: usize) {
        for &e in &self.by_v[v] {
            self.hit[e] += 1;
            match self.hit[e] {
                1 => self.crit[v] += 1,
                2 => {
                    let u = self.H[e].and(&self.S).lowest().unwrap();
                    self.crit[u] -= 1;
                }
                _ => {}
            }
        }
        self.S.insert(v);
    }

    fn remove(&mut self, v: usize) {
        self.S.remove(v);
        for &e in &self.by_v[v] {
            match self.hit[e] {
                1 => self.crit[v] -= 1,
                2 => {
                    let u = self.H[e].and(&self.S).lowest().unwrap();
                    self.crit[u] += 1;
                }
                _ => {}
            }
            self.hit[e] -= 1;
        }
    }

    fn go(&mut self, cand: &mut Bits<W>, emit: &mut dyn FnMut(Bits<W>) -> bool) -> Result<(), Stop> {
        self.nodes += 1;
        if self.nodes % 4096 == 0 && self.clock.expired() {
            return Err(Stop::Timeout);
        }
        let e = (0..self.H.len()).filter(|&e| self.hit[e] == 0)
            .min_by_key(|&e| self.H[e].meet_len(cand));
        let Some(e) = e else {
            return if emit(self.S) { Ok(()) } else { Err(Stop::Cap) };
        };
        let C = self.H[e].and(cand);
        *cand = cand.minus(&C);
        let mut out = Ok(());
        for v in C.iter() {
            self.add(v);
            if self.S.iter().all(|u| self.crit[u] > 0) {
                out = self.go(cand, emit);
            }
            self.remove(v);
            cand.insert(v);
            if out.is_err() {
                break;
            }
        }
        *cand = cand.or(&C);
        out
    }
}

/// Streams Tr(H); `emit` returns false to stop. Tr(∅) = {∅}; an empty edge gives Tr = ∅.
pub fn transversals<const W: usize>(H: &Mbf<W>, clock: &Clock,
                                    emit: &mut dyn FnMut(Bits<W>) -> bool) -> Result<(), Stop> {
    if H.edges.iter().any(|e| e.is_empty()) {
        return Ok(());
    }
    let mut by_v = vec![Vec::new(); H.n];
    for (j, e) in H.edges.iter().enumerate() {
        for v in e.iter() {
            by_v[v].push(j);
        }
    }
    let mut m = Mmcs { H: &H.edges, by_v, hit: vec![0; H.len()], crit: vec![0; H.n],
                       S: Bits::EMPTY, clock, nodes: 0 };
    let mut cand = H.support();
    m.go(&mut cand, emit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thesis::berge;

    #[test]
    fn agrees_with_berge_on_small_hypergraphs() {
        let mut seed = 7u64;
        let mut rnd = |m: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        for _ in 0..300 {
            let n = 1 + rnd(12) as usize;
            let edges = (0..rnd(8)).map(|_| Bits::<1>::of((0..n).filter(|_| rnd(3) == 0)))
                .collect();
            let H = Mbf::new(n, crate::thesis::minimal(edges));
            let c = Clock::unlimited();
            let mut got = Vec::new();
            transversals(&H, &c, &mut |t| { got.push(t); true }).unwrap();
            got.sort_by(|a, b| a.lex(b));
            assert_eq!(got, berge::transversal(&H, &c, usize::MAX).unwrap(), "{H:?}");
        }
    }
}
