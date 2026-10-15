"""Benchmark suite on the frozen snapshot (docs/BENCHMARKS.md).

Baskets: per granularity class, size s ∈ SIZES, rule ∈ {uniform, overlap, scarce}, seed.
Each enumeration runs in its own process with a time limit and an output cap; a stopped
run is partial and is reported as censored.
"""
import hashlib
import json
import multiprocessing as mp
import platform
import random
import subprocess
import time
from importlib.metadata import version
from pathlib import Path

from fka.algorithm import fk_a
from fka.memo import fk_a_memo
from fka.hypergraph import Hypergraph

from . import audit, certify, qbl, solve
from .instance import Instance, hypergraph

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "data/raw/QBL.xlsx"
THESIS = "1bb9de48ab91629a7ae9d97488c2181b4098806d"
SIZES = (3, 5, 8, 12, 20)
CLASSES = ("description", "program", "naics", "material")
LIMITS = {"timeout_s": 60.0, "cap": 10_000, "K": 10, "fka_nodes": 200_000,
          "exhaustive_support": 16, "repeat_below_s": 1.0, "repeats": 3}
STREAMS = {"berge": solve.berge, "fkb": solve.fkb_stream, "milp": solve.milp_stream}


def baskets(M, seed):
    """[(class, rule, s, P)] for one seed. M: product -> (class, E_p)."""
    rng, out = random.Random(seed), []
    for c in CLASSES:
        prods = sorted(p for p, (cl, _) in M.items() if cl == c)
        for s in SIZES:
            if len(prods) < s:
                continue
            out.append((c, "uniform", s, sorted(rng.sample(prods, s))))
            P = [rng.choice([p for p in prods if len(M[p][1]) >= 2])]
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
    """One process-isolated enumeration. status: complete | timeout | cap.
    count = panels received; stored = panels kept (≤ cap). Berge emits only after it has
    finished, so it is never stopped at the cap: its count is exact, its storage capped."""
    a, b = mp.Pipe(duplex=False)
    p = mp.Process(target=_child, args=(method, I, b), daemon=True)
    p.start()
    a.recv()                                             # child ready: start the clock
    t0, panels, times, status, total, n = time.perf_counter(), [], [], "timeout", None, 0
    while (left := timeout - (time.perf_counter() - t0)) > 0 and a.poll(left):
        msg = a.recv()
        if msg[0] == "done":
            status, total = "complete", msg[1]
            break
        n += 1
        if n <= cap:
            panels.append(frozenset(msg[2])), times.append(msg[1])
        elif method != "berge":
            status = "cap"
            break
    p.terminate(), p.join()
    return panels, {"method": method, "status": status, "count": n, "stored": len(panels),
                    "t_first": times[0] if times else None,
                    "t_K": times[K - 1] if len(times) >= K else None,
                    "t_total": total}


