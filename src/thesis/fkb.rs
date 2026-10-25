//! FK-B (fkb `algorithm.fk_b`, variant "faithful", split rule "most_frequent") and dualisation
//! by repeated equivalence tests (fkb `dualize`, FK_Dualization.m).
//!
//! FK-B decides whether the CNF C and the DNF D are one function (C = Tr(D)); otherwise it
//! returns S with C(S) ≠ D(S). At a split on x it recurses once on the x = 0 (or x = 1)
//! restriction and once per clause (or monomial) of the other side when x is at most
//! μ-frequent there, μ(n) = log n / log log n; otherwise it splits both ways.
//!
//! Dualisation: C ← one greedy clause; while FK-B finds S with C(S) = 1, D(S) = 0, add the
//! complement of a maximal false point of D above S, a new member of Tr(D).

use super::{berge, Mbf};
use crate::bits::Bits;
use crate::run::{Clock, Stop};

fn cnf_value<const W: usize>(C: &Mbf<W>, S: &Bits<W>) -> bool {
    C.edges.iter().all(|c| c.meets(S))
}

fn dnf_value<const W: usize>(D: &Mbf<W>, S: &Bits<W>) -> bool {
    D.edges.iter().any(|m| m.sub(S))
}

fn low<const W: usize>(t: &Bits<W>) -> Bits<W> {
    t.lowest().map_or(Bits::EMPTY, Bits::unit)
}

/// The first term of largest size, in canonical order.
fn largest<const W: usize>(M: &Mbf<W>) -> Bits<W> {
    let s = M.edges.iter().map(|t| t.len()).max().unwrap();
    *M.edges.iter().find(|t| t.len() == s).unwrap()
}

/// Conditions 1–3 and the constant sides (fkb `check_conditions`): None when all hold, else
/// the decided verdict, Some(None) for equivalence and Some(Some(S)) for a conflict.
fn conditions<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<Option<Bits<W>>> {
    let (Cs, Ds) = (C.support(), D.support());
    let V = Cs.or(&Ds);
    let c = if C.is_empty() { Some(1) } else if C.contains(&Bits::EMPTY) { Some(0) } else { None };
    let d = if D.is_empty() { Some(0) } else if D.contains(&Bits::EMPTY) { Some(1) } else { None };
    if c.is_some() || d.is_some() {
        let S = match (c, d) {
            (Some(c), Some(d)) if c == d => return Some(None),
            (Some(c), Some(_)) => if c == 1 { Bits::EMPTY } else { V },
            (Some(1), None) => Bits::EMPTY,
            (Some(_), None) => V,
            (None, Some(0)) => C.edges.iter().fold(Bits::EMPTY, |S, c| S.or(&low(c))),
            _ => Bits::EMPTY,
        };
        return Some(Some(S));
    }
    // 1: a monomial disjoint from some clause.
    if let Some(m) = D.edges.iter().find(|m| C.edges.iter().any(|c| !c.meets(m))) {
        return Some(Some(*m));
    }
    // 2: unequal supports.
    if Cs != Ds {
        let xd = Ds.minus(&Cs);
        let S = if !xd.is_empty() {
            D.edges.iter().find(|m| m.meets(&xd)).unwrap().minus(&xd)
        } else {
            let xc = Cs.minus(&Ds);
            Cs.minus(C.edges.iter().find(|c| c.meets(&xc)).unwrap()).or(&xc)
        };
        return Some(Some(S));
    }
    // 3: a term longer than the other side's term count.
    if D.edges.iter().any(|t| t.len() > C.len()) {
        let m = largest(D);
        return Some(Some(C.edges.iter().fold(Bits::EMPTY, |S, c| S.or(&low(&c.and(&m))))));
    }
    if C.edges.iter().any(|t| t.len() > D.len()) {
        let c = largest(C);
        let T = D.edges.iter().fold(Bits::EMPTY, |T, m| T.or(&low(&m.and(&c))));
        return Some(Some(V.minus(&T)));
    }
    None
}

