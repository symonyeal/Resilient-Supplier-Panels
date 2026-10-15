"""Seams 3 and 4 on edge cases, and agreement of independent routes on random instances.

Expected values are worked by hand (docstrings) or come from exhaustive search over the
matrix, which shares no code with H, Berge or FK-B.
"""
import random

from fkb.dualize import dualise

from srl.instance import Instance, hypergraph
from srl import certify, solve

ROUTES = {"berge": solve.berge, "fkb": lambda I: list(solve.fkb_stream(I)),
          "milp": lambda I: list(solve.milp_stream(I)), "exhaustive": solve.exhaustive}


def all_routes(I):
    out = {m: set(f(I)) for m, f in ROUTES.items()}
    assert len({frozenset(s) for s in out.values()}) == 1, out
    return out["exhaustive"]


def test_empty_basket_has_only_the_empty_panel():
    assert all_routes(Instance(V=tuple("ab"), P=(), E=())) == {frozenset()}


def test_empty_edge_is_infeasible_with_witness():
    I = Instance(V=tuple("ab"), P=("p", "q"), E=(set("a"), set()))
    assert all_routes(I) == set() and certify.infeasibility(I) == ("q", ())
    assert solve.milp(I) == ("infeasible", None) and certify.essential(I) is None


def test_single_supplier_product_is_essential_and_kills_resilience():
    """E = {a, bc}: k=0 panels ab, ac with a essential; k=1 needs |E_p| ≥ 2, fails on p."""
    I = Instance(V=tuple("abc"), P=("p", "q"), E=(set("a"), set("bc")))
    assert all_routes(I) == {frozenset("ab"), frozenset("ac")}
    assert set(certify.essential(I)) == {"a"}
    assert all_routes(I.with_k(1)) == set() and certify.infeasibility(I.with_k(1))[0] == "p"


def test_duplicate_and_superset_edges_are_removed_without_changing_tr():
    I = Instance(V=tuple("abc"), P=("p", "q", "r"), E=(set("ab"), set("ab"), set("abc")))
    H, _labels, prov = hypergraph(I)
    assert len(H) == 1 and sorted(p for p, _ in prov[H.edges[0]]) == ["p", "q"]
    assert all_routes(I) == {frozenset("a"), frozenset("b")}


def test_identical_signatures_both_needed_for_resilience():
    """E_p = E_q = {a, b}: interchangeable for k=0 (panels a, b), both needed for k=1."""
    I = Instance(V=tuple("abc"), P=("p", "q"), E=(set("ab"), set("ab")))
    assert all_routes(I) == {frozenset("a"), frozenset("b")}
    assert all_routes(I.with_k(1)) == {frozenset("ab")}


def test_correlated_scenario():
    """E_p = abc, a and b fail together (F = ab): edges abc and c, so Tr = {c}."""
    I = Instance(V=tuple("abc"), P=("p",), E=(set("abc"),), F=(set("ab"),))
    assert all_routes(I) == {frozenset("c")} and set(certify.essential(I)) == {"c"}


def test_fkb_stream_equals_thesis_dualise():
    I = random_instance(random.Random(7), n=9, m=6)
    H, labels, _ = hypergraph(I)
    D = {frozenset(labels[i] for i in range(H.n) if c >> i & 1) for c in dualise(H).cnf.edges}
    assert set(solve.fkb_stream(I)) == D


def random_instance(rng, n, m, k=0):
    V = tuple(f"s{i}" for i in range(n))
    E = [set(rng.sample(V, rng.randint(1, max(1, n // 2)))) for _ in range(m)]
    return Instance(V=V, P=tuple(f"p{j}" for j in range(m)), E=E, k=k)


def test_random_agreement_certificates_and_permutation_invariance():
    rng = random.Random(0)
    for trial in range(60):
        I = random_instance(rng, n=rng.randint(3, 10), m=rng.randint(1, 6), k=trial % 3)
        Tr = all_routes(I)
        assert all(certify.minimal(I, T) for T in Tr)
        assert certify.complete(I, Tr) == ("complete", None)
        if Tr:
            T = next(iter(Tr))
            assert certify.complete(I, Tr - {T}) == ("missing", T)
            assert {v for v in I.support() if all(v in T for T in Tr)} == set(certify.essential(I))
            assert len(solve.milp(I)[1]) == min(map(len, Tr))
        else:
            assert certify.infeasibility(I) is not None
        pi = list(I.V)
        rng.shuffle(pi)
        J = Instance(V=tuple(pi), P=I.P[::-1], E=I.E[::-1], k=I.k)
        assert set(solve.berge(J)) == Tr
