"""Saved seed-1 claims agree with the frozen matrix and rebuild the published report."""
import json
from pathlib import Path

from srl import audit, bench, certify, qbl, report, solve

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results/suite-seed1.json"


def test_saved_minima_and_matrix_certificates():
    R = json.loads(RESULTS.read_bytes())
    M = audit.matrix(qbl.read(bench.RAW))[4]
    for r in R["instances"]:
        I = audit.basket(M, r["P"], r["k"])
        status, T = solve.milp(I)
        assert status == r["min"]["status"]
        assert (len(T) if T is not None else None) == r["min"]["size"]
        if T is not None:
            assert certify.minimal(I, T) and certify.minimal(I, r["min"]["panel"])
        assert json.loads(json.dumps(certify.infeasibility(I))) == r["infeasibility"]
        assert json.loads(json.dumps(certify.essential(I))) == r["essential"]


def test_saved_report_rebuilds_without_changes(tmp_path):
    (tmp_path / "results").mkdir()
    (tmp_path / "docs/findings").mkdir(parents=True)
    path = tmp_path / "results/suite-seed1.json"
    path.write_bytes(RESULTS.read_bytes())
    assert report.write(path).read_text(encoding="utf-8") == (
        ROOT / "docs/findings/results-seed1.md").read_text(encoding="utf-8")
