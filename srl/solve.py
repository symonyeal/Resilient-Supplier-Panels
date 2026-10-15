"""Panels: minimum cardinality (MILP), and minimal-panel enumeration by four routes.

min Σ_v x_v  s.t.  Σ_{v∈E_p} x_v ≥ k + 1,  Σ_{v∈E_p\\F} x_v ≥ 1 (explicit F),  x ∈ {0,1}^V.
Unit weights count suppliers; they are not costs.
"""
from itertools import combinations

import numpy as np
from scipy.optimize import Bounds, LinearConstraint, milp as highs

from fka.hypergraph import Hypergraph
from fka.transversal import transversal
from fkb.algorithm import cnf_value, dnf_value, fk_b
from fkb.dualize import initial_cnf, maximum_false_point

from . import certify
from .instance import decode, hypergraph


def _rows(I, U):
    """Rows A x ≥ b of the feasibility system over suppliers U."""
    A, b = [], []
    for Ep in I.E:
        A.append([v in Ep for v in U])
        b.append(I.k + 1)
        for F in I.F:
            A.append([v in Ep - F for v in U])
            b.append(1)
    return A, b


def milp(I, cuts=()):
    """Minimum-cardinality feasible panel containing no S ∈ cuts.
    OUTPUT ("optimal", T) | ("infeasible", None) | ("highs status s", None)."""
    U = I.support()
    A, b = _rows(I, U)
    for S in cuts:                                       # Σ_{v∈S} x_v ≤ |S| − 1
        A.append([-(v in S) for v in U])
        b.append(1 - len(S))
    if not U:
        ok = not any(b_i > 0 for b_i in b)
        return ("optimal", frozenset()) if ok else ("infeasible", None)
    r = highs(np.ones(len(U)), integrality=np.ones(len(U)), bounds=Bounds(0, 1),
              constraints=LinearConstraint(np.array(A, float), b, np.inf))
    if r.status != 0:
        return ("infeasible" if r.status == 2 else f"highs status {r.status}"), None
    return "optimal", frozenset(v for v, x in zip(U, r.x) if x > 0.5)


def milp_stream(I):
    """All minimal panels in nondecreasing size: an optimum avoiding supersets of the
    panels found is minimal (a proper feasible subset would be cheaper and still
    avoid them); infeasibility of the cut problem proves completeness."""
    found = []
    while (r := milp(I, found))[0] == "optimal":
        found.append(r[1])
        yield r[1]


def exhaustive(I):
    """Tr by checking every subset of the support against the matrix. O(2^n |P| |F|)."""
    U, found = I.support(), []
    assert len(U) <= 22, "exhaustive search is for small supports"
    for r in range(len(U) + 1):
        for T in map(frozenset, combinations(U, r)):
            if not any(S <= T for S in found) and certify.feasible(I, T):
                found.append(T)
    return found


def berge(I):
    """Tr(H) by Berge's method (thesis oracle, fka.transversal)."""
    H, labels, _ = hypergraph(I)
    return [decode(labels, t) for t in transversal(H).edges]


def fkb_stream(I):
    """Tr(H) by FK-B self-reduction (fkb.dualize.dualise, one clause per pass):
    C ← initial guess; while FK-B finds S with C(S)=1, D(S)=0, add the complement of a
    maximal false point of D above S. Yields each member of Tr(H) as it is found."""
    D, labels, _ = hypergraph(I)
    if not D.edges or 0 in D.edges:                      # Tr(∅) = {∅}; Tr ∋ ∅-edge = ∅
        yield from ([frozenset()] if not D.edges else [])
        return
    C = initial_cnf(D)
    yield from (decode(labels, c) for c in C.edges)
    while not (tree := fk_b(C, D)).dual:
        S = tree.verdict.CA
        assert cnf_value(C, S) and not dnf_value(D, S)
        c = D.support() & ~maximum_false_point(D, S)
        C = Hypergraph.from_bitsets(C.n, C.edges + (c,))
        yield decode(labels, c)
