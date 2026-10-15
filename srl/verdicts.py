"""Pre-registered hypotheses H1–H5 and kill conditions K1–K7 (docs/findings/hypotheses.md),
scored from saved results only."""
import json
from pathlib import Path
from statistics import median

from . import audit, qbl
from .bench import RAW


def score(path):
    R = json.loads(Path(path).read_text())
    X = R["instances"]
    prim = [x for x in X if x["rule"] in ("uniform", "overlap")]
    feas = [x for x in prim if x["min"]["size"] is not None]
    exact = [x for x in feas if x["count_exact"] is not None]
    A = audit.audit(qbl.read(RAW))
    sp = A["suppliers_per_product"]
    prec = ("material", "naics", "program", "description")
    n3 = sum(sp[c][">=3"] for c in prec)
    n = sum(sum(sp[c].values()) for c in prec)
    small1 = [x for x in prim if x["k"] == 1 and x["size"] <= 8]
    ok1 = [x for x in small1 if x["min"]["size"] is not None]
    many = [x for x in exact if x["count_exact"] >= 2]
    ess_sub = [x for x in feas if len(x["essential"]) < x["min"]["size"]]
    th = [x["thesis"] for x in X if x.get("thesis") and x["thesis"]["status"] == "dual"]
    ratio = [t["tree_nodes"] / t["dag_nodes"] for t in th]
    res1 = [x for x in exact if x["k"] == 1 and x.get("orbits")]
    comp = [x["count_exact"] / x["orbits"] for x in res1]
    runs = [r for x in prim for r in x["runs"]]
    stopped = [r for r in runs if r["status"] in ("timeout", "cap")]

    def total(x, m):
        r = next(r for r in x["runs"] if r["method"] == m)
        return r["t_total"] if r["status"] == "complete" else float("inf")

    fkb_slowest = [x for x in X if total(x, "fkb") > min(total(x, "berge"), total(x, "milp"))]
    fkb_beats_berge = [x for x in X if total(x, "fkb") < total(x, "berge")]

    def first(x, m):
        r = next(r for r in x["runs"] if r["method"] == m)
        return r["t_first"] if r["t_first"] is not None else float("inf")

    late = [x for x in X if total(x, "berge") > 0.1 and x["min"]["size"] is not None]
    h2v = {v: sum(x["sensitivity"][v]["feasible"] for x in small1)
           for v in ("merged", "identity_candidates_merged")}
    dirn = {v: {"infeasible->feasible": sum(x["sensitivity"][v]["feasible"]
                                             and x["min"]["size"] is None for x in prim),
                "feasible->infeasible": sum(not x["sensitivity"][v]["feasible"]
                                             and x["min"]["size"] is not None for x in prim)}
            for v in ("merged", "identity_candidates_merged")}
    flip = {v: sum(x["sensitivity"][v]["feasible"] != (x["min"]["size"] is not None)
                   for x in prim) for v in ("merged", "identity_candidates_merged")}
    dmin = {v: sum(x["sensitivity"][v]["feasible"] and x["min"]["size"] is not None
                   and x["sensitivity"][v]["min"] != x["min"]["size"] for x in prim)
            for v in ("merged", "identity_candidates_merged")}
    return {
        "H1 precise products with >=3 suppliers": f"{n3}/{n} = {n3 / n:.3f} (threshold 0.25)",
        "H2 primary size<=8 baskets k=1 feasible": f"{len(ok1)}/{len(small1)} (threshold 0.5)",
        "H3 feasible primary cases with exact |Tr|>=2": f"{len(many)}/{len(exact)} "
        f"(threshold 0.5); exact count unknown in {len(feas) - len(exact)}",
        "H3 essential set strictly smaller than minimum": f"{len(ess_sub)}/{len(feas)}",
        "H4 tree/DAG >= 2": f"{sum(r >= 2 for r in ratio)}/{len(ratio)}; median "
        f"{median(ratio):.2f}" if ratio else "no certified instance",
        "H4 signature compression >= 2 on k=1": f"{sum(c >= 2 for c in comp)}/{len(comp)}",
        "H5 / K7 feasibility verdict flips (primary)": flip,
        "H5 / K7 flip directions": dirn,
        "H5 / K7 H2 numerator under variant (of the same baskets)": h2v,
        "H5 / K7 minimum changes where both feasible (primary)": dmin,
        "K1 precise incidence share": A["incidences_precise_share"],
        "K3 size<=8 k=1 feasible share": round(len(ok1) / len(small1), 3),
        "K4 median exact |Tr| over feasible primary": median(x["count_exact"] for x in exact),
        "K5 primary runs stopped (timeout or cap)": f"{len(stopped)}/{len(runs)}",
        "K6 instances where FK-B is slower than the faster of Berge and MILP":
            f"{len(fkb_slowest)}/{len(X)}",
        "K6 instances where FK-B beats Berge": f"{len(fkb_beats_berge)}/{len(X)}; slowest "
        f"Berge among them {max(total(x, 'berge') for x in fkb_beats_berge):.4f} s"
        if fkb_beats_berge else "0",
        "first output: Berge > 0.1 s, FK-B first faster":
            f"{sum(first(x, 'fkb') < first(x, 'berge') for x in late)}/{len(late)}",
        "rival: median MILP minimum time (s)": round(median(x["t_milp_min"] for x in X), 4),
        "rival: max MILP minimum time (s)": round(max(x["t_milp_min"] for x in X), 4),
    }


if __name__ == "__main__":
    import sys
    for k, v in score(sys.argv[1]).items():
        print(f"{k}: {v}")
