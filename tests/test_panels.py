"""Seams 3 and 4: instance -> panels, panel -> certificate.

Hand-worked example: E = {ab, bc, cd}. For k = 0, Tr = {ac, bc, bd} by inspection
(ad misses bc). For k = 1, each |E_p| = 2 forces T = abcd.
"""
from srl.instance import Instance
from srl import certify, solve

P3 = Instance(V=tuple("abcd"), P=("p1", "p2", "p3"), E=(set("ab"), set("bc"), set("cd")))


def fam(panels):
    return {"".join(sorted(T)) for T in panels}


def test_hand_example_minimal_panels():
    assert fam(solve.berge(P3)) == {"ac", "bc", "bd"}
    assert fam(solve.exhaustive(P3)) == {"ac", "bc", "bd"}


def test_hand_example_minimum_and_resilience():
    status, T = solve.milp(P3)
    assert status == "optimal" and fam([T]) <= {"ac", "bc", "bd"}
    status, T = solve.milp(P3.with_k(1))
    assert (status, fam([T])) == ("optimal", {"abcd"})
    assert fam(solve.berge(P3.with_k(1))) == {"abcd"}


def test_witnesses_certify_minimality():
    W = certify.witnesses(P3, set("bd"))
    assert certify.feasible(P3, set("bd")) and set(W) == set("bd")
    E = dict(zip(P3.P, P3.E))
    assert all(set("bd") & E[p] == {v} for v, (p, F) in W.items())   # v alone covers p
    assert certify.witnesses(P3, set("abd"))["a"] is None   # a is redundant in abd


def test_essential_suppliers():
    assert certify.essential(P3) == {}
    assert set(certify.essential(P3.with_k(1))) == set("abcd")