/// min(nC, nD) ≤ 2, decided from the minimal transversals of the small side.
fn easy<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<Bits<W>> {
    let V = C.support().or(&D.support());
    let tr = |M: &Mbf<W>| berge::transversal(M, &Clock::unlimited(), usize::MAX).unwrap();
    if D.len() <= 2 {
        tr(D).into_iter().map(|t| V.minus(&t)).find(|S| cnf_value(C, S))
    } else {
        tr(C).into_iter().find(|s| !dnf_value(D, s))
    }
}

fn mu(n: usize) -> f64 {
    if n < 3 { f64::INFINITY } else { (n as f64).ln() / (n as f64).ln().ln() }
}

/// x is at most μ-frequent in A: deg_A(x)/|A| ≤ 1/μ(|A||B|).
fn mu_frequent<const W: usize>(x: usize, A: &Mbf<W>, B: &Mbf<W>) -> bool {
    !A.is_empty() && A.degree(x) as f64 / A.len() as f64 <= 1.0 / mu(A.len() * B.len())
}

/// 'mostFreq': walk both degree orders down together; the least variable in both top-i lists.
fn split_var<const W: usize>(C: &Mbf<W>, D: &Mbf<W>) -> Option<usize> {
    let live = C.support().or(&D.support()).to_vec();
    let order = |M: &Mbf<W>| {
        let mut o = live.clone();
        o.sort_by_key(|&v| (std::cmp::Reverse(M.degree(v)), v));
        o
    };
    let (bc, bd) = (order(C), order(D));
    (1..=live.len()).find_map(|i| bc[..i].iter().filter(|v| bd[..i].contains(v)).min().copied())
}

/// Decides C = Tr(D). OUTPUT Ok((S, nodes)): S = None for equivalence, else a conflicting
/// assignment; Err once the clock expires.
pub fn fk_b<const W: usize>(C: &Mbf<W>, D: &Mbf<W>, clock: &Clock)
                            -> Result<(Option<Bits<W>>, usize), Stop> {
    let mut nodes = 0;
    let S = node(C, D, &mut nodes, clock)?;
    Ok((S, nodes))
}

fn node<const W: usize>(Cin: &Mbf<W>, Din: &Mbf<W>, nodes: &mut usize, clock: &Clock)
                        -> Result<Option<Bits<W>>, Stop> {
    *nodes += 1;
    if clock.expired() {
        return Err(Stop::Timeout);
    }
    let (C, D) = (Cin.irredundant(), Din.irredundant());
    if let Some(v) = conditions(&C, &D) {
        return Ok(v);
    }
    if C.len().min(D.len()) <= 2 {
        return Ok(easy(&C, &D));
    }
    let x = split_var(&C, &D).expect("a live variable past condition 2");
    let bit = Bits::unit(x);
    let (C0, C1, D0, D1) = (C.phi0(x), C.phi1(x), D.phi0(x), D.phi1(x));
    let mut go = |C: Mbf<W>, D: Mbf<W>, add: Bits<W>| -> Result<Option<Bits<W>>, Stop> {
        Ok(node(&C, &D, nodes, clock)?.map(|S| S.or(&add)))
    };
    let f = |A: &Mbf<W>, s: &Bits<W>, B: &Mbf<W>| (A.set_false(s, true), B.set_false(s, false));
    let t = |A: &Mbf<W>, s: &Bits<W>, B: &Mbf<W>| (A.set_true(s, true), B.set_true(s, false));
    if mu_frequent(x, &D, &C) {
        let (a, b) = f(&C, &bit, &D);
        if let Some(S) = go(a, b, Bits::EMPTY)? {
            return Ok(Some(S));
        }
        for c in &C0.edges {                             // each clause of C_x0 forced FALSE
            let (a, b) = f(&C1, c, &D0);
            if let Some(S) = go(a, b, bit)? {
                return Ok(Some(S));
            }
        }
    } else if mu_frequent(x, &C, &D) {
        let (a, b) = t(&C, &bit, &D);
        if let Some(S) = go(a, b, bit)? {
            return Ok(Some(S));
        }
        for m in &D0.edges {                             // each monomial of D_x0 forced TRUE
            let (a, b) = t(&C0, m, &D1);
            if let Some(S) = go(a, b, *m)? {
                return Ok(Some(S));
            }
        }
    } else {
        let (a, b) = f(&C, &bit, &D);
        if let Some(S) = go(a, b, Bits::EMPTY)? {
            return Ok(Some(S));
        }
        let (a, b) = t(&C, &bit, &D);
        if let Some(S) = go(a, b, bit)? {
            return Ok(Some(S));
        }
    }
    Ok(None)
}

