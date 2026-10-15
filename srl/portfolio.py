"""Certified public cases. The browser selects recorded results and runs no solver."""
import hashlib
import json
import subprocess
from collections import Counter
from importlib.metadata import distribution
from pathlib import Path
from statistics import median

from . import audit, bench, certify, contracts, qbl, solve
from .resources import data_path

CASES = (
    ("canned", "Three canned products", "Many alternatives, one precise coverage question.",
     ("beans pinto can", "beans vegetarian can", "tomato sauce can")),
    ("poultry", "Three poultry products", "Overlapping qualifications permit a compact panel.",
     ("chicken leg qtr frz bag-4/10 lb", "chicken oven roasted frz 8 pc ctn-30 lb",
      "chicken small chilled-bulk")),
    ("scarce", "A scarce-product basket", "A single-source product blocks backup coverage.",
     ("lentils dry bag", "pears halves ex lt can", "sweet potato fresh ctn 40")),
)


def shortlist(C, T, n=6):
    """Keep an optimum, representatives by size, then maximise distance to panels kept.
    Distance is symmetric difference; this is an illustrative shortlist, not a ranking."""
    if T is None:
        return []
    C = sorted(C, key=lambda S: (len(S), sorted(S)))
    out = [T]
    for r in sorted({len(S) for S in C}):
        S = next(S for S in C if len(S) == r)
        if S not in out and len(out) < n:
            out.append(S)
    while len(out) < min(n, len(C)):
        out.append(max((S for S in C if S not in out),
                       key=lambda S: (min(len(S ^ U) for U in out), -len(S), sorted(S))))
    return out


def coverage(I, T, F):
    """Product coverage of a fixed panel under one explicit outage, read from E_p."""
    E = [sorted((set(T) - set(F)) & Ep) for Ep in I.E]
    return {"suppliers": E, "uncovered": [p for p, e in zip(I.P, E) if not e],
            "covered": all(E)}


def analyse(I):
    """A complete small case: optimum, certified family and matrix MILP exhaustion."""
    C, run = bench.run("berge", I, timeout=10, cap=10_000, K=10)
    if run["status"] != "complete" or run["count"] != len(C):
        raise ValueError(f"public case was not fully stored: {run['status']}")
    if not all(certify.minimal(I, T) for T in C):
        raise ValueError("public case contains an uncertified panel")
    exhaustion, _ = certify.complete(I, C)
    status, T = solve.milp(I)
    if exhaustion != "complete" or (status == "optimal") != bool(C):
        raise ValueError("public case failed independent exhaustion or optimum check")
    if T is not None and (not certify.minimal(I, T) or len(T) != min(map(len, C))):
        raise ValueError("public case minimum disagrees with its certified family")
    if status not in ("optimal", "infeasible"):
        raise ValueError(f"public case minimum unresolved: {status}")
    C = sorted(C, key=lambda S: (len(S), sorted(S)))
    return {"status": status, "minimum_size": len(T) if T is not None else None,
            "count": len(C), "sizes": dict(sorted(Counter(map(len, C)).items())),
            "essential": certify.essential(I), "infeasibility": certify.infeasibility(I),
            "checks": {"all_minimal": True, "milp_exhaustion": exhaustion,
                       "minimum_matches_family": True},
            "family": dict(contracts.document(I), kind="enumeration", run=run,
                           panels=[sorted(S) for S in C]),
            "panels": [{"members": sorted(S), "witnesses": certify.witnesses(I, S)}
                       for S in shortlist(C, T)]}


def case(M, names, id, title, note, P):
    """Both uniform failure budgets and fixed-panel outage checks for a real basket."""
    I = audit.basket(M, P)
    modes = {str(k): analyse(I.with_k(k)) for k in (0, 1)}
    pair = (modes["1"]["panels"][0]["members"][:2] if modes["1"]["panels"] else list(I.V[:2]))
    outages = [{"id": "none", "label": "No outage", "failed": []}]
    outages += [{"id": v, "label": names[v], "failed": [v]} for v in I.V]
    outages += [{"id": "pair", "label": "Two-supplier stress test", "failed": pair}]
    for m in modes.values():
        for panel in m["panels"]:
            panel["outages"] = {o["id"]: coverage(I, panel["members"], o["failed"])
                                for o in outages}
    return {"id": id, "title": title, "note": note, "products": list(I.P),
            "suppliers": [{"id": v, "name": names[v],
                           "qualifications": [v in E for E in I.E]} for v in I.V],
            "eligible": len(I.V), "product_sizes": [len(E) for E in I.E],
            "outages": outages, "modes": modes}


def evidence(path):
    """Summaries of the original seed-1 measurements, retaining their revision and limits."""
    R = json.loads(Path(path).read_bytes())
    X = R["instances"]
    primary = [x for x in X if x["rule"] in ("uniform", "overlap") and x["k"] == 1]
    routes = []
    for method in bench.STREAMS:
        runs = [r for x in X for r in x["runs"] if r["method"] == method]
        first = [r["t_first"] for r in runs if r["t_first"] is not None]
        routes.append({"method": method, "complete": sum(r["status"] == "complete" for r in runs),
                       "runs": len(runs), "first_median_s": median(first),
                       "first_n": len(first)})
    return {"instances": len(X), "resilient": sum(x["min"]["status"] == "optimal" for x in primary),
            "primary": len(primary), "routes": routes, "provenance": R["provenance"]}


def build(destination, root):
    """Write public cases only after their certificates pass. No main-suite rerun."""
    root, destination = Path(root), Path(destination)
    S, _, issues, _, M = audit.matrix(qbl.read(data_path("raw/QBL.xlsx")))
    installed = json.loads(distribution("fka").read_text("direct_url.json"))
    if installed.get("vcs_info", {}).get("commit_id") != bench.THESIS:
        raise ValueError("the installed thesis revision does not match the pin")
    git = lambda *a: subprocess.check_output(["git", *a], cwd=root, text=True).strip()
    sources = sorted((root / "srl").glob("*.py")) + [root / "analysis/build_site.py"]
    fingerprint = hashlib.sha256(b"".join(p.name.encode() + p.read_bytes().replace(b"\r\n", b"\n")
                                        for p in sources))
    payload = {"schema_version": "1.0", "kind": "portfolio", "provenance": {
        **contracts.provenance(), "code_rev": git("rev-parse", "HEAD"),
        "code_sha256": fingerprint.hexdigest(),
        "dirty": bool(git("status", "--porcelain", "--", "srl", "analysis", "pyproject.toml")),
        "identity_candidates": sum(k == "identity_candidate" for _, k, _ in issues),
        "snapshot": json.loads(data_path("manifest.json").read_bytes())["QBL.xlsx"]},
        "cases": [case(M, {v: s["name"] for v, s in S.items()}, *c) for c in CASES],
        "evidence": evidence(root / "results/suite-seed1.json")}
    if payload["provenance"]["data_sha256"] != payload["provenance"]["snapshot"]["sha256"]:
        raise ValueError("public case data does not match the frozen manifest")
    destination.mkdir(parents=True, exist_ok=True)
    text = json.dumps(payload, separators=(",", ":"), default=list)
    (destination / "data.js").write_text("window.DATA=" + text + ";\n", encoding="utf-8")
    (destination / "cases.json").write_text(text + "\n", encoding="utf-8")
    schema = Path(__file__).parent / "schemas/document-1.0.json"
    (destination / schema.name).write_bytes(schema.read_bytes())
    return payload
