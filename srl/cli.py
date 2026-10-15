"""python -m srl {acquire,audit,build,solve,enum,verify,bench,report}. No research logic."""
import argparse
import json
from datetime import UTC, datetime
from pathlib import Path

from . import audit, bench, certify, qbl, solve
from .instance import Instance


def today():
    return datetime.now(UTC).date().isoformat()


def load(path):
    d = json.loads(Path(path).read_text())
    return d, Instance(V=tuple(d["V"]), P=tuple(d["P"]), E=d["E"], k=d["k"], F=d.get("F", []))


def main(argv=None):
    ap = argparse.ArgumentParser(prog="srl")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("acquire", help="download the sources to data/fetched (network)")
    sub.add_parser("audit", help="Stage 0 audit of the frozen snapshot")
    b = sub.add_parser("build", help="instance JSON for a basket")
    b.add_argument("products", nargs="+")
    b.add_argument("--k", type=int, default=0)
    b.add_argument("--variant", choices=("strict", "merged"), default="strict")
    b.add_argument("--fail", action="append", default=[],
                   help="comma-separated supplier ids failing together (repeatable)")
    for name in ("solve", "verify"):
        s = sub.add_parser(name)
        s.add_argument("file")
    e = sub.add_parser("enum")
    e.add_argument("file")
    e.add_argument("--method", choices=(*bench.STREAMS, "exhaustive"), default="berge")
    e.add_argument("--timeout", type=float, default=bench.LIMITS["timeout_s"])
    e.add_argument("--cap", type=int, default=bench.LIMITS["cap"])
    s = sub.add_parser("bench")
    s.add_argument("--seed", type=int, required=True)
    s.add_argument("--timeout", type=float, default=bench.LIMITS["timeout_s"],
                   help="calibration only; the main suite uses bench.LIMITS")
    s.add_argument("--no-variants", action="store_true", help="skip sensitivity reruns")
    s = sub.add_parser("report")
    s.add_argument("file")
    a = ap.parse_args(argv)

    if a.cmd == "acquire":
        from .acquire import fetch_all
        print(json.dumps(fetch_all(), indent=1))
    elif a.cmd == "audit":
        m = json.loads((bench.ROOT / "data/manifest.json").read_text())["QBL.xlsx"]
        A = audit.audit(qbl.read(bench.RAW))
        A["snapshot_age_days"] = qbl.age_days(m["retrieved_utc"], today())
        A["stale"] = qbl.stale(m["retrieved_utc"], today())
        print(json.dumps(A, indent=1))
    elif a.cmd == "build":
        S, _Q, _issues, _D, M = audit.matrix(qbl.read(bench.RAW), a.variant)
        missing = [p for p in a.products if p not in M]
        if missing:
            raise SystemExit(f"not a {a.variant} product: {missing}")
        E = [sorted(M[p][1]) for p in a.products]
        print(json.dumps({"V": sorted(set().union(*E)), "P": a.products, "E": E, "k": a.k,
                          "F": [f.split(",") for f in a.fail], "variant": a.variant,
                          "data_sha256": bench.data_sha256(),
                          "names": {s: S[s]["name"] for e in E for s in e}}, indent=1))
    elif a.cmd == "solve":
        d, I = load(a.file)
        status, T = solve.milp(I)
        print(json.dumps({"status": status, "panel": sorted(T) if T else T,
                          "witnesses": certify.witnesses(I, T) if T else None,
                          "infeasibility": certify.infeasibility(I),
                          "essential": certify.essential(I)}, indent=1, default=list))
    elif a.cmd == "enum":
        d, I = load(a.file)
        if a.method == "exhaustive":
            C = solve.exhaustive(I)
            r = {"method": "exhaustive", "status": "complete", "count": len(C),
                 "stored": len(C)}
        else:
            C, r = bench.run(a.method, I, a.timeout, a.cap, bench.LIMITS["K"])
        print(json.dumps(dict(d, run=r, panels=[sorted(T) for T in C]), indent=1))
    elif a.cmd == "verify":
        d, I = load(a.file)
        C = [frozenset(T) for T in d["panels"]]
        r = d["run"]
        out = {"panels": len(C), "all_feasible_and_minimal": all(certify.minimal(I, T)
                                                                  for T in C),
               "run": r["status"]}
        if r["status"] == "complete" and r["count"] == len(C):   # whole family stored
            out["milp_exhaustion"] = certify.complete(I, C)[0]
        print(json.dumps(out, indent=1))
    elif a.cmd == "bench":
        L = dict(bench.LIMITS, timeout_s=a.timeout)
        bench.suite(a.seed, L, out=bench.ROOT / f"results/suite-seed{a.seed}.json",
                    variants=not a.no_variants)
    elif a.cmd == "report":
        from .report import write
        write(a.file)
