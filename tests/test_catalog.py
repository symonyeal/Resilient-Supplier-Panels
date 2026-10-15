"""Seam 2: the product dictionary that turns tokens into qualified products."""
from pathlib import Path

from srl import catalog, qbl

RAW = Path(__file__).resolve().parents[1] / "data/raw/QBL.xlsx"


def test_rules_classify_each_vocabulary():
    assert catalog.rule("100193 - pork shoulder, picnics") == "material"
    assert catalog.rule("311999 deb dry edible beans") == "naics"
    assert catalog.rule("wf - wheat flour") == ("program", "wf")
    assert catalog.rule("beans pinto can") == "description"
    assert catalog.rule("walnuts") == "commodity"


def test_mappings_split_double_codes_and_mark_broad():
    D = catalog.load(["dyo - high protein yogurt dfm - fluid milk", "fresh fruit", "walnuts"])
    assert D["dyo - high protein yogurt dfm - fluid milk"][1] == (
        "dyo - high protein yogurt", "dfm - fluid milk")
    assert D["fresh fruit"][0] == "broad" and D["walnuts"][0] == "commodity"


def test_every_snapshot_token_has_an_entry_and_one_merged_image():
    _S, Q, _ = qbl.records(qbl.read(RAW))
    D = catalog.load({t for _, t, _ in Q})
    assert all(c in catalog.CLASSES for c, *_ in D.values())
    M = catalog.merged_of(D)
    assert M["wf - wheat flour"] == "wf" and M["pp - peanut products"] == "pp"
    assert M["pp - pasta products"] == "pa"


def test_multi_product_tokens_reach_both_products():
    rows = [{"row": 2, "Vendor Name": "A", "Billing Address": "1", "City": "", "State": "",
             "ZIP Code": "1", "Size": "", "Product": "DYO - High Protein Yogurt DFM - Fluid Milk"}]
    S, Q, _ = qbl.records(rows)
    D = catalog.load({t for _, t, _ in Q})
    I = qbl.instance(S, Q, ["dfm - fluid milk"], product=lambda t: D[t][1])
    assert I.E == (frozenset({"r0002"}),)


def test_no_snapshot_token_holds_two_codes_unsplit():
    """A line naming two materials or programmes is two qualifications (docs/DATA.md)."""
    import re
    _, Q, _ = qbl.records(qbl.read(RAW))
    D = catalog.load({t for _, t, _ in Q})
    two = re.compile(r"\d{6} - .* \d{6} - |^[a-z]{1,5}\d? - .* [a-z]{2,5}\d? - [a-z]")
    assert all(len(s) == 2 for t, (_, s, _, _) in D.items() if two.search(t)
               and t != "pa - pasta - pasta product")