/// A transversal of D from the variables in `allowed`, greedily by most surviving monomials,
/// least index on ties; None when some monomial has no allowed variable.
fn greedy_cover<const W: usize>(D: &Mbf<W>, allowed: &Bits<W>) -> Option<Bits<W>> {
    let mut left: Vec<Bits<W>> = D.edges.clone();
    let mut picked = Bits::EMPTY;
    while !left.is_empty() {
        let mut count = vec![0usize; D.n];
        for m in &left {
            for v in m.and(allowed).iter() {
                count[v] += 1;
            }
        }
        let best = (0..D.n).filter(|&v| count[v] > 0).max_by_key(|&v| (count[v], std::cmp::Reverse(v)))?;
        picked.insert(best);
        left.retain(|m| !m.has(best));
    }
    Some(picked)
}

/// Drops unneeded variables, highest index first (Minimality_Check.m).
fn minimality_check<const W: usize>(mut cover: Bits<W>, D: &Mbf<W>) -> Bits<W> {
    for v in cover.to_vec().into_iter().rev() {
        let without = cover.without(v);
        if D.edges.iter().all(|m| m.meets(&without)) {
            cover = without;
        }
    }
    cover
}

/// The complement within supp D of a maximal false point above S: a member of Tr(D).
fn new_clause<const W: usize>(D: &Mbf<W>, S: &Bits<W>) -> Bits<W> {
    let ground = D.support().or(S);
    let off = greedy_cover(D, &ground.minus(S)).expect("D(S) = 0 leaves every monomial coverable");
    minimality_check(off, D)
}

/// Tr(D) streamed as found: one greedy clause, then one clause per FK-B pass. `emit` returns
/// false to stop. OUTPUT the FK-B node count of each pass, or the stop reason.
pub fn dualise<const W: usize>(D: &Mbf<W>, clock: &Clock,
                               emit: &mut dyn FnMut(Bits<W>) -> bool) -> Result<Vec<usize>, Stop> {
    let D = D.irredundant();
    if D.is_empty() {
        emit(Bits::EMPTY);
        return Ok(Vec::new());
    }
    if D.contains(&Bits::EMPTY) {
        return Ok(Vec::new());
    }
    let first = minimality_check(greedy_cover(&D, &D.support()).unwrap(), &D);
    let mut C = Mbf::new(D.n, vec![first]);
    if !emit(first) {
        return Err(Stop::Cap);
    }
    let mut nodes = Vec::new();
    loop {
        let (S, n) = fk_b(&C, &D, clock)?;
        nodes.push(n);
        let Some(S) = S else { return Ok(nodes) };
        assert!(cnf_value(&C, &S) && !dnf_value(&D, &S), "expected C(S) = 1, D(S) = 0");
        let c = new_clause(&D, &S);
        assert!(!C.contains(&c), "dualisation repeated a clause");
        C = Mbf::new(C.n, C.edges.iter().copied().chain([c]).collect());
        if !emit(c) {
            return Err(Stop::Cap);
        }
        if clock.expired() {
            return Err(Stop::Timeout);
        }
    }
}
