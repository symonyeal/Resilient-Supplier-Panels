"""Install a built wheel in the system environment; exercise its CLI away from the repo."""
import hashlib
import json
import os
import subprocess
import sys
import sysconfig
import tempfile
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


def check(wheel):
    """Installed entry point and bundled data must work without source-checkout imports."""
    subprocess.run([sys.executable, "-m", "pip", "install", "--force-reinstall", "--no-deps",
                    str(Path(wheel).resolve())], check=True)
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    env.pop("SRL_DATA_DIR", None)
    cli = Path(sysconfig.get_path("scripts")) / ("srl.exe" if os.name == "nt" else "srl")
    with tempfile.TemporaryDirectory(prefix="srl-install-") as tmp:
        def run(*args):
            r = subprocess.run([str(cli), *args], cwd=tmp, env=env, check=True,
                               capture_output=True, text=True)
            return r.stdout
        assert run("--version").strip() == "srl 0.2.0"
        assert json.loads(run("audit"))["rows"] == 983
        run("build", "beans pinto can", "beans vegetarian can", "tomato sauce can", "--k", "1",
            "--output", "basket.json")
        run("solve", "basket.json", "--output", "solution.json")
        run("enum", "basket.json", "--method", "berge", "--timeout", "30",
            "--output", "family.json")
        verified = json.loads(run("verify", "family.json"))
        assert verified["panels"] == 1284 and verified["milp_exhaustion"] == "complete"
        schema = json.loads(run("schema"))
        validator = Draft202012Validator(schema)
        for name in ("basket", "solution", "family"):
            d = json.loads((Path(tmp) / f"{name}.json").read_bytes())
            validator.validate(d)
        solution = json.loads((Path(tmp) / "solution.json").read_bytes())
        assert len(solution["panel"]) == 2
        expected = hashlib.sha256((ROOT / "data/raw/QBL.xlsx").read_bytes()).hexdigest()
        assert solution["instance"]["data_sha256"] == expected
        print("Installed-wheel checks passed: audit, build, solve, enum, verify, schema; "
              "1284 panels.")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python analysis/check_install.py dist/srl-0.2.0-py3-none-any.whl")
    check(sys.argv[1])
