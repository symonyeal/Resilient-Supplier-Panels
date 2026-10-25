"""Run the whole-class protocol in docs/BENCHMARKS.md without overwriting a campaign."""
import argparse
import json
import subprocess
from pathlib import Path


def campaign(binary, dest):
    """Save input and output documents; a killed solve reports only timeout."""
    dest.mkdir(parents=True, exist_ok=False)
    change = subprocess.run([binary, "change"], check=True, capture_output=True)
    provenance = json.loads(change.stdout)["provenance"]
    (dest / "provenance.json").write_text(json.dumps(provenance, indent=1) + "\n", encoding="utf-8")
    rows = []
    for cls in ("description", "program", "naics", "material"):
        for minimum in (1, 2):
            stem = f"{cls}-min{minimum}"
            instance = dest / f"{stem}.json"
            build = ["build", "--class", cls, "--min-suppliers", str(minimum), "--k", "1"]
            subprocess.run([binary, *build, "-o", str(instance)], check=True)
            doc = json.loads(instance.read_bytes())
            count = subprocess.run([binary, "count", str(instance), "--timeout", "10"],
                                   check=True, capture_output=True)
            try:
                solve = subprocess.run([binary, "solve", str(instance)], check=True,
                                       timeout=10, capture_output=True)
                solution = json.loads(solve.stdout)
            except subprocess.TimeoutExpired:
                solution = {"status": "timeout", "wall_limit_s": 10}
            row = {"class": cls, "min_suppliers": minimum, "build": build,
                   "products": len(doc["P"]), "suppliers": len(doc["V"]),
                   "count": json.loads(count.stdout), "solution": solution}
            rows.append(row)
            print(cls, minimum, row["count"], solution["status"], flush=True)
    (dest / "summary.json").write_text(json.dumps(rows, indent=1) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="srl")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    campaign(str(Path(args.binary).resolve()) if Path(args.binary).exists() else args.binary, args.output)
