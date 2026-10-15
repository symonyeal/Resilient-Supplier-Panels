"""Rebuild the public case data with certificates: python analysis/build_site.py."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from srl.portfolio import build

if __name__ == "__main__":
    data = build(ROOT / "site", ROOT)
    for c in data["cases"]:
        print(c["id"], {k: {"status": m["status"], "minimum": m["minimum_size"],
                           "count": m["count"]}
                        for k, m in c["modes"].items()})
