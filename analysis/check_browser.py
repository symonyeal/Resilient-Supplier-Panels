"""Exercise the public case controls and downloads in a real browser."""
import argparse
import json
import sys
import tempfile
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

ROOT = Path(__file__).resolve().parents[1]


def check(url, screenshots=None):
    """Check retained/lost/infeasible coverage, alternatives, downloads and mobile layout."""
    with sync_playwright() as p, tempfile.TemporaryDirectory(prefix="srl-browser-") as tmp:
        browser = p.chromium.launch(channel="msedge" if sys.platform == "win32" else None)
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
        page.goto(url)
        expect(page.locator("#hero-count")).to_have_text("1,284")
        expect(page.locator("#coverage-status")).to_have_text("Coverage retained")
        if screenshots:
            Path(screenshots).mkdir(parents=True, exist_ok=True)
            page.screenshot(path=str(Path(screenshots) / "desktop.png"))
        page.get_by_label("Simulated outage", exact=True).select_option("pair")
        expect(page.locator("#coverage-status")).to_have_text("Coverage lost")
        expect(page.locator("#status-tag")).to_have_text("0 / 3 products")
        page.locator('[name="budget"][value="0"]').check()
        vendor = page.evaluate("DATA.cases[0].modes['0'].panels[0].members[0]")
        page.locator("#outage").select_option(vendor)
        expect(page.locator("#coverage-status")).to_have_text("Coverage lost")
        page.locator("#reset").click()
        page.locator("#panel").select_option("1")
        expect(page.locator('[data-panel="1"]')).to_have_attribute("aria-pressed", "true")
        page.locator('[data-panel="2"]').click()
        expect(page.locator("#panel")).to_have_value("2")
        page.locator("#supplier-search").fill("not-a-supplier-xyz")
        expect(page.locator("#matrix-body")).to_contain_text("No suppliers match")
        page.locator('[data-case="scarce"]').click()
        expect(page.locator("#coverage-status")).to_have_text("Coverage requirement is infeasible")
        expect(page.locator("#panel")).to_be_disabled()
        expect(page.locator("#infeasible")).to_contain_text("Blocked by:")
        page.locator("#reset").click()
        with page.expect_download() as event:
            page.locator("#download-report").click()
        path = Path(tmp) / event.value.suggested_filename
        event.value.save_as(path)
        report = path.read_text(encoding="utf-8")
        assert "Coverage retained" in report and "1bb9de48" in report
        assert "qualification" in report.lower() and "Original panel certificates" in report
        with page.expect_download() as event:
            page.locator("#download-family").click()
        path = Path(tmp) / event.value.suggested_filename
        event.value.save_as(path)
        family = json.loads(path.read_bytes())
        assert family["kind"] == "enumeration" and family["run"]["count"] == 1284
        assert len(family["panels"]) == 1284 and family["k"] == 1
        page.locator("#outcomes").scroll_into_view_if_needed()
        expect(page.locator("#outcome-count")).to_have_text("88 of 88 shown")
        expect(page.locator("#outcome-chart .point")).to_have_count(55 + 1)   # feasible + ring
        if screenshots:
            page.screenshot(path=str(Path(screenshots) / "outcomes.png"))
        page.locator("#f-outcome").select_option("bound")
        expect(page.locator("#outcome-count")).to_have_text("4 of 88 shown")
        expect(page.locator("#record")).to_contain_text("Minimal panels · lower bound")
        page.locator("#f-outcome").select_option("infeasible")
        expect(page.locator("#record")).to_contain_text("Infeasibility witness")
        page.locator("#f-product").fill("not-a-product-xyz")
        expect(page.locator("#outcome-rows")).to_contain_text("No instances match")
        expect(page.locator("#download-record")).to_be_disabled()
        page.locator("#f-reset").click()
        page.locator('[data-record="17"]').click()
        expect(page.locator("#record-title")).to_contain_text("uniform · 20 products, k = 1")
        with page.expect_download() as event:
            page.locator("#download-outcomes").click()
        path = Path(tmp) / event.value.suggested_filename
        event.value.save_as(path)
        assert len(path.read_text(encoding="utf-8").splitlines()) == 1 + 88
        with page.expect_download() as event:
            page.locator("#download-record").click()
        path = Path(tmp) / event.value.suggested_filename
        event.value.save_as(path)
        record = json.loads(path.read_bytes())
        assert record["k"] == 1 and record["infeasibility"] and record["provenance"]["seed"] == 1
        page.set_viewport_size({"width": 390, "height": 844})
        page.reload()
        expect(page.locator("#coverage-status")).to_have_text("Coverage retained")
        assert page.evaluate("document.documentElement.scrollWidth <= innerWidth")
        page.keyboard.press("Tab")
        expect(page.locator(".skip")).to_be_focused()
        page.locator("#outage").select_option("pair")
        expect(page.locator("#coverage-status")).to_have_text("Coverage lost")
        if screenshots:
            page.locator("#lab").scroll_into_view_if_needed()
            page.screenshot(path=str(Path(screenshots) / "mobile.png"))
        assert not errors, errors
        browser.close()
        print("Browser checks passed: controls, failures, infeasibility, downloads, "
              "keyboard, 390px.")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default=(ROOT / "site/index.html").as_uri())
    ap.add_argument("--screenshots", type=Path)
    a = ap.parse_args()
    check(a.url, a.screenshots)
