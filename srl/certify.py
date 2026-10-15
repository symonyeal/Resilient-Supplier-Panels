"""Certificates read from the qualification matrix (E_p, k, F), never from H.

Feasible:  |T ∩ E_p| ≥ k + 1 and T ∩ (E_p \\ F) ≠ ∅ for every p and explicit F.
Minimal:   feasible, and every v ∈ T has a witness (p, F) with T ∩ (E_p \\ F) = {v}.
Essential: v lies in every minimal panel  ⇔  V \\ {v} is infeasible (monotonicity).
"""
import numpy as np
from scipy.optimize import Bounds, LinearConstraint, milp


def feasible(I, T):
    """T meets E_p minus F for every p and every F in the failure model (MODEL, Thm 2)."""
    T = set(T)
    return (all(len(T & Ep) >= I.k + 1 for Ep in I.E)
            and all(T & (Ep - F) for Ep in I.E for F in I.F))


def witnesses(I, T):
    """v -> (p, F) with T ∩ (E_p \\ F) = {v}, or None when v is removable."""
    T, W = set(T), {}
    for v in T:
        W[v] = None
        for p, Ep in enumerate(I.E):
            if v not in Ep:
                continue
            S = T & Ep
            if len(S) == I.k + 1:                       # tight: fail the other k
                W[v] = (I.P[p], tuple(sorted(S - {v})))
                break
            F = next((F for F in I.F if T & (Ep - F) == {v}), None)
            if F is not None:
                W[v] = (I.P[p], tuple(sorted(F)))
                break
    return W


def minimal(I, T):
    """T is feasible and every member has a witness (MODEL, Thm 3.1)."""
    return feasible(I, T) and all(w is not None for w in witnesses(I, T).values())


def infeasibility(I):
    """(p, F) with E_p \\ F = ∅, F in the failure model; None if V is feasible."""
    for p, Ep in enumerate(I.E):
        if len(Ep) <= I.k:
            return I.P[p], tuple(sorted(Ep))
        F = next((F for F in I.F if Ep <= F), None)
        if F is not None:
            return I.P[p], tuple(sorted(F))
    return None


def essential(I):
    """v -> (p, F) with E_p \\ F = {v}; None if the instance is infeasible."""
    if infeasibility(I):
        return None
    W = {}
    for p, Ep in enumerate(I.E):
        if len(Ep) == I.k + 1:
            for v in Ep:
                W.setdefault(v, (I.P[p], tuple(sorted(Ep - {v}))))
        for F in I.F:
            if len(Ep - F) == 1:
                W.setdefault(next(iter(Ep - F)), (I.P[p], tuple(sorted(F))))
    return W


def complete(I, family):
    """Is every minimal panel in `family`? Search min Σx over feasible T that contain no
    member of the family: Σ_{v∈S}(1 − x_v) ≥ 1 for S ∈ family. Encoded here, separately
    from solve.py. Returns ("complete", None), ("missing", a minimal panel not listed) or
    ("highs status s", None). Assurance: HiGHS result, not a proof."""
    U = I.support()
    i = {v: j for j, v in enumerate(U)}
    rows, lb = [], []
    for Ep in I.E:
        rows.append([v in Ep for v in U])
        lb.append(I.k + 1)
        for F in I.F:
            rows.append([v in Ep - F for v in U])
            lb.append(1)
    for S in family:
        rows.append([-(v in S) for v in U])
        lb.append(1 - len(S))
    if not U:
        return ("complete", None) if family or infeasibility(I) else ("missing", frozenset())
    r = milp(np.ones(len(U)), integrality=np.ones(len(U)), bounds=Bounds(0, 1),
             constraints=LinearConstraint(np.array(rows, float), lb, np.inf))
    if r.status == 2:
        return "complete", None
    if r.status != 0:
        return f"highs status {r.status}", None
    T = {v for v in U if r.x[i[v]] > 0.5}
    for v in sorted(T):                                  # shrink to a minimal panel
        if feasible(I, T - {v}):
            T -= {v}
    return "missing", frozenset(T)
