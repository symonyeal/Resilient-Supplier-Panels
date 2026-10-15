"""Benchmark suite on the frozen snapshot (docs/BENCHMARKS.md).

Baskets: per granularity class, size s ∈ SIZES, rule ∈ {uniform, overlap, scarce}, seed.
Each enumeration runs in its own process with a time limit and an output cap; a stopped
run is partial and is reported as censored, a crashed one as an error.
"""
import hashlib
import json
import multiprocessing as mp
import platform
import random
import re
import subprocess
import time
from importlib.metadata import version
from pathlib import Path

from fka.algorithm import fk_a
from fka.memo import fk_a_memo
from fka.hypergraph import Hypergraph

from . import audit, catalog, certify, qbl, solve
from .instance import Instance, hypergraph

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "data/raw/QBL.xlsx"
THESIS = re.search(r"Hypergraph-Transversal-Thesis@(\w+)",
                   (ROOT / "requirements.txt").read_text()).group(1)
SIZES = (3, 5, 8, 12, 20)
CLASSES = ("description", "program", "naics", "material")
# Main-suite limits, set from calibration (docs/BENCHMARKS.md, Calibration record).
LIMITS = {"timeout_s": 10.0, "cap": 10_000, "K": 10, "fka_nodes": 200_000,
          "fka_max_family": 2_000, "exhaustive_support": 18, "repeat_below_s": 1.0,
          "repeats": 3}
STREAMS = {"berge": solve.berge, "fkb": solve.fkb_stream, "milp": solve.milp_stream}


def data_sha256():
    return hashlib.sha256(RAW.read_bytes()).hexdigest()


def baskets(M, seed):
    """[(class, rule, s, P)] for one seed. M: product -> (class, E_p)."""
    rng, out = random.Random(seed), []
    for c in CLASSES:
        prods = sorted(p for p, (cl, _) in M.items() if cl == c)
        starts = [p for p in prods if len(M[p][1]) >= 2]
        for s in SIZES:
            if len(prods) < s:
                continue
            out.append((c, "uniform", s, sorted(rng.sample(prods, s))))
            if not starts:
                continue
            P = [rng.choice(starts)]
            while len(P) < s:                            # greedy Jaccard to the union
                U = set().union(*(M[p][1] for p in P))
                rest = [q for q in prods if q not in P]
                rng.shuffle(rest)
                P.append(max(rest, key=lambda q: len(M[q][1] & U) / len(M[q][1] | U)))
            out.append((c, "overlap", s, sorted(P)))
        scarce = [p for p in prods if len(M[p][1]) <= 2]
        if len(scarce) >= 3:
            out.append((c, "scarce", 3, sorted(rng.sample(scarce, 3))))
    return out


def _child(method, I, conn):
    conn.send(("start", 0.0))
    t0 = time.perf_counter()
    for T in STREAMS[method](I):
        conn.send(("panel", time.perf_counter() - t0, sorted(T)))
    conn.send(("done", time.perf_counter() - t0))


def run(method, I, timeout, cap, K):
    """One process-isolated enumeration of Tr(H).

    OUTPUT  panels . the first ≤ cap panels received
            r ...... status complete | timeout | cap | error; count = panels received,
                     stored = panels kept. Berge emits only after it has finished, so it is
                     never stopped at the cap: its count is exact, its storage capped.
    """
    a, b = mp.Pipe(duplex=False)
    p = mp.Process(target=_child, args=(method, I, b), daemon=True)
    p.start()
    b.close()                                            # child death now reads as EOF
    panels, times, status, total, n = [], [], "timeout", None, 0
    try:
        a.poll(120) and a.recv()                         # child ready: start the clock
        t0 = time.perf_counter()
        while (left := timeout - (time.perf_counter() - t0)) > 0 and a.poll(left):
            msg = a.recv()
            if msg[0] == "done":
                status, total = "complete", msg[1]
                break
            n += 1
            if n <= cap:
                panels.append(frozenset(msg[2]))
                times.append(msg[1])
            elif method != "berge":
                status = "cap"
                break
    except EOFError:
        status = "error"
    p.terminate()
    p.join()
    return panels, {"method": method, "status": status, "count": n, "stored": len(panels),
                    "exitcode": p.exitcode if status == "error" else None,
                    "t_first": times[0] if times else None,
                    "t_K": times[K - 1] if len(times) >= K else None, "t_total": total}


