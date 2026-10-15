"""Findings tables, rebuilt from saved result JSON only (never from a live run)."""
import json
from pathlib import Path
from statistics import median


def count(r):
    """A run's output count; a stopped run gives a lower bound, never a count."""
    return str(r["count"]) if r["status"] == "complete" else f"≥{r['count']} ({r['status']})"


def essential(rec):
    """Essential suppliers come from the matrix witness (V \\ {v} infeasible), so they need
    no complete enumeration."""
    if rec["essential"] is None:
        return "— (infeasible)"
    return f"{len(rec['essential'])} (matrix witness)"


def sec(t):
    return "—" if t is None else f"{t:.3g}"


def exhibit(title, head, rows, note=None):
    """Markdown table titled by its claim (skill exhibit)."""
    return [f"### {title}", "", *([note, ""] if note else []), "| " + " | ".join(head) + " |",
            "|" + "---|" * len(head), *rows, ""]


def basket_id(r):
    return f"{r['class']}/{r['rule']}/{r['size']}"


def tables(res):
    X = res["instances"]
    by = {(basket_id(r), r["k"]): r for r in X}
    ids = sorted({basket_id(r) for r in X}, key=lambda s: (s.split("/")[0], s.split("/")[1],
                                                        int(s.split("/")[2])))
    primary = [b for b in ids if b.split("/")[1] in ("uniform", "overlap")]
    out = []

    # Exhibit 1: minimum panel, k = 0 against k = 1
    rows, inc = [], []
    for b in ids:
        r0, r1 = by[(b, 0)], by[(b, 1)]
        m0, m1 = r0["min"]["size"], r1["min"]["size"]
        if m0 is not None and m1 is not None and b in primary:
            inc.append(m1 - m0)
        w = r1["infeasibility"]
        rows.append(f"| {b} | {r0['stats']['V']} | {sec(m0)} | "
                    f"{m1 if m1 is not None else 'infeasible'} | "
                    f"{'' if w is None else w[0]} |")
    feas = sum(by[(b, 1)]["min"]["size"] is not None for b in primary)
    head = (f"k = 1 is feasible for {feas} of {len(primary)} primary baskets; where feasible "
            f"it adds a median of {median(inc) if inc else '—'} suppliers "
            f"(range {min(inc) if inc else '—'}–{max(inc) if inc else '—'})")
    out += exhibit(head, ["basket (class/rule/size)", "eligible suppliers", "min panel k=0",
                          "min panel k=1", "k=1 blocked by (single-supplier product)"], rows)

    # Exhibit 2: alternatives and essential suppliers
    rows, many = [], 0
    for b in ids:
        for k in (0, 1):
            r = by[(b, k)]
            if r["min"]["size"] is None:
                continue
            c = r.get("count_exact")
            many += c is not None and c >= 2 and b in primary
            rows.append(f"| {b} | {k} | {r['min']['size']} | "
                        f"{c if c is not None else min_count(r)} | {r.get('orbits', '—')} | "
                        f"{essential(r)} |")
    out += exhibit(f"{many} feasible primary basket/k cases have at least 2 minimal panels "
                   "(complete count)", ["basket", "k", "min size", "minimal panels",
                                         "up to signature symmetry", "essential suppliers"], rows)

    # Exhibit 3: routes
    rows, stat = [], {}
    for b in ids:
        for k in (0, 1):
            r = by[(b, k)]
            for run in r["runs"]:
                stat.setdefault(run["method"], []).append(run["status"] == "complete")
            if r["min"]["size"] is None:
                continue
            cells = " | ".join(f"{count(u)} · {sec(u['t_first'])} / {sec(u['t_K'])} / "
                               f"{sec(u['t_total'])}" for u in r["runs"])
            rows.append(f"| {b} | {k} | {cells} |")
    done = ", ".join(f"{m} {sum(v)}/{len(v)}" for m, v in stat.items())
    out += exhibit(f"Complete runs within the limit: {done}", ["basket", "k", *stat], rows,
                   "Each cell: count · time to first / to K=10 / total (s). "
                   "'≥n (timeout)' is censored.")

    # Exhibit 4: thesis tree against memoised DAG
    rows, ratios = [], []
    for b in ids:
        for k in (0, 1):
            t = by[(b, k)].get("thesis")
            if not t or t.get("status") != "dual":
                continue
            ratios.append(t["tree_nodes"] / t["dag_nodes"])
            rows.append(f"| {b} | {k} | {t['tree_nodes']} | {t['dag_nodes']} | "
                        f"{t['tree_nodes'] / t['dag_nodes']:.2f} | {sec(t['t_tree'])} | "
                        f"{sec(t['t_memo'])} |")
    if ratios:
        out += exhibit(f"FK-A memoisation: median tree/DAG node ratio {median(ratios):.2f} "
                       f"over {len(ratios)} certified instances",
                       ["basket", "k", "FK-A tree nodes", "distinct subproblems", "ratio",
                        "t tree (s)", "t memo (s)"], rows)

    # Exhibit 5: sensitivity
    rows, flips = [], 0
    for b in ids:
        for k in (0, 1):
            r = by[(b, k)]
            s = r.get("sensitivity", {})
            if not s:
                continue
            base = r["min"]["size"]
            cells = []
            for name in ("merged", "identity_candidates_merged"):
                v = s[name]
                flips += (v["feasible"] != (base is not None)) and b in primary
                cells.append(f"{v['min'] if v['feasible'] else 'infeasible'}")
            rows.append(f"| {b} | {k} | {base if base is not None else 'infeasible'} | "
                        + " | ".join(cells) + " |")
    out += exhibit(f"Feasibility verdict changes in {flips} primary cases under the two "
                   "sensitivity variants", ["basket", "k", "min (strict)",
                                            "min (merged products)",
                                            "min (identity candidates merged)"], rows)
    return "\n".join(out)


def min_count(r):
    return max((count(u) for u in r["runs"]), key=lambda s: int(s.lstrip("≥").split()[0]))


def write(path):
    res = json.loads(Path(path).read_text())
    p = res["provenance"]
    md = (f"# Results: seed {p['seed']}\n\nGenerated by `python -m srl report {Path(path).name}`"
          f" from saved results only.\n\nSource: QBL snapshot `{p['data_sha256'][:12]}…`; "
          f"code `{p['code_rev'][:7]}`{' (dirty)' if p['dirty'] else ''}; thesis "
          f"`{p['thesis_rev'][:7]}`; limits {p['limits']['timeout_s']} s, "
          f"cap {p['limits']['cap']}; {p['cpu']}, {p['os']}, Python {p['python']}. "
          f"Suite wall time {res['t_suite']:.0f} s. Assumptions: strict mapping, all eligible "
          "suppliers, k-failure model; qualification only (no capacity, price or lead time)."
          f"\n\n{tables(res)}")
    out = Path(path).parents[1] / f"docs/findings/results-seed{p['seed']}.md"
    out.write_text(md, encoding="utf-8")
    return out
