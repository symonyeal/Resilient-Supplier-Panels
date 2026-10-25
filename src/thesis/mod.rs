//! Ports of the thesis package (`fka`, `fkb`, Hypergraph-Dualization at 1bb9de4), restricted to
//! what this project calls: Berge's Tr, FK-A with its memoised DAG, FK-B and the dualisation
//! loop. Term order, tie-breaks and conditions follow the Python line by line, so recursion
//! trees and output order agree with the thesis code (docs/VALIDATION.md).
//!
//! A hypergraph is read as a monotone Boolean function: a CNF's terms are clauses, a DNF's are
//! monomials; `cnf = Tr(dnf)` is the duality question both algorithms decide.

pub mod berge;
pub mod fka;
pub mod fkb;

use crate::bits::Bits;

/// A term set on variables 0..n−1, deduplicated and in canonical (lex) order
/// (fka `Hypergraph`). Equal term sets compare equal, which memoisation relies on.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Mbf<const W: usize> {
    pub n: usize,
    pub edges: Vec<Bits<W>>,
}

impl<const W: usize> Mbf<W> {
    pub fn new(n: usize, mut edges: Vec<Bits<W>>) -> Self {
        edges.sort_by(|a, b| a.lex(b));
        edges.dedup();
        Mbf { n, edges }
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn support(&self) -> Bits<W> {
        self.edges.iter().fold(Bits::EMPTY, |u, t| u.or(t))
    }

    pub fn degree(&self, x: usize) -> usize {
        self.edges.iter().filter(|t| t.has(x)).count()
    }

    pub fn contains(&self, t: &Bits<W>) -> bool {
        self.edges.contains(t)
    }

    /// f ∨ g: the union of term sets, unreduced (fka `vee`).
    pub fn vee(&self, o: &Self) -> Self {
        Mbf::new(self.n, self.edges.iter().chain(&o.edges).copied().collect())
    }

    /// φ_{x,1}: the terms without x.
    pub fn phi1(&self, x: usize) -> Self {
        Mbf::new(self.n, self.edges.iter().filter(|t| !t.has(x)).copied().collect())
    }

    /// φ_{x,0}: the terms with x, x deleted; a term {x} becomes the empty term and is kept.
    pub fn phi0(&self, x: usize) -> Self {
        Mbf::new(self.n, self.edges.iter().filter(|t| t.has(x)).map(|t| t.without(x)).collect())
    }

    /// The variables of c set FALSE (A_c_x): a CNF loses them from every clause, a DNF loses
    /// the monomials meeting them.
    pub fn set_false(&self, c: &Bits<W>, clauses: bool) -> Self {
        let e = if clauses {
            self.edges.iter().map(|t| t.minus(c)).collect()
        } else {
            self.edges.iter().filter(|t| !t.meets(c)).copied().collect()
        };
        Mbf::new(self.n, e)
    }

    /// The variables of m set TRUE (A_m_x), the mirror of `set_false`.
    pub fn set_true(&self, m: &Bits<W>, clauses: bool) -> Self {
        let e = if clauses {
            self.edges.iter().filter(|t| !t.meets(m)).copied().collect()
        } else {
            self.edges.iter().map(|t| t.minus(m)).collect()
        };
        Mbf::new(self.n, e)
    }

    /// The minimal terms (fka `irredundant(...).reduced`).
    pub fn irredundant(&self) -> Self {
        Mbf { n: self.n, edges: minimal(self.edges.clone()) }
    }
}

/// The inclusion-minimal sets, in canonical order (fka `minimal_elements`, re-sorted).
/// Ascending size: a set can only contain a strictly smaller one.
pub fn minimal<const W: usize>(mut s: Vec<Bits<W>>) -> Vec<Bits<W>> {
    s.sort_by(|a, b| a.len().cmp(&b.len()).then(a.lex(b)));
    s.dedup();
    let mut kept: Vec<Bits<W>> = Vec::new();
    for t in s {
        if !kept.iter().any(|k| k.sub(&t)) {
            kept.push(t);
        }
    }
    kept.sort_by(|a, b| a.lex(b));
    kept
}
