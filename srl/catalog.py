"""Product dictionary: token -> (class, strict products, merged products, justification).

Rules (docs/DATA.md), applied in order; data/catalog/mappings.csv overrides them:
  material     '^dddddd - '         a numbered USDA material
  naics        '^dddddd x'          NAICS-prefixed commodity code (international programmes)
  program      '^code - name'       AMS programme code; merged variant reads it as its code
  description  contains a pack/form word (can, pkg, ctn, frz, ...)
  commodity    anything else: a named commodity without specification
Strict product = the token itself unless a mapping says otherwise.
"""
import csv
import re
from pathlib import Path

MAPPINGS = Path(__file__).resolve().parents[1] / "data/catalog/mappings.csv"
PACK = re.compile(r"\b(can|cans|pkg|ctn|carton|bag|bags|box|pouch|bulk|lb|lbs|oz|dozen|jug|"
                  r"bottle|frz|iqf|chilled|jar|tote|drum|pail|case)\b")
CLASSES = ("material", "naics", "program", "description", "commodity", "broad")


def rule(t):
    if re.match(r"^\d{6} - ", t):
        return "material"
    if re.match(r"^\d{6} \S", t):
        return "naics"
    if m := re.match(r"^([a-z]{1,5}\d?) - ", t):
        return "program", m.group(1)
    return "description" if PACK.search(t) else "commodity"


def load(tokens, path=MAPPINGS):
    """Dictionary entry for every token: {t: (class, strict, merged, justification)}."""
    with open(path, encoding="utf-8") as f:
        over = {r["token"]: r for r in csv.DictReader(f)}
    D = {}
    for t in tokens:
        if t in over:
            r = over[t]
            D[t] = (r["class"], tuple(r["strict"].split("|")), tuple(r["merged"].split("|")),
                    "mapping: " + r["justification"])
        elif isinstance(c := rule(t), tuple):
            D[t] = (c[0], (t,), (c[1],), "rule: programme code " + c[1].upper())
        else:
            D[t] = (c, (t,), (t,), "rule: " + c)
    return D


def merged_of(D):
    """Strict product -> merged product, read off the parallel lists in D."""
    M = {}
    for _, s, m, _ in D.values():
        for p, q in zip(s, m):
            assert M.setdefault(p, q) == q, (p, M[p], q)
    return M
