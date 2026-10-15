"""Seam 5: benchmark run -> truthful status, and result -> report.

Fixture: three products with 30 disjoint suppliers each, k = 1. Then
|Tr| = C(30, 2)^3 = 435^3, which no route finishes in a second.
"""
import pytest

from srl import bench, certify, report
from srl.instance import Instance

BIG = Instance(V=tuple(f"s{i}" for i in range(90)), P=("a", "b", "c"),
               E=[{f"s{i}" for i in range(30 * j, 30 * j + 30)} for j in range(3)], k=1)
SMALL = Instance(V=tuple("abcd"), P=("p1", "p2", "p3"), E=(set("ab"), set("bc"), set("cd")))


@pytest.mark.slow
def test_timeout_is_partial_and_every_returned_panel_is_certified():
    panels, r = bench.run("fkb", BIG, timeout=1.0, cap=10**6, K=10)
    assert r["status"] == "timeout" and r["t_total"] is None
    assert all(certify.minimal(BIG, T) for T in panels)


@pytest.mark.slow
def test_cap_stops_a_stream_and_berge_counts_exactly():
    _, r = bench.run("milp", BIG, timeout=30.0, cap=3, K=2)
    assert (r["status"], r["count"], r["stored"]) == ("cap", 4, 3)
    _, r = bench.run("berge", SMALL, timeout=30.0, cap=2, K=1)
    assert (r["status"], r["count"], r["stored"]) == ("complete", 3, 2)


def test_report_never_states_a_partial_count_as_exact():
    assert report.count({"status": "complete", "count": 3, "stored": 3}) == "3"
    assert report.count({"status": "complete", "count": 12000, "stored": 10000}) == "12000"
    assert report.count({"status": "timeout", "count": 41, "stored": 41}) == "≥41 (timeout)"
    assert report.count({"status": "cap", "count": 10001, "stored": 10000}) == "≥10001 (cap)"


def test_essential_claims_only_from_complete_or_certified_sources():
    rec = {"essential": {"a": ["p", []]}, "count_exact": None}
    assert report.essential(rec) == "1 (matrix witness)"
    assert report.essential({"essential": None, "count_exact": None}) == "— (infeasible)"
