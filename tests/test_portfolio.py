"""Published case counts, fixed-panel outages and source hashes are independently checked."""
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

from srl import certify
from srl.instance import Instance

ROOT = Path(__file__).resolve().parents[1]
D = json.loads((ROOT / "site/cases.json").read_bytes())


def test_generated_cases_have_matrix_certificates_and_exhaustion():
    schema = json.loads((ROOT / "srl/schemas/document-1.0.json").read_bytes())
    for c in D["cases"]:
        for k, mode in c["modes"].items():
            d = mode["family"]
            Draft202012Validator(schema).validate(d)
            I = Instance(V=d["V"], P=d["P"], E=d["E"], k=int(k))
            C = {frozenset(T) for T in d["panels"]}
            assert len(C) == mode["count"] == d["run"]["count"]
            assert all(certify.minimal(I, T) for T in C)
            assert certify.complete(I, C) == ("complete", None)
            assert (min(map(len, C)) if C else None) == mode["minimum_size"]
            assert json.loads(json.dumps(certify.essential(I))) == mode["essential"]
            for p in mode["panels"]:
                assert frozenset(p["members"]) in C
                for o in c["outages"]:
                    J = Instance(V=I.V, P=I.P, E=I.E, F=(o["failed"],))
                    result = p["outages"][o["id"]]
                    assert certify.feasible(J, p["members"]) == result["covered"]
                    for Ep, got in zip(I.E, result["suppliers"]):
                        assert set(got) == (set(p["members"]) - set(o["failed"])) & Ep


def test_generated_data_matches_files_and_source():
    p = D["provenance"]
    for key, name in (("data_sha256", "raw/QBL.xlsx"),
                      ("mappings_sha256", "catalog/mappings.csv")):
        assert hashlib.sha256((ROOT / "data" / name).read_bytes()).hexdigest() == p[key]
    sources = sorted((ROOT / "srl").glob("*.py")) + [ROOT / "analysis/build_site.py"]
    source = b"".join(f.name.encode() + f.read_bytes().replace(b"\r\n", b"\n") for f in sources)
    assert hashlib.sha256(source).hexdigest() == p["code_sha256"]
    text = (ROOT / "site/data.js").read_text(encoding="utf-8")
    assert json.loads(text.removeprefix("window.DATA=").strip().removesuffix(";")) == D
