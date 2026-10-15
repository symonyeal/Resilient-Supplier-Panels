"""Instance and scenario hypergraph (docs/MODEL.md).

V suppliers, P basket, E_p ⊆ V the suppliers qualified for p. Failure model: every F
with |F| ≤ k, together with the explicit scenarios in F (∅ always included).
A panel T is feasible iff T ∩ (E_p \\ F) ≠ ∅ for all p, F; then
    H = min{E_p \\ F},   Tr(H) = the inclusion-minimal feasible panels  (MODEL.md, Thm 1).
"""
from dataclasses import dataclass, replace
from itertools import combinations

from fka.hypergraph import Hypergraph, verts
from fka.transversal import minimal_elements


@dataclass(frozen=True)
class Instance:
    V: tuple
    P: tuple
    E: tuple
    k: int = 0
    F: tuple = ()

    def __post_init__(self):
        object.__setattr__(self, "E", tuple(frozenset(e) for e in self.E))
        object.__setattr__(self, "F", tuple(frozenset(f) for f in self.F))
        assert len(self.P) == len(self.E) and self.k >= 0

    def with_k(self, k):
        return replace(self, k=k)

    def support(self):
        """Suppliers in some E_p, in V's order: the only ones a minimal panel can use."""
        U = set().union(*self.E)
        return tuple(v for v in self.V if v in U)


def scenarios(I, p):
    """The failed sets that matter for product p: k-subsets of E_p (or E_p itself if
    |E_p| ≤ k) and each explicit F ∩ E_p. Supersets of these give only subsets of edges."""
    Ep = I.E[p]
    yield from ([Ep] if len(Ep) <= I.k else map(frozenset, combinations(sorted(Ep), I.k)))
    for F in I.F:
        yield F & Ep


def hypergraph(I):
    """H on the support, deduplicated and superset-reduced (both preserve Tr exactly).

    OUTPUT  H ...... fka Hypergraph, vertex i = labels[i]
            labels . supplier ids
            prov ... edge -> [(p, F)] generating it, for every kept edge
    """
    labels = I.support()
    idx = {v: i for i, v in enumerate(labels)}
    prov = {}
    for p in range(len(I.P)):
        for F in scenarios(I, p):
            e = sum(1 << idx[v] for v in I.E[p] - F)
            prov.setdefault(e, []).append((I.P[p], tuple(sorted(F))))
    H = Hypergraph.from_bitsets(len(labels), minimal_elements(prov))
    return H, labels, {e: prov[e] for e in H.edges}


def decode(labels, t):
    """Bitset t over the support -> the set of supplier ids."""
    return frozenset(labels[i] for i in verts(t))
