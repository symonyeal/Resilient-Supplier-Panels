"""Acquisition (network). Downloads to data/fetched/, never over the frozen data/raw/."""
import hashlib
import json
import urllib.request
from datetime import datetime, UTC
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fetch(url, dir):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (srl acquire)"})
    with urllib.request.urlopen(req, timeout=60) as r:
        body, lm = r.read(), r.headers.get("Last-Modified")
    path = Path(dir) / url.rsplit("/", 1)[1]
    path.write_bytes(body)
    return {"url": url, "retrieved_utc": datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sha256": hashlib.sha256(body).hexdigest(), "bytes": len(body), "last_modified": lm}


def fetch_all():
    """Fetch every manifest source; report whether each matches the frozen snapshot."""
    frozen = json.loads((ROOT / "data/manifest.json").read_text())
    out = ROOT / "data/fetched"
    out.mkdir(parents=True, exist_ok=True)
    got = {}
    for name, m in frozen.items():
        e = fetch(m["url"], out)
        e["matches_frozen"] = e["sha256"] == m["sha256"]
        got[name] = e
    (out / "manifest.json").write_text(json.dumps(got, indent=1))
    return got
