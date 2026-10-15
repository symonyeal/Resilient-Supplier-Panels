"""Version 1.0 exchange documents; legacy instance JSON remains readable."""
import csv
import hashlib
import json
from pathlib import Path

from . import THESIS_REV, __version__
from .instance import Instance
from .resources import data_path

SCHEMA_VERSION = "1.0"


def load(path):
    """Read an instance or enumeration, checking its version and matrix invariants."""
    d = json.loads(Path(path).read_bytes())
    if not isinstance(d, dict):
        raise TypeError("expected an instance or enumeration JSON object")
    if d.get("schema_version", SCHEMA_VERSION) != SCHEMA_VERSION:
        raise ValueError(f"unsupported schema version: {d['schema_version']}")
    if d.get("kind", "instance") not in ("instance", "enumeration"):
        raise ValueError("expected an instance or enumeration document")
    return d, Instance(V=d["V"], P=d["P"], E=d["E"], k=d["k"], F=d.get("F", []))


def provenance():
    """Content hashes and software versions used to interpret the matrix."""
    return {"data_sha256": hashlib.sha256(data_path("raw/QBL.xlsx").read_bytes()).hexdigest(),
            "mappings_sha256": hashlib.sha256(data_path("catalog/mappings.csv").read_bytes())
            .hexdigest(), "software_version": __version__, "thesis_rev": THESIS_REV}


def document(I, **extra):
    """A versioned instance with ordered labels and sorted qualification/failure sets."""
    return {"schema_version": SCHEMA_VERSION, "kind": "instance", "V": list(I.V),
            "P": list(I.P), "E": [sorted(E) for E in I.E], "k": I.k,
            "F": [sorted(F) for F in I.F], **extra}


def basket_csv(path):
    """Read distinct, nonempty exact product tokens from a CSV column named product."""
    with open(path, encoding="utf-8-sig", newline="") as f:
        rows = csv.DictReader(f)
        if rows.fieldnames != ["product"]:
            raise ValueError("basket CSV must have exactly one column named product")
        R = list(rows)
        if any(None in r or r["product"] is None for r in R):
            raise ValueError("each basket CSV row must have exactly one product")
        P = [r["product"].strip() for r in R]
    if not P or any(not p for p in P) or len(set(P)) != len(P):
        raise ValueError("basket CSV must contain distinct, nonempty products")
    return P


def write(d, path=None):
    """Write JSON explicitly as UTF-8, or emit it to standard output."""
    text = json.dumps(d, indent=1, default=list) + "\n"
    if path:
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(text, encoding="utf-8")
    else:
        print(text, end="")