def stats(I):
    H, _labels, prov = hypergraph(I)
    raw = sum(len(v) for v in prov.values())
    sizes = sorted(len(E) for E in I.E)
    deg = sorted(sum(v in E for E in I.E) for v in I.V)
    sig = {}
    for v in I.V:
        sig.setdefault(tuple(v in E for E in I.E), []).append(v)
    return {"P": len(I.P), "V": len(I.V), "incidences": sum(sizes),
            "density": round(sum(sizes) / max(1, len(I.P) * len(I.V)), 4),
            "E_p_sizes": sizes, "supplier_degree": [deg[0], deg[len(deg) // 2], deg[-1]]
            if deg else [], "H_edges_after_reduction": len(H), "H_edges_kept_with_dups": raw,
            "signature_classes": len(sig), "largest_class": max(map(len, sig.values()),
                                                                default=0)}, sig


def orbits(family, sig):
    """Panels up to permutations within signature classes (they fix every E_p and the
    uniform failure model): count of distinct class-multisets."""
    cls = {v: i for i, vs in enumerate(sig.values()) for v in vs}
    return len({tuple(sorted(cls[v] for v in T)) for T in family})


def thesis_check(I, family, nodes):
    """FK-A duality certificate C = Tr(H), tree nodes against the memoised DAG."""
    H, labels, _ = hypergraph(I)
    idx = {v: i for i, v in enumerate(labels)}
    C = Hypergraph.from_bitsets(H.n, [sum(1 << idx[v] for v in T) for T in family])
    t0 = time.perf_counter()
    try:
        tree = fk_a(C, H, max_nodes=nodes)
    except RuntimeError:
        return {"status": "node_limit"}
    t1 = time.perf_counter()
    memo = fk_a_memo(C, H)
    t2 = time.perf_counter()
    return {"status": "dual" if tree.dual else "not_dual", "tree_nodes": len(tree),
            "dag_nodes": memo.nodes, "memo_agrees": memo.dual == tree.dual,
            "t_tree": t1 - t0, "t_memo": t2 - t1}


def instance_record(I, L):
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
    assert (status == "infeasible") == (rec["infeasibility"] is not None)
    fams = {}
    for m in STREAMS:
        panels, r = run(m, I, L["timeout_s"], L["cap"], L["K"])
        if r["status"] == "complete" and r["t_total"] < L["repeat_below_s"]:
            r["t_total_repeats"] = [r["t_total"]] + [
                run(m, I, L["timeout_s"], L["cap"], L["K"])[1]["t_total"]
                for _ in range(L["repeats"] - 1)]
        t0 = time.perf_counter()
        r["all_certified_minimal"] = all(certify.minimal(I, P) for P in panels)
        r["t_certify"] = time.perf_counter() - t0
        rec["runs"].append(r)
        fams[m] = (r["status"] == "complete" and r["stored"] == r["count"], set(panels))
    rec["count_exact"] = next((r["count"] for r in rec["runs"] if r["status"] == "complete"),
                              None)
    done = [f for full, f in fams.values() if full]
    rec["complete_routes_agree"] = len({frozenset(f) for f in done}) <= 1 if done else None
    if done:
        F = done[0]
        t0 = time.perf_counter()
        rec["milp_exhaustion"] = certify.complete(I, F)[0]
        rec["t_exhaustion"] = time.perf_counter() - t0
        if len(I.support()) <= L["exhaustive_support"]:
            rec["exhaustive_agrees"] = set(solve.exhaustive(I)) == F
        rec["count"] = len(F)
        rec["orbits"] = orbits(F, sig)
        rec["thesis"] = thesis_check(I, F, L["fka_nodes"]) if F else None
        if ess is not None:
            rec["essential_matches_family"] = (
                {v for v in I.support() if all(v in P for P in F)} == set(ess))
    return rec


def provenance(L, seed):
    git = lambda *a: subprocess.run(["git", *a], cwd=ROOT, capture_output=True, check=False,
                                    text=True).stdout.strip()
    return {"data_sha256": hashlib.sha256(RAW.read_bytes()).hexdigest(),
            "code_rev": git("rev-parse", "HEAD"), "dirty": bool(git("status", "--porcelain")),
            "thesis_rev": THESIS, "seed": seed, "limits": L,
            "versions": {p: version(p) for p in ("numpy", "scipy", "fka")},
            "python": platform.python_version(), "os": platform.platform(),
            "cpu": platform.processor(), "cpus": mp.cpu_count(), "memory": "not measured"}


def suite(seed, L=LIMITS, out=None, variants=True):
    rows = qbl.read(RAW)
    S, _Q, issues, D, M = audit.matrix(rows)
    MM = audit.matrix(rows, "merged")[4]
    merged = {p: m for _, s, mm, _ in D.values() for p, m in zip(s, mm)}
    first = {x["rows"][0]: sid for sid, x in S.items()}
    rep = {first[r]: d for r, k, d in issues if k == "identity_candidate"}   # merge into match
    res = {"provenance": provenance(L, seed), "instances": []}
    t_start = time.perf_counter()
    for c, rule, s, P in baskets(M, seed):
        for k in (0, 1):
            I = Instance(V=tuple(sorted(set().union(*(M[p][1] for p in P)))), P=tuple(P),
                         E=[M[p][1] for p in P], k=k)
            rec = {"class": c, "rule": rule, "size": s, "k": k, "P": P, "variant": "strict"}
            rec.update(instance_record(I, L))
            if variants:
                Pm = sorted({merged[p] for p in P})
                Im = Instance(V=tuple(sorted(set().union(*(MM[p][1] for p in Pm)))),
                              P=tuple(Pm), E=[MM[p][1] for p in Pm], k=k)
                Ec = [{rep.get(v, v) for v in E} for E in I.E]
                Ic = Instance(V=tuple(sorted(set().union(*Ec))), P=I.P, E=Ec, k=k)
                rec["sensitivity"] = {name: sens(J, L) for name, J in
                                      (("merged", Im), ("identity_candidates_merged", Ic))}
            res["instances"].append(rec)
            print(f"{c:11} {rule:7} s={s:2} k={k} V={len(I.V):3} min={rec['min']['size']} "
                  + " ".join(f"{r['method']}:{r['status']}/{r['count']}" for r in rec["runs"]),
                  flush=True)
    res["t_suite"] = time.perf_counter() - t_start
    if out:
        Path(out).write_text(json.dumps(res, indent=1, default=list))
    return res


def sens(J, L):
    status, T = solve.milp(J)
    panels, r = run("berge", J, L["timeout_s"], L["cap"], L["K"])
    return {"min": len(T) if T is not None else None, "feasible": status == "optimal",
            "berge": r["status"], "count": len(panels)}
