"""python -m srl {acquire,audit,build,solve,enum,verify,bench,report}. No research logic."""
import argparse
import json
import os
from datetime import UTC, datetime
from pathlib import Path

from . import __version__, audit, bench, certify, contracts, qbl, solve
from .contracts import load
from .instance import Instance
from .resources import data_path


def today():
    return datetime.now(UTC).date().isoformat()


def main(argv=None):
    ap = argparse.ArgumentParser(prog="srl")
    ap.add_argument("--version", action="version", version=f"srl {__version__}")
    ap.add_argument("--data-dir", type=Path, help="override the packaged data directory")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("acquire", help="download the sources to data/fetched (network)")
    sub.add_parser("audit", help="Stage 0 audit of the frozen snapshot")
    sub.add_parser("schema", help="JSON Schema for version 1.0 exchange documents")
    b = sub.add_parser("build", help="instance JSON for a basket")
    b.add_argument("products", nargs="*")
    b.add_argument("--basket", type=Path, help="CSV with one product column")
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
    for name in ("audit", "schema", "build", "solve", "enum", "verify", "report"):
        sub.choices[name].add_argument("--output", "-o", type=Path)
    a = ap.parse_args(argv)
    if a.data_dir:
        os.environ["SRL_DATA_DIR"] = str(a.data_dir.resolve())
    if a.cmd in ("solve", "enum", "verify"):
        try:
            d, I = load(a.file)
        except (ValueError, KeyError, TypeError, OSError) as e:
            ap.error(str(e))

    if a.cmd == "acquire":
        from .acquire import fetch_all
        print(json.dumps(fetch_all(), indent=1))
    elif a.cmd == "audit":
        m = json.loads(data_path("manifest.json").read_bytes())["QBL.xlsx"]
        A = audit.audit(qbl.read(data_path("raw/QBL.xlsx")))
        A["snapshot_age_days"] = qbl.age_days(m["retrieved_utc"], today())
        A["stale"] = qbl.stale(m["retrieved_utc"], today())
        contracts.write(A, a.output)
    elif a.cmd == "schema":
        contracts.write(json.loads((Path(__file__).parent / "schemas/document-1.0.json")
                                   .read_bytes()), a.output)
    elif a.cmd == "build":
        if a.basket:
            if a.products:
                ap.error("use either product arguments or --basket")
            try:
                a.products = contracts.basket_csv(a.basket)
            except (ValueError, OSError) as e:
                ap.error(str(e))
        if not a.products:
            ap.error("provide products or --basket")
        S, _Q, _issues, _D, M = audit.matrix(qbl.read(data_path("raw/QBL.xlsx")), a.variant)
        missing = [p for p in a.products if p not in M]
        if missing:
            raise SystemExit(f"not a {a.variant} product: {missing}")
        E = [sorted(M[p][1]) for p in a.products]
        V, F = sorted(set().union(*E)), [f.split(",") for f in a.fail]
        try:
            I = Instance(V=V, P=a.products, E=E, k=a.k, F=F)
        except ValueError as e:
            ap.error(str(e))
        contracts.write(contracts.document(I, variant=a.variant, **contracts.provenance(),
                        names={s: S[s]["name"] for e in E for s in e}), a.output)
    elif a.cmd == "solve":
        status, T = solve.milp(I)
        contracts.write({"schema_version": contracts.SCHEMA_VERSION, "kind": "solution",
                          "software_version": __version__,
                          "instance": dict(d, **contracts.document(I)),
                          "status": status, "panel": sorted(T) if T is not None else None,
                          "witnesses": certify.witnesses(I, T) if T is not None else None,
                          "infeasibility": certify.infeasibility(I),
                          "essential": certify.essential(I)}, a.output)
    elif a.cmd == "enum":
        if a.method == "exhaustive":
            C = solve.exhaustive(I)
            r = {"method": "exhaustive", "status": "complete", "count": len(C),
                 "stored": len(C)}
        else:
            C, r = bench.run(a.method, I, a.timeout, a.cap, bench.LIMITS["K"])
        out = dict(d, **contracts.document(I))
        out.update(kind="enumeration", run=r, panels=[sorted(T) for T in C])
        contracts.write(out, a.output)
    elif a.cmd == "verify":
        C = [frozenset(T) for T in d["panels"]]
        r = d["run"]
        out = {"panels": len(C), "all_feasible_and_minimal": all(certify.minimal(I, T)
                                                                  for T in C),
               "distinct_panels": len(set(C)) == len(C),
               "run": r["status"]}
        ok = out["all_feasible_and_minimal"] and out["distinct_panels"]
        if ok and r["status"] == "complete" and r["count"] == len(C):
            out["milp_exhaustion"] = certify.complete(I, C)[0]
            ok = out["milp_exhaustion"] == "complete"
        contracts.write(out, a.output)
        if not ok:
            raise SystemExit(1)
    elif a.cmd == "bench":
        L = dict(bench.LIMITS, timeout_s=a.timeout)
        bench.suite(a.seed, L, out=Path(f"results/suite-seed{a.seed}.json"),
                    variants=not a.no_variants)
    elif a.cmd == "report":
        from .report import write
        write(a.file, a.output)
