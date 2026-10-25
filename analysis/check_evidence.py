"""Check retained USAspending records, recipient sums and the published candidate links."""
import gzip
import hashlib
import json
from collections import defaultdict
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def check():
    """Reconcile every aggregate with distinct award records; fail on missing evidence."""
    folder = ROOT / "data/evidence"
    manifest = json.loads((folder / "manifest.json").read_bytes())
    data = {}
    for name, entry in manifest.items():
        packed = (folder / name).read_bytes()
        raw = gzip.decompress(packed)
        assert hashlib.sha256(packed).hexdigest() == entry["sha256"], name
        assert hashlib.sha256(raw).hexdigest() == entry["uncompressed_sha256"], name
        data[name.removesuffix(".gz")] = json.loads(raw)
    raw = data["awards-raw.json"]["awards"]
    assert raw and len({a["generated_internal_id"] for a in raw}) == len(raw)
    totals = defaultdict(Decimal)
    counts = defaultdict(int)
    for award in raw:
        uei = award["Recipient UEI"]
        totals[uei] += Decimal(str(award["Award Amount"] or 0))
        counts[uei] += 1
    catalog = json.loads((ROOT / "data/catalog/usaspending.json").read_bytes())
    assert catalog["awards"] == len(raw)
    assert catalog["links"] == {s: r["both"] for s, r in data["links.json"].items() if r["both"]}
    for uei, recipient in catalog["recipients"].items():
        assert abs(totals[uei] - Decimal(str(recipient["amount"]))) <= Decimal("0.01"), uei
        assert recipient["awards"] == counts[uei], uei
    assert all(u in catalog["recipients"] for links in catalog["links"].values() for u in links)
    print(f"Evidence checked: {len(raw)} distinct awards, {len(catalog['recipients'])} recipient totals, "
          f"{len(catalog['links'])} candidate supplier links, {len(manifest)} hashes.")


if __name__ == "__main__":
    check()
