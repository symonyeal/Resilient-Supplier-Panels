"""The public explorer retains the complete measured cohort and honest partial counts."""
import json
from pathlib import Path

import pytest

from srl import portfolio

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results/suite-seed1.json"


def test_outcome_export_retains_every_record_and_run():
    R = json.loads(RESULTS.read_bytes())
    E = portfolio.evidence(RESULTS)
    assert len(E["records"]) == len(R["instances"]) == 88
    for a, b in zip(E["records"], R["instances"]):
        for key in ("class", "rule", "size", "k", "P", "min", "essential",
                    "infeasibility", "runs", "count_exact"):
            assert a[key] == b[key]
        assert a["count_lower_bound"] == max(r["count"] for r in b["runs"])
        assert "H_edges_before_reduction" not in a
    partial = [x for x in E["records"] if x["count_exact"] is None]
    assert len(partial) == 4 and all(x["count_lower_bound"] > 0 for x in partial)
    assert E["checks"]["matrix_records"] == 88


def test_invalid_saved_minimum_cannot_enter_the_explorer(tmp_path):
    R = json.loads(RESULTS.read_bytes())
    R["instances"][0]["min"]["panel"] = []
    path = tmp_path / "invalid.json"
    path.write_text(json.dumps(R))
    with pytest.raises(ValueError, match="saved minimum"):
        portfolio.evidence(path)


def test_published_outcomes_reproduce_the_saved_source():
    D = json.loads((ROOT / "site/cases.json").read_bytes())
    E = portfolio.evidence(RESULTS)
    assert D["evidence"]["records"] == E["records"]
    assert D["evidence"]["provenance"] == E["provenance"]
