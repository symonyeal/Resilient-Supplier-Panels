//! FK-A (thesis Algorithm 1, p.13; fka `algorithm.fk_a`, variant "modified", split rule
//! "degree_sum") and its memoised subproblem DAG (fka `memo.fk_a_memo`).
//!
//! Step 1 reduces both sides; Step 2 checks conditions (i)–(iv); Step 3 settles nC·nD ≤ 1
//! and the single-term case D_{1,k} (§5.1.1); Step 4 splits on the variable of largest
//! deg_C + deg_D (least index on ties) into L = (C_x1, D_x0 ∨ D_x1), R = (D_x1, C_x0 ∨ C_x1).
//!
//! τ = nodes of the recursion tree, δ = distinct reduced pairs (C, D) among them; δ ≤ τ.

use std::collections::HashMap;

use super::{berge, Mbf};
use crate::bits::Bits;
use crate::run::Clock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict<const W: usize> {
    pub dual: bool,
    /// Conflicting assignment S (eq. 2.1): meets every clause, contains no monomial.
    pub CA: Option<Bits<W>>,
}

fn no<const W: usize>(CA: Option<Bits<W>>) -> Verdict<W> {
    Verdict { dual: false, CA }
}

/// S meets every clause of C and contains no monomial of D (eq. 2.1).
pub fn verify_ca<const W: usize>(C: &Mbf<W>, D: &Mbf<W>, S: &Bits<W>) -> bool {
    C.edges.iter().all(|c| c.meets(S)) && !D.edges.iter().any(|m| m.sub(S))
}

/// Conditions (i)–(iv) on irredundant sides; true when all hold.
fn conditions<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> bool {
    if C.support() != D.support() {
        return false;                                    // (i) equal supports
    }
    let big = |M: &Mbf<W>| M.edges.iter().map(|t| t.len()).max();
    if big(C).is_some_and(|s| D.len() < s) || big(D).is_some_and(|s| C.len() < s) {
        return false;                                    // (ii) |t| ≤ size of the other side
    }
    if C.edges.iter().any(|a| D.edges.iter().any(|b| !a.meets(b))) {
        return false;                                    // (iii) cross pairs intersect
    }
    // (iv) Σ 2^−|t| ≥ 1, exactly: total_s = ⌊Σ_{j≥s} cnt_j 2^{s−j}⌋ carried down to s = 0.
    let L = C.edges.iter().chain(&D.edges).map(|t| t.len()).max().unwrap_or(0);
    let mut cnt = vec![0u64; L + 1];
    for t in C.edges.iter().chain(&D.edges) {
        cnt[t.len()] += 1;
    }
    let total = (0..=L).rev().fold(0u64, |carry, s| cnt[s] + carry / 2);
    total >= 1
}

/// A conflicting assignment among the minimal transversals of C, or None (fka `find_CA`).
pub fn find_ca<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<Bits<W>> {
    let full = Bits::full(C.n.max(D.n));
    let T = berge::transversal(&C.irredundant(), &Clock::unlimited(), usize::MAX).unwrap();
    T.into_iter().find(|t| D.edges.iter().all(|m| m.meets(&full.minus(t))))
}

fn trivial_ca<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<Bits<W>> {
    let full = Bits::full(C.n.max(D.n));
    if C.len() == 1 {
        if let Some(u) = C.edges[0].iter().find(|&u| verify_ca(C, D, &Bits::unit(u))) {
            return Some(Bits::unit(u));
        }
    }
    if D.len() == 1 {
        if let Some(w) = D.edges[0].iter().find(|&w| verify_ca(C, D, &full.without(w))) {
            return Some(full.without(w));
        }
    }
    if C.is_empty() && verify_ca(C, D, &Bits::EMPTY) {
        return Some(Bits::EMPTY);
    }
    if D.is_empty() && verify_ca(C, D, &full) {
        return Some(full);
    }
    None
}

