"""Unresolved solves give no completeness or infeasibility certificate."""
import json
from copy import deepcopy
from pathlib import Path
from types import SimpleNamespace

import pytest

from srl import bench, certify, cli, report, solve, verdicts
from srl.instance import Instance

I = Instance(V=tuple("abc"), P=("p",), E=(set("abc"),))
RESULTS = Path(__file__).resolve().parents[1] / "results/suite-seed1.json"
WORKER = bench._child


def limited_worker(method, I, conn):
    results = iter([("optimal", frozenset("a")), ("highs status 1", None)])
    solve.milp = lambda *a: next(results)
    WORKER(method, I, conn)


@pytest.mark.slow
def test_process_run_preserves_unresolved_status(monkeypatch):
    monkeypatch.setattr(bench, "_child", limited_worker)
    C, r = bench.run("milp", I, timeout=5, cap=10, K=1)
    assert C == [frozenset("a")]
    assert (r["status"], r["count"], r["t_total"]) == ("highs status 1", 1, None)


@pytest.mark.parametrize("status", [1, 3, 4])
def test_milp_stream_preserves_unresolved_status(monkeypatch, status):
    monkeypatch.setattr(solve, "highs", lambda *a, **kw: SimpleNamespace(status=status))
    stream = solve.milp_stream(I)
    with pytest.raises(StopIteration) as e:
        next(stream)
    assert e.value.value == f"highs status {status}"


def test_worker_preserves_status_after_partial_output(monkeypatch):
    results = iter([("optimal", frozenset("a")), ("highs status 1", None)])
    monkeypatch.setattr(solve, "milp", lambda *a: next(results))
    a, b = bench.mp.Pipe(duplex=False)
    try:
        bench._child("milp", I, b)
        assert a.recv()[0] == "start"
        assert a.recv()[2] == ["a"]
        assert a.recv()[2] == "highs status 1"
    finally:
        a.close()
        b.close()


def test_unresolved_minimum_has_no_infeasibility_agreement(monkeypatch):
    monkeypatch.setattr(solve, "milp", lambda *a: ("highs status 1", None))
    L = dict(bench.LIMITS, repeats=1)
    monkeypatch.setattr(bench, "run", lambda *a: ([], {
        "status": "highs status 1", "count": 0, "stored": 0, "t_total": None}))
    assert bench.instance_record(I, L)["highs_agrees_with_witness"] is None
    assert bench.sens(I)["feasible"] is None


@pytest.mark.parametrize("encoding", ["utf-8", "utf-8-sig", "utf-16"])
def test_cli_reads_shell_redirected_json(tmp_path, encoding):
    d = {"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}
    path = tmp_path / "instance.json"
    path.write_text(json.dumps(d), encoding=encoding)
    assert cli.load(path)[1].E == (frozenset("a"),)


@pytest.mark.parametrize("kw", [
    {"V": ("a", "a")}, {"E": ({"d"},)}, {"F": ({"d"},)},
    {"P": ()}, {"P": ("p", "p"), "E": (set("a"), set("b"))},
    {"k": -1}, {"k": 0.5}, {"k": True},
])
def test_instance_rejects_data_outside_the_model(kw):
    d = {"V": tuple("abc"), "P": ("p",), "E": (set("abc"),)}
    with pytest.raises(ValueError):
        Instance(**dict(d, **kw))


def test_unknown_supplier_cannot_be_certified():
    assert not certify.feasible(I, {"a", "unknown"})


@pytest.mark.parametrize("C", [[frozenset()], [frozenset("ab")], [frozenset("d")]])
def test_completeness_rejects_invalid_family(C):
    assert certify.complete(I, C) == ("invalid", None)


@pytest.mark.parametrize("family, status", [((), "missing"), ((frozenset(),), "complete")])
def test_empty_basket_completeness_accepts_iterators(family, status):
    J = Instance(V=(), P=(), E=())
    assert certify.complete(J, iter(family))[0] == status


def test_stats_count_edges_before_superset_reduction():
    J = Instance(V=tuple("abc"), P=("p", "q", "r"),
                 E=(set("ab"), set("ab"), set("abc")))
    st, _ = bench.stats(J)
    assert st["H_edges_before_reduction"] == 3
    assert st["H_edges_after_reduction"] == 1


def test_scenario_membership_breaks_supplier_symmetry():
    """All pairs except ab are feasible; cross-class pairs and cd are two orbits."""
    J = Instance(V=tuple("abcd"), P=("p",), E=(set("abcd"),), k=1, F=(set("ab"),))
    _, sig = bench.stats(J)
    assert bench.orbits(solve.exhaustive(J), sig) == 2
    assert sorted(sorted(v) for v in sig.values()) == [["a", "b"], ["c", "d"]]


def test_report_keeps_unresolved_minimum_status():
    res = json.loads(RESULTS.read_text())
    res["instances"] = deepcopy(res["instances"][:2])
    r = res["instances"][1]
    r["min"] = {"status": "highs status 1", "size": None}
    r["infeasibility"] = None
    text = report.tables(res)
    assert "highs status 1" in text
    assert "unresolved" in text


def test_report_without_sensitivity_runs(tmp_path):
    res = json.loads(RESULTS.read_text())
    for r in res["instances"]:
        r.pop("sensitivity", None)
    path = tmp_path / "suite.json"
    path.write_text(json.dumps(res))
    scores = verdicts.score(path)
    assert scores["H5 / K7 feasibility verdict flips (primary)"] == {
        "merged": "not measured", "identity_candidates_merged": "not measured"}
    text = report.tables(res)
    assert "Sensitivity variants were not measured" in text
    assert "changes in 0 primary cases" not in text


@pytest.mark.parametrize("panels", [[[]], [["a"], ["a"]]])
def test_cli_verify_fails_for_an_invalid_certificate(tmp_path, capsys, panels):
    path = tmp_path / "invalid.json"
    path.write_text(json.dumps({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0,
                                "panels": panels,
                                "run": {"status": "complete", "count": len(panels)}}))
    with pytest.raises(SystemExit) as e:
        cli.main(["verify", str(path)])
    assert e.value.code == 1
    out = json.loads(capsys.readouterr().out)
    assert not out["all_feasible_and_minimal"] or not out["distinct_panels"]
    assert out.get("milp_exhaustion") != "complete"


def test_cli_verify_fails_for_a_missing_panel(tmp_path, capsys):
    path = tmp_path / "incomplete.json"
    path.write_text(json.dumps({"V": ["a", "b"], "P": ["p"], "E": [["a", "b"]], "k": 0,
                                "panels": [["a"]], "run": {"status": "complete", "count": 1}}))
    with pytest.raises(SystemExit) as e:
        cli.main(["verify", str(path)])
    assert e.value.code == 1
    out = json.loads(capsys.readouterr().out)
    assert out["milp_exhaustion"] == "missing"


def test_build_rejects_negative_failure_budget(capsys):
    with pytest.raises(SystemExit) as e:
        cli.main(["build", "beans pinto can", "--k", "-1"])
    assert e.value.code == 2
    assert "k must be a nonnegative integer" in capsys.readouterr().err


@pytest.mark.parametrize("kw", [{"timeout": 0}, {"timeout": float("nan")},
                                {"timeout": float("inf")}, {"cap": -1}, {"K": 0}])
def test_enumeration_rejects_invalid_limits(kw):
    with pytest.raises(ValueError):
        bench.run("berge", I, **dict({"timeout": 1, "cap": 10, "K": 1}, **kw))