def timed(method, I):
    """In-process rerun of a run already known to finish under a second."""
    t0 = time.perf_counter()
    for _ in STREAMS[method](I):
        pass
    return time.perf_counter() - t0


def stats(I):
    """Structure of an instance, and its supplier signature classes {E-membership: [v]}."""
    H, _, prov = hypergraph(I)
    sizes = sorted(len(E) for E in I.E)
    deg = sorted(sum(v in E for E in I.E) for v in I.V)
    sig = {}
    for v in I.V:
        sig.setdefault(tuple(v in E for E in I.E), []).append(v)
    return {"P": len(I.P), "V": len(I.V), "incidences": sum(sizes),
            "density": round(sum(sizes) / max(1, len(I.P) * len(I.V)), 4),
            "E_p_sizes": sizes,
            "supplier_degree_min_median_max": [deg[0], deg[len(deg) // 2], deg[-1]] if deg
            else [],
            "H_edges_after_reduction": len(H),
            "H_edges_before_reduction": sum(len(g) for g in prov.values()),
            "signature_classes": len(sig),
            "largest_class": max(map(len, sig.values()), default=0)}, sig


def orbits(C, sig):
    """|C| up to permutations within signature classes (they fix every E_p and 𝓕_k):
    the number of distinct class multisets."""
    cls = {v: i for i, vs in enumerate(sig.values()) for v in vs}
    return len({tuple(sorted(cls[v] for v in T)) for T in C})


def thesis_check(I, C, nodes):
    """FK-A decides C = Tr(H) (duality certificate); tree nodes against fk_a_memo's
    distinct subproblems on the same recursion."""
    H, labels, _ = hypergraph(I)
    idx = {v: i for i, v in enumerate(labels)}
    Cb = Hypergraph.from_bitsets(H.n, [sum(1 << idx[v] for v in T) for T in C])
    t0 = time.perf_counter()
    try:
        tree = fk_a(Cb, H, max_nodes=nodes)
    except RuntimeError:                                 # fka's node limit
        return {"status": "node_limit"}
    t1 = time.perf_counter()
    memo = fk_a_memo(Cb, H)
    t2 = time.perf_counter()
    return {"status": "dual" if tree.dual else "not_dual", "tree_nodes": len(tree),
            "dag_nodes": memo.nodes, "memo_agrees": memo.dual == tree.dual,
            "t_tree": t1 - t0, "t_memo": t2 - t1}


def instance_record(I, L):
    """Minimum, certificates, every route under the limits, and checks of complete families."""
    t0 = time.perf_counter()
    st, sig = stats(I)
    rec = {"stats": st, "t_build": time.perf_counter() - t0, "runs": []}
    t0 = time.perf_counter()
    status, T = solve.milp(I)
    rec["t_milp_min"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    rec["infeasibility"] = certify.infeasibility(I)
    ess = certify.essential(I)
    rec["min"] = {"status": status, "size": len(T) if T is not None else None,
                  "panel": sorted(T) if T is not None else None,
                  "certified_minimal": T is not None and certify.minimal(I, T)}
    rec["essential"] = None if ess is None else {v: list(w) for v, w in sorted(ess.items())}
    rec["t_certify_min"] = time.perf_counter() - t0
    rec["highs_agrees_with_witness"] = (status == "infeasible") == (rec["infeasibility"]
                                                                  is not None)
    fams = {}
    for m in STREAMS:
        panels, r = run(m, I, L["timeout_s"], L["cap"], L["K"])
        if r["status"] == "complete" and r["t_total"] < L["repeat_below_s"]:
            r["t_total_repeats"] = [r["t_total"]] + [timed(m, I)
                                                     for _ in range(L["repeats"] - 1)]
        t0 = time.perf_counter()
        r["all_certified_minimal"] = all(certify.minimal(I, T) for T in panels)
        r["t_certify"] = time.perf_counter() - t0
        rec["runs"].append(r)
        fams[m] = (r["status"] == "complete" and r["stored"] == r["count"], set(panels))
    rec["count_exact"] = next((r["count"] for r in rec["runs"] if r["status"] == "complete"),
                              None)
    full = [C for whole, C in fams.values() if whole]
    rec["complete_routes_agree"] = len({frozenset(C) for C in full}) <= 1 if full else None
    if full:
        C = full[0]
        t0 = time.perf_counter()
        rec["milp_exhaustion"] = certify.complete(I, C)[0]
        rec["t_exhaustion"] = time.perf_counter() - t0
        if len(I.support()) <= L["exhaustive_support"]:
            rec["exhaustive_agrees"] = set(solve.exhaustive(I)) == C
        rec["orbits"] = orbits(C, sig)
        rec["thesis"] = (thesis_check(I, C, L["fka_nodes"])
                         if C and len(C) <= L["fka_max_family"] else None)
        if ess is not None:
            rec["essential_matches_family"] = (
                {v for v in I.support() if all(v in T for T in C)} == set(ess))
    return rec


def provenance(L, seed):
    """Everything a result needs to be reproduced and dated."""
    git = lambda *a: subprocess.run(["git", *a], cwd=ROOT, capture_output=True, check=False,
                                    text=True).stdout.strip()
    sha = data_sha256()
    frozen = json.loads((ROOT / "data/manifest.json").read_text())["QBL.xlsx"]["sha256"]
    return {"data_sha256": sha, "data_matches_manifest": sha == frozen,
            "code_rev": git("rev-parse", "HEAD"),
            "dirty": bool(git("status", "--porcelain", "--untracked-files=no")),
            "thesis_rev": THESIS, "seed": seed, "limits": L,
            "versions": {p: version(p) for p in ("numpy", "scipy", "fka")},
            "python": platform.python_version(), "os": platform.platform(),
            "cpu": platform.processor(), "cpus": mp.cpu_count(), "memory": "not measured"}


def suite(seed, L=LIMITS, out=None, variants=True):
    """Run every basket of `seed` at k ∈ {0, 1}; sensitivity variants by one HiGHS solve."""
    t0 = time.perf_counter()
    rows = qbl.read(RAW)
    S, _, issues, D, M = audit.matrix(rows)
    MM = audit.matrix(rows, "merged")[4]
    merged = catalog.merged_of(D)
    first = {x["rows"][0]: sid for sid, x in S.items()}
    rep = {first[r]: d for r, k, d in issues if k == "identity_candidate"}   # merge into match
    res = {"provenance": provenance(L, seed), "t_matrix": time.perf_counter() - t0,
           "instances": []}
    t_start = time.perf_counter()
    for c, rule, s, P in baskets(M, seed):
        for k in (0, 1):
            I = audit.basket(M, P, k)
            rec = {"class": c, "rule": rule, "size": s, "k": k, "P": P, "variant": "strict"}
            rec.update(instance_record(I, L))
            if variants:
                Ec = [{rep.get(v, v) for v in E} for E in I.E]
                Ic = Instance(V=tuple(sorted(set().union(*Ec))), P=I.P, E=Ec, k=k)
                Im = audit.basket(MM, sorted({merged[p] for p in P}), k)
                rec["sensitivity"] = {name: sens(J) for name, J in
                                      (("merged", Im), ("identity_candidates_merged", Ic))}
            res["instances"].append(rec)
            print(f"{c:11} {rule:7} s={s:2} k={k} V={len(I.V):3} min={rec['min']['size']} "
                  + " ".join(f"{r['method']}:{r['status']}/{r['count']}" for r in rec["runs"]),
                  flush=True)
    res["t_suite"] = time.perf_counter() - t_start
    if out:
        Path(out).write_text(json.dumps(res, indent=1, default=list))
    return res


def sens(J):
    """Sensitivity variant: feasibility and minimum only (one HiGHS solve)."""
    status, T = solve.milp(J)
    return {"min": len(T) if T is not None else None, "feasible": status == "optimal",
            "status": status}
