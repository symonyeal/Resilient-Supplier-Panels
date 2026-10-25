//! Panels: minimum cardinality, and minimal-panel enumeration by ordered cuts and by
//! exhaustive search.
//!
//! min Σ_v x_v  s.t.  Σ_{v∈E_p} x_v ≥ k + 1,  Σ_{v∈E_p\F} x_v ≥ 1 (explicit F),
//!                    Σ_{v∈S} x_v ≤ |S| − 1 (S ∈ cuts),  x ∈ {0,1}^U,  U the support.
//! Unit weights count suppliers; they are not costs. Solved exactly by branch and bound.

use crate::bits::Bits;
use crate::certify;
use crate::instance::{combinations, decode, encode, Instance};
use crate::run::{Clock, Stop};

/// Rows R x ≥ d over support positions.
fn rows<const W: usize>(I: &Instance, labels: &[usize]) -> Vec<(Bits<W>, usize)> {
    let mut A = Vec::new();
    for Ep in &I.E {
        A.push((encode(labels, Ep), I.k + 1));
        for F in &I.F {
            let R: Vec<usize> = Ep.iter().copied().filter(|v| !F.contains(v)).collect();
            A.push((encode(labels, &R), 1));
        }
    }
    A
}

struct Bnb<'a, const W: usize> {
    A: &'a [(Bits<W>, usize)],
    cuts: &'a [Bits<W>],
    best: Option<Bits<W>>,
    clock: &'a Clock,
    nodes: usize,
    stopped: bool,
}

impl<const W: usize> Bnb<'_, W> {
    /// Depth first from (inn, out): forced moves, a disjoint-row packing bound, then a branch
    /// on the free member of the tightest deficient row that meets most deficient rows.
    fn go(&mut self, mut inn: Bits<W>, mut out: Bits<W>) {
        self.nodes += 1;
        if self.nodes % 64 == 0 && self.clock.expired() {
            self.stopped = true;
        }
        if self.stopped {
            return;
        }
        loop {
            let mut changed = false;
            for (R, d) in self.A {
                let avail = R.minus(&out);
                if avail.len() < *d {
                    return;
                }
                if R.meet_len(&inn) < *d && avail.len() == *d {
                    inn = inn.or(&avail);
                    changed = true;
                }
            }
            for S in self.cuts {
                if S.sub(&inn) {
                    return;
                }
                let free = S.minus(&inn);
                if !free.meets(&out) && free.len() == 1 {
                    out = out.or(&free);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut short: Vec<(Bits<W>, usize)> = self.A.iter()
            .filter_map(|(R, d)| {
                let have = R.meet_len(&inn);
                (have < *d).then(|| (R.minus(&inn).minus(&out), d - have))
            })
            .collect();
        let Some(tight) = short.iter().min_by_key(|(f, d)| f.len() - d).map(|x| x.0) else {
            if self.best.is_none_or(|b| inn.len() < b.len()) {
                self.best = Some(inn);
            }
            return;
        };
        short.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        let mut used = Bits::EMPTY;
        let mut bound = inn.len();
        for (f, d) in &short {
            if !f.meets(&used) {
                bound += d;
                used = used.or(f);
            }
        }
        if self.best.is_some_and(|b| bound >= b.len()) {
            return;
        }
        let v = tight.iter().max_by_key(|&v| {
            (short.iter().filter(|(f, _)| f.has(v)).count(), std::cmp::Reverse(v))
        }).unwrap();
        self.go(inn.with(v), out);
        self.go(inn, out.with(v));
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Minimum {
    Optimal(Vec<usize>),
    Infeasible,
    Stopped(Stop),
}

/// Minimum-cardinality feasible panel containing no S ∈ cuts (supplier indices).
pub fn minimum(I: &Instance, cuts: &[Vec<usize>], clock: &Clock) -> Minimum {
    let labels = I.support();
    crate::with_width!(labels.len(), W => {
        let A = rows::<W>(I, &labels);
        let C: Vec<Bits<W>> = cuts.iter().map(|S| encode(&labels, S)).collect();
        min_bits(&A, &C, clock).map_or(Minimum::Stopped(Stop::Timeout),
                                        |b| b.map_or(Minimum::Infeasible,
                                                     |T| Minimum::Optimal(decode(&labels, &T))))
    })
}

/// None when stopped by the clock; Some(None) when infeasible.
fn min_bits<const W: usize>(A: &[(Bits<W>, usize)], cuts: &[Bits<W>], clock: &Clock)
                            -> Option<Option<Bits<W>>> {
    let mut b = Bnb { A, cuts, best: None, clock, nodes: 0, stopped: false };
    b.go(Bits::EMPTY, Bits::EMPTY);
    (!b.stopped).then_some(b.best)
}

/// All minimal panels in nondecreasing size: an optimum avoiding supersets of the panels
/// found is minimal (a proper feasible subset would be cheaper and still avoid them);
/// infeasibility of the cut problem proves completeness. `emit` returns false to stop.
pub fn cut_stream(I: &Instance, clock: &Clock, emit: &mut dyn FnMut(Vec<usize>) -> bool)
                  -> Result<(), Stop> {
    let labels = I.support();
    crate::with_width!(labels.len(), W => {
        let A = rows::<W>(I, &labels);
        let mut found: Vec<Bits<W>> = Vec::new();
        loop {
            if clock.expired() {
                return Err(Stop::Timeout);
            }
            match min_bits(&A, &found, clock) {
                None => return Err(Stop::Timeout),
                Some(None) => return Ok(()),
                Some(Some(T)) => {
                    found.push(T);
                    if !emit(decode(&labels, &T)) {
                        return Err(Stop::Cap);
                    }
                }
            }
        }
    })
}

/// Tr by checking every subset of the support against the matrix, in size order.
/// O(2^n |P| |F|); for supports of at most 22.
pub fn exhaustive(I: &Instance) -> Vec<Vec<usize>> {
    let U = I.support();
    assert!(U.len() <= 22, "exhaustive search is for small supports");
    let mut found: Vec<Vec<usize>> = Vec::new();
    for r in 0..=U.len() {
        for T in combinations(&U, r) {
            if !found.iter().any(|S| S.iter().all(|v| T.contains(v))) && certify::feasible(I, &T) {
                found.push(T);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::tests::inst;

    #[test]
    fn hand_example_minimum_and_resilience() {
        let I = inst("abcd", &["ab", "bc", "cd"], 0, &[]);
        let c = Clock::unlimited();
        let Minimum::Optimal(T) = minimum(&I, &[], &c) else { panic!() };
        assert_eq!(T.len(), 2);
        assert_eq!(minimum(&I.with_k(1), &[], &c), Minimum::Optimal(vec![0, 1, 2, 3]));
        let mut all = Vec::new();
        cut_stream(&I, &c, &mut |T| { all.push(T); true }).unwrap();
        all.sort();
        assert_eq!(all, vec![vec![0, 2], vec![1, 2], vec![1, 3]]);
        assert_eq!(exhaustive(&I), vec![vec![0, 2], vec![1, 2], vec![1, 3]]);
    }

    #[test]
    fn empty_cases() {
        let c = Clock::unlimited();
        assert_eq!(minimum(&inst("ab", &[], 0, &[]), &[], &c), Minimum::Optimal(vec![]));
        assert_eq!(minimum(&inst("ab", &["a", ""], 0, &[]), &[], &c), Minimum::Infeasible);
    }
}
