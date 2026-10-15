"""Stage 0 audit of a QBL snapshot: what the qualification matrix supports (docs/DATA.md)."""
from collections import Counter, defaultdict

from . import catalog, qbl
from .instance import Instance


def matrix(rows, variant="strict"):
    """Qualification matrix of a variant.
    OUTPUT S, Q, issues (qbl.records), D (catalog), M = {product: (class, E_p)}."""
    S, Q, issues = qbl.records(rows)
    D = catalog.load({t for _, t, _ in Q})
    j = 1 if variant == "strict" else 2
    E = defaultdict(set)
    cls = {}
    for s, t, _ in Q:
        for p in D[t][j]:
            E[p].add(s)
            cls.setdefault(p, D[t][0])
    return S, Q, issues, D, {p: (cls[p], E[p]) for p in E}


def basket(M, P, k=0):
    """Instance for basket P with every eligible supplier: V = ∪ E_p."""
    E = [M[p][1] for p in P]
    return Instance(V=tuple(sorted(set().union(*E))), P=tuple(P), E=E, k=k)


def hist(xs, top=3):
    h = Counter(min(x, top) for x in xs)
    return {f"{n}" if n < top else f">={top}": h[n] for n in range(top + 1)}


def audit(rows):
    """Stage 0 questions (docs/DATA.md): identities, tokens, granularity, supplier counts
    per product and per supplier, repeated signatures, share of precise incidences."""
    S, Q, issues, D, M = matrix(rows)
    kinds = Counter(k for _, k, _ in issues)
    by_class = defaultdict(list)
    for (c, E) in M.values():
        by_class[c].append(len(E))
    deg = Counter(s for _, E in M.values() for s in E)
    sig = Counter(frozenset(p for p, (c, E) in M.items() if s in E and c != "broad") for s in S)
    sig.pop(frozenset(), None)
    inc = Counter(D[t][0] for _, t, _ in Q)
    precise = sum(inc[c] for c in ("material", "naics", "program", "description"))
    return {
        "rows": len(rows),
        "rows_usable": len(rows) - kinds["missing_name"] - kinds["missing_product"],
        "issues": dict(kinds),
        "suppliers": len(S),
        "multiline_cells": sum("\n" in str(r["Product"]).strip() for r in rows),
        "tokens": len(D),
        "tokens_by_class": dict(Counter(c for c, *_ in D.values())),
        "multi_product_tokens": sum(len(s) > 1 for _, s, _, _ in D.values()),
        "mapped_by_judgement": sum("judgement" in j for *_, j in D.values()),
        "products": len(M),
        "suppliers_per_product": {c: hist(v) for c, v in sorted(by_class.items())},
        "products_per_supplier": hist([deg[s] for s in S], top=5),
        "signatures_nonbroad": len(sig),
        "signatures_repeated": sum(n > 1 for n in sig.values()),
        "suppliers_in_repeated_signatures": sum(n for n in sig.values() if n > 1),
        "incidences": len(Q),
        "incidences_precise_share": round(precise / len(Q), 3),
    }