/// Step 3 and the modified Step 3′; None when the pair is not trivial.
fn trivial<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<Verdict<W>> {
    let settle = |dual: bool| Verdict { dual, CA: if dual { None } else { trivial_ca(C, D) } };
    if C.len() * D.len() <= 1 {
        let dual = if C.is_empty() {
            D.len() == 1 && D.edges[0].is_empty()
        } else if D.is_empty() {
            C.len() == 1 && C.edges[0].is_empty()
        } else {
            C.edges[0].len() == 1 && C.edges[0] == D.edges[0]
        };
        return Some(settle(dual));
    }
    for (one, other) in [(C, D), (D, C)] {
        if one.len() == 1 {
            let t = one.edges[0];
            let dual = other.len() == t.len() && t.iter().all(|v| other.contains(&Bits::unit(v)));
            return Some(settle(dual));
        }
    }
    None
}

/// argmax_x deg_C(x) + deg_D(x) over live x, least index on ties.
fn split_var<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for x in 0..C.n.max(D.n) {
        let d = C.degree(x) + D.degree(x);
        if d > 0 && best.is_none_or(|(_, b)| d > b) {
            best = Some((x, d));
        }
    }
    best.map(|(x, _)| x)
}

/// Decides C = Tr(D). OUTPUT (verdict, τ), or None once the tree would exceed `max_nodes`.
pub fn fk_a<const W: usize>(C: &Mbf<W>, D: &Mbf<W>, max_nodes: usize) -> Option<(Verdict<W>, usize)> {
    let mut tau = 0;
    let v = tree(C, D, &mut tau, max_nodes)?;
    Some((v, tau))
}

fn tree<const W: usize>(Cin: &Mbf<W>, Din: &Mbf<W>, tau: &mut usize, max: usize)
                        -> Option<Verdict<W>> {
    if *tau >= max {
        return None;
    }
    *tau += 1;
    let (C, D) = (Cin.irredundant(), Din.irredundant());
    if !conditions(&C, &D) {
        return Some(no(find_ca(&C, &D)));
    }
    if let Some(v) = trivial(&C, &D) {
        return Some(v);
    }
    let x = split_var(&C, &D).expect("a live variable past Step 2");
    let (C0, C1, D0, D1) = (C.phi0(x), C.phi1(x), D.phi0(x), D.phi1(x));
    let left = tree(&C1, &D0.vee(&D1), tau, max)?;
    if !left.dual {
        return Some(no(left.CA.map(|S| S.with(x))));    // S + x still meets every clause
    }
    let right = tree(&D1, &C0.vee(&C1), tau, max)?;
    if !right.dual {
        // sides swapped: complement within V \ {x} (thesis p.11)
        return Some(no(right.CA.map(|S| Bits::full(C.n).without(x).minus(&S))));
    }
    Some(Verdict { dual: true, CA: None })
}

/// The same recursion memoised on the reduced pair. OUTPUT (dual, δ, depth).
pub fn fk_a_memo<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> (bool, usize, usize) {
    let mut memo: HashMap<(Mbf<W>, Mbf<W>), Option<bool>> = HashMap::new();
    let mut deepest = 0;
    let dual = dag(C, D, 0, &mut memo, &mut deepest);
    (dual, memo.len(), deepest)
}

type Memo<const W: usize> = HashMap<(Mbf<W>, Mbf<W>), Option<bool>>;

fn dag<const W: usize>(Cin: &Mbf<W>, Din: &Mbf<W>, depth: usize, memo: &mut Memo<W>,
                       deepest: &mut usize) -> bool {
    let key = (Cin.irredundant(), Din.irredundant());
    if let Some(hit) = memo.get(&key) {
        return hit.expect("cycle in the FK-A subproblem relation");
    }
    memo.insert(key.clone(), None);
    *deepest = (*deepest).max(depth);
    let (C, D) = &key;
    let out = if !conditions(C, D) {
        false
    } else if let Some(v) = trivial(C, D) {
        v.dual
    } else {
        let x = split_var(C, D).expect("a live variable past Step 2");
        let (C0, C1, D0, D1) = (C.phi0(x), C.phi1(x), D.phi0(x), D.phi1(x));
        dag(&C1, &D0.vee(&D1), depth + 1, memo, deepest)
            && dag(&D1, &C0.vee(&C1), depth + 1, memo, deepest)
    };
    memo.insert(key, Some(out));
    out
}
