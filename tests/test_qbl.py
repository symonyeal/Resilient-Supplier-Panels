"""Seams 1 and 2: workbook -> records with provenance -> instance.

Real fixture: the frozen snapshot data/raw/QBL.xlsx (public USDA data, hash in
data/manifest.json). Expected literals were read off the workbook in Excel-independent
inspection (openpyxl, 2026-10-15), not from this reader.
"""
import hashlib
import json
from pathlib import Path

from srl import qbl
from srl.instance import Instance

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "data/raw/QBL.xlsx"


def row(n, name, addr="1 Main St", zip_="12345", product="Fresh Apples", size="Small"):
    return {"row": n, "Vendor Name": name, "Billing Address": addr, "City": "X", "State": "WA",
            "ZIP Code": zip_, "Size": size, "Product": product}


def test_snapshot_hash_matches_manifest():
    m = json.loads((ROOT / "data/manifest.json").read_text())["QBL.xlsx"]
    assert hashlib.sha256(RAW.read_bytes()).hexdigest() == m["sha256"]


def test_reader_on_frozen_snapshot():
    rows = qbl.read(RAW)
    assert len(rows) == 983 and rows[0]["row"] == 2
    assert rows[0]["Vendor Name"] == "1st Choice Logistics DBA JAWS Transportation"
    assert rows[0]["ZIP Code"] == "83211"
    assert rows[0]["Product"] == "Potato Bulk For Process \nFRZ Potatoes Russet Fresh CTN"


def test_multiline_cells_split_and_tokens_normalised():
    _S, Q, _issues = qbl.records([row(2, "A Co", product="WF -Wheat Flour\n  Beans Pinto Can \n")])
    assert sorted(t for _, t, _ in Q) == ["beans pinto can", "wf - wheat flour"]
    assert all(r == 2 for _, _, r in Q)


def test_repeated_rows_merge_and_name_collisions_stay_distinct():
    rows = [row(2, "A Co"), row(3, "A  CO", product="Pears Bosc Fresh Ctn"),
            row(4, "A Co", addr="9 Other Rd"), row(5, "A Co, Inc.")]
    S, _Q, issues = qbl.records(rows)
    assert len(S) == 3                              # rows 2 and 3 share the identity key
    kinds = {(k, r) for r, k, _ in issues}
    assert ("same_identity", 3) in kinds
    assert ("identity_candidate", 4) in kinds and ("identity_candidate", 5) in kinds


def test_missing_fields_are_reported_not_dropped_silently():
    S, Q, issues = qbl.records([row(2, "", product="X Can"), row(3, "B Co", product="  ")])
    assert not S and not Q
    assert {(r, k) for r, k, _ in issues} == {(2, "missing_name"), (3, "missing_product")}


def test_zip_codes_padded_to_five_digits():
    assert qbl.records([row(2, "A", zip_="8518")])[0]["r0002"]["zip5"] == "08518"


def test_records_to_instance_keeps_all_eligible_suppliers():
    rows = [row(2, "A", product="x can\ny can"), row(3, "B", product="x can"), row(4, "C")]
    S, Q, _ = qbl.records(rows)
    I = qbl.instance(S, Q, ["x can", "y can"], k=1)
    assert isinstance(I, Instance) and I.k == 1
    assert I.E == (frozenset({"r0002", "r0003"}), frozenset({"r0002"}))
    assert I.V == ("r0002", "r0003")


def test_stale_snapshot_flag():
    assert qbl.age_days("2026-10-15T07:16:47Z", "2027-02-01") == 109
    assert qbl.stale("2026-10-15T07:16:47Z", "2027-02-01") and not qbl.stale(
        "2026-10-15T07:16:47Z", "2026-11-01")
