"""USDA AMS Qualified Bidders List: workbook -> rows -> suppliers and qualifications.

Harmless normalisation (whitespace, case, spacing round " - ", ZIP zero-padding) is done
here. Semantic interpretation (granularity, merged products) is in srl/catalog.py and
data/catalog/mappings.csv. See docs/DATA.md.
"""
import re
import zipfile
from datetime import date
from xml.etree import ElementTree as ET

from .instance import Instance

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
SUFFIX = {"inc", "llc", "llp", "lp", "co", "company", "corp", "corporation", "ltd", "the",
          "incorporated", "and", "of"}
STALE_DAYS = 90


def read(path):
    """Rows of the first sheet as {header: text, 'row': sheet row}, stdlib only."""
    z = zipfile.ZipFile(path)
    shared = [("".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t")))
              for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS)]
    sheet = ET.fromstring(z.read("xl/worksheets/sheet1.xml"))
    table = []
    for r in sheet.iter(f"{{{NS['m']}}}row"):
        cells = {}
        for c in r.findall("m:c", NS):
            col = re.match(r"[A-Z]+", c.get("r")).group()
            v = c.find("m:v", NS)
            if c.get("t") == "s":
                cells[col] = shared[int(v.text)]
            elif c.get("t") == "inlineStr":
                cells[col] = "".join(t.text or "" for t in c.iter(f"{{{NS['m']}}}t"))
            elif v is not None:
                cells[col] = v.text.removesuffix(".0")
        table.append((int(r.get("r")), cells))
    (_, head), body = table[0], table[1:]
    return [dict({head[c]: cells.get(c, "") for c in head}, row=n)
            for n, cells in body if any(str(x).strip() for x in cells.values())]


def clean(s):
    return re.sub(r"\s+", " ", str(s or "")).strip().casefold()


def token(s):
    """Product token: whitespace collapsed, casefolded, ' - ' spacing unified."""
    return re.sub(r"\s+-\s*|\s*-\s+", " - ", clean(s))


def zip5(s):
    d = str(s or "").strip().split("-")[0]
    return d.zfill(5) if len(d) <= 5 else d.zfill(9)[:5]


def core(name):
    """Name with punctuation and legal suffixes removed: a review key, never a merge key."""
    return " ".join(w for w in re.sub(r"[^\w\s]", " ", clean(name)).split() if w not in SUFFIX)


def records(rows):
    """Suppliers keyed by identity (name, address, ZIP5), qualifications with source rows.

    OUTPUT  S ...... id -> supplier fields; id = 'r' + first sheet row
            Q ...... [(id, token, row)]
            issues . [(row, kind, detail)]: missing_name, missing_product, same_identity
                     (merged: equal key), identity_candidate (equal core name, kept apart)
    """
    S, Q, issues, by_key, by_core = {}, [], [], {}, {}
    for r in rows:
        n, name = r["row"], clean(r["Vendor Name"])
        toks = [token(t) for t in re.split(r"[\r\n]+", str(r["Product"] or "")) if token(t)]
        if not name or not toks:
            issues.append((n, "missing_name" if not name else "missing_product", ""))
            continue
        key = (name, clean(r["Billing Address"]), zip5(r["ZIP Code"]))
        if key in by_key:
            sid = by_key[key]
            S[sid]["rows"].append(n)
            issues.append((n, "same_identity", sid))
        else:
            sid = by_key[key] = f"r{n:04d}"
            if core(name) in by_core:
                issues.append((n, "identity_candidate", by_core[core(name)]))
            by_core.setdefault(core(name), sid)
            S[sid] = {"name": " ".join(str(r["Vendor Name"]).split()), "address": key[1],
                      "city": clean(r["City"]), "state": clean(r["State"]).upper(),
                      "zip5": key[2], "size": clean(r["Size"]), "rows": [n]}
        Q += [(sid, t, n) for t in toks]
    return S, Q, issues


def instance(S, Q, P, k=0, F=(), product=lambda t: (t,)):
    """Basket P with every eligible supplier: E_p = {s : (s, t) ∈ Q, p ∈ product(t)}."""
    E = [{s for s, t, _ in Q if p in product(t)} for p in P]
    V = tuple(sorted(set().union(*E)))
    return Instance(V=V, P=tuple(P), E=E, k=k, F=F)


def age_days(retrieved_utc, today):
    return (date.fromisoformat(today) - date.fromisoformat(retrieved_utc[:10])).days


def stale(retrieved_utc, today):
    return age_days(retrieved_utc, today) > STALE_DAYS
