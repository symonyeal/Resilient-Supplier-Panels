"""Public file contracts, exact basket import, data selection and CLI interoperability."""
import json
import shutil
import tomllib
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from srl import __version__, cli, contracts
from srl.resources import data_path

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "srl/schemas/document-1.0.json").read_bytes())


def test_versioned_cli_pipeline_and_schema(tmp_path):
    b, s, e = [tmp_path / f"{name}.json" for name in ("basket", "solution", "family")]
    cli.main(["build", "--basket", str(ROOT / "examples/canned.csv"), "--k", "1", "-o", str(b)])
    cli.main(["solve", str(b), "-o", str(s)])
    cli.main(["enum", str(b), "--method", "berge", "-o", str(e)])
    validator = Draft202012Validator(SCHEMA)
    for path in (b, s, e):
        validator.validate(json.loads(path.read_bytes()))
    assert len(json.loads(s.read_bytes())["panel"]) == 2
    assert len(json.loads(e.read_bytes())["panels"]) == 1284


def test_schema_is_well_formed_and_version_matches_package():
    Draft202012Validator.check_schema(SCHEMA)
    assert tomllib.loads((ROOT / "pyproject.toml").read_text())["project"]["version"] == __version__


def test_legacy_instance_gives_a_versioned_solution(tmp_path):
    path, out = tmp_path / "legacy.json", tmp_path / "solution.json"
    path.write_text(json.dumps({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}))
    cli.main(["solve", str(path), "-o", str(out)])
    Draft202012Validator(SCHEMA).validate(json.loads(out.read_bytes()))
    cli.main(["enum", str(path), "--method", "exhaustive", "-o", str(out)])
    Draft202012Validator(SCHEMA).validate(json.loads(out.read_bytes()))


def test_future_version_is_rejected(tmp_path):
    path = tmp_path / "future.json"
    path.write_text(json.dumps({"schema_version": "2.0", "V": [], "P": [], "E": [], "k": 0}))
    with pytest.raises(ValueError, match="unsupported schema version"):
        contracts.load(path)


@pytest.mark.parametrize("text", ["name\na\n", "product\n", "product\na\na\n",
                                 "product\na,b\n"])
def test_invalid_csv_is_rejected(tmp_path, text):
    path = tmp_path / "basket.csv"
    path.write_text(text)
    with pytest.raises(ValueError):
        contracts.basket_csv(path)


def test_data_override_after_import_is_used(tmp_path, monkeypatch):
    for name in ("raw/QBL.xlsx", "manifest.json", "catalog/mappings.csv"):
        dst = tmp_path / name
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / "data" / name, dst)
    manifest = json.loads((tmp_path / "manifest.json").read_bytes())
    manifest["QBL.xlsx"]["retrieved_utc"] = "1900-01-01T00:00:00Z"
    (tmp_path / "manifest.json").write_text(json.dumps(manifest))
    monkeypatch.setenv("SRL_DATA_DIR", str(tmp_path))
    out = tmp_path / "audit.json"
    cli.main(["audit", "-o", str(out)])
    assert json.loads(out.read_bytes())["stale"]
    assert data_path("catalog/mappings.csv") == tmp_path / "catalog/mappings.csv"


def test_default_data_does_not_depend_on_working_directory(tmp_path, monkeypatch):
    monkeypatch.delenv("SRL_DATA_DIR", raising=False)
    before = data_path("raw/QBL.xlsx")
    monkeypatch.chdir(tmp_path)
    assert data_path("raw/QBL.xlsx") == before and before.is_file()
