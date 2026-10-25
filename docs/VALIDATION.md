# Validation

## Release 0.4 additions

The three embedded workbook hashes are checked against the manifest. Historical changes
and seeded gap rankings have real-workbook regression checks in `tests/data.rs`; selected
backup changes were independently read from the workbooks. `tests/entities.rs` checks
transitive grouping, separate identity merging, owner-outage matrix certificates,
malformed catalog rejection and the frozen catalogs' supplier IDs/names. It also requires
missing award amounts to fail instead of silently dropping them.

`analysis/check_evidence.py` checks four compressed-source hashes, all 13,798 distinct
award IDs, every published recipient aggregate (336) against raw Award Amount values,
and all 341 candidate links against the retained join table. This checks the extraction
and aggregation; it does not prove that a heuristic link identifies a QBL facility.
`tests/analyses.rs` independently checks saved whole-class panels for matrix feasibility
and minimality, plus infeasibility witnesses. It does not independently enumerate the
75-billion-panel family or prove the solver's optimality.

The original saved campaigns and reports retain their results. Every source change
requires rebuilding site provenance. The [0.4 findings](findings/2026-10-25-open-work.md)
give the protocol, observed statuses and identification review log.

This file lists what must be checked, how, and what has been checked. A check is a
certificate (feasibility, a witness, agreement of independent routes, a completeness
search), never an output snapshot; the one exception, agreement with the thesis code's
recorded outputs, is labelled T.

Assurance levels:
- **D**: direct certificate from the matrix.
- **X**: exhaustive: agreement with exhaustive search over subsets, or the exhaustive
  completeness search (MODEL, Theorem 3.3).
- **A**: agreement of independent routes.
- **Q**: the class count of MODEL, Theorem 4.
- **F**: FK-A duality decision (thesis algorithm).
- **T**: agreement with recorded outputs of the thesis code (`fka`/`fkb` at `1bb9de4`).
- **S**: solver result (HiGHS), not a formal proof. Release 0.2 only.

## Release 0.3 (Rust), 25 October 2026

**Baseline.** The Python package at `99cd50c` passed its gate (84 tests, ruff, pip check).
Its outputs were captured on the thesis code checked out at the pin `1bb9de4` (the local
installation was an editable checkout at a later thesis commit, whose split-variable
refactor gives the same choices). The capture is kept outside the repository; the
compact part every commit checks is `tests/reference/`.

**Port parity** (`tests/parity.rs` with the capture, `tests/thesis.rs` with the compact
reference). Two-way comparison on every key; the comparator is itself tested on planted
differences, and a planted change of FK-A's tie-break is detected.

| Output | Compared on | Result |
| --- | --- | --- |
| workbook rows, supplier records, issues, catalog, merged images, audit | all 983 rows, 715 tokens | equal |
| strict and merged matrices | 710 strict products and the merged ones | equal |
| Python Mersenne Twister draws; basket lists | seeds 0, 1, 2, 12345, 2^40 + 7; baskets for seeds 0–5 | equal |
| hypergraph with provenance, stats, classes, infeasibility and essential witnesses, minimum size and status | 88 seed-1 instances and both sensitivity variants of each | equal |
| Berge family, member witnesses, orbits, first 300 FK-B panels in order | 88 seed-1 instances | equal |
| FK-A tree and DAG nodes; with one panel removed, tree and DAG nodes and the conflicting assignment | 46 families of at most 2,000 (40 with two or more panels) | equal |
| all of the above, FK-B node counts per dualisation pass (130 instances), exhaustive family | 300 random instances, k ∈ {0, 1, 2}, a quarter with explicit scenarios | equal |
| pre-registered scores; site evidence of the 0.2 campaign | `results/suite-seed1.json` | equal |
| site cases (families, certificates, outages, shortlists) | 3 cases × 2 failure budgets | equal with the 0.2 optimum as the shortlist's first panel; see DECISIONS for the deterministic choice |
| `results-seed1.md` rebuilt from the saved 0.2 campaign | whole file | equal except the line naming the command |
| common-owner instances and complete families | 4 cases | equal: 32, 168, 21 and 168 panels |

**New routes.** MMCS and the class count agree with Berge on every family of both parity
sets; the cut route and the completeness search do on every family of at most 2,000.

## Tests (every commit)

`cargo test --release`; `cargo clippy --release --all-targets -- -D warnings` is clean.

| Claim | Check | Level |
| --- | --- | --- |
| Tr on a hand-worked instance: {ac, bc, bd}; k = 1 forces abcd | `tests/routes.rs` | hand |
| Empty basket → {∅}; empty edge → infeasible with witness | `tests/routes.rs` | D |
| Single-supplier product is essential and blocks k = 1 | `tests/routes.rs` | D, X |
| Duplicate and superset edges removed; Tr unchanged; provenance kept | `tests/routes.rs`, `src/instance.rs` | X |
| Identical signatures: interchangeable at k = 0, both needed at k = 1 | `tests/routes.rs` | X |
| Correlated scenario F = {a, b} | `tests/routes.rs` | X |
| Berge = FK-B = cuts = MMCS = exhaustive = class count on 350 random instances, k ∈ {0, 1, 2}, with and without scenarios | `tests/routes.rs` | X, A, Q |
| Every panel minimal by matrix witness; essential set = intersection of Tr; minimum = least size; completeness accepts Tr and returns the panel removed from it | `tests/routes.rs` | D, X |
| Permutation invariance (suppliers and products reordered) | `tests/routes.rs` | X |
| Timeout yields partial output, all certified; cap stops streams; Berge counts exactly; working-set limit reported as `memory`; a failed route is `error` | `tests/routes.rs` | D |
| Class count of 435³ panels on three disjoint 30-supplier products at k = 1 | `tests/routes.rs` | Q |
| Agreement with the thesis code on 300 random and 88 seed-1 instances | `tests/thesis.rs` | T |
| Saved minima, witnesses and essential sets of both campaigns recomputed; both reports rebuild byte for byte | `tests/site.rs` | D |
| Published cases: certificates, exhaustion, outage coverage, data and source hashes, evidence = campaign | `tests/site.rs` | D, X |
| Report never prints a partial count as exact | `src/report.rs` | — |
| Snapshot hash = manifest; reader = independent reader (openpyxl literals) on row 2; multiline cells, merged rows, name collisions, missing fields, ZIP padding, staleness | `tests/data.rs` | — |
| Every token has an entry and one merged image; no token holds two codes unsplit | `tests/data.rs` | — |
| CLI: versioned pipeline (1,284 panels on the example by every route and by `count`), legacy and UTF-16 input, input errors exit 2, failed verification exits 1, data override, schema, campaigns never overwritten | `tests/cli.rs` | — |

## Empirical results (per campaign)

Every instance record in `results/suite-seed*.json` carries:
- the minimum panel and a minimality witness for each member (D);
- either an infeasibility witness, agreeing with the solver, or the essential suppliers
  with their witnesses (D);
- for each route, every returned panel certified minimal (D);
- for complete, fully stored families:
  - agreement between routes (A);
  - the completeness search (X in 0.3; MILP exhaustion, S, in 0.2);
  - exhaustive agreement where the support is at most 18 (X);
  - an FK-A duality decision within 200,000 nodes (F);
  - the essential set equal to the intersection of the family (D, A);
- in 0.3, the class count and its agreement with the enumerated count (Q).

### Seed 1, release 0.3 (code `55e8e4a`, clean; data hash = manifest; 88 instances)

| Check | Level | Passed |
| --- | --- | --- |
| minimum panel certified minimal (or infeasible) | D | 88 / 88 |
| solver infeasibility ⇔ matrix witness | D | 88 / 88 |
| every panel returned by every route certified minimal (partial runs included) | D | 352 / 352 runs |
| complete routes agree on the family | A | 83 / 83 with a fully stored family |
| completeness search finds no missing panel | X | 83 / 83 |
| exhaustive search agrees (support ≤ 18) | X | 46 / 46 |
| essential set = intersection of the complete family | D, A | 50 / 50 |
| FK-A decides C = Tr(H); `fk_a_memo` agrees | F | 46 / 46 (families ≤ 2,000) |
| class count = enumerated count | Q, A | 88 / 88 |
| class count and class vectors = stored complete family | Q, X | 83 / 83 |
| minimum, statuses, witnesses, sensitivity verdicts, orbits, FK-A counts = 0.2 campaign | — | 88 / 88 |

Not checked: the five families above the 10,000-panel cap are counted (Berge counts beyond
storage, and the class count agrees) but not stored, so neither the completeness search
nor FK-A ran on them; any check under the sensitivity variants beyond feasibility and the
minimum.

Agreement of Berge with FK-B and MMCS tests the shared scenario hypergraph against itself.
The independent test of that transformation is the agreement of exhaustive search and the
completeness search, both of which read the matrix.

### Seed 1, release 0.2 (code `09cb123`, clean; data hash = manifest; 88 instances)

| Check | Level | Passed |
| --- | --- | --- |
| minimum panel certified minimal (or infeasible) | D | 88 / 88 |
| HiGHS infeasibility ⇔ matrix witness | D, S | 88 / 88 |
| every panel returned by every route certified minimal (partial runs included) | D | 88 / 88 instances |
| complete routes agree on the family | A | 83 / 83 with a fully stored family |
| MILP exhaustion finds no missing panel | S | 83 / 83 |
| exhaustive search agrees (support ≤ 18) | X | 46 / 46 |
| essential set = intersection of the complete family | D, A | 50 / 50 |
| FK-A decides C = Tr(H); `fk_a_memo` agrees | F | 46 / 46 (families ≤ 2,000) |

Not checked:
- families that every route left incomplete (4 primary cases), and families above 2,000
  panels by FK-A;
- any check under the sensitivity variants beyond feasibility and the minimum.

Agreement of Berge with FK-B tests the shared scenario hypergraph against itself. The
independent test of that transformation is the agreement of exhaustive search and MILP
exhaustion, both of which read the matrix.

## Common-owner case, 20 October 2026

The frozen-qualification study in COMMON_OWNER_CASE.md was checked by direct exhaustive
enumeration, matrix feasibility/minimality checks and a separate MILP completeness search
for each of its four cases. All returned complete. The one-supplier and combined
one-supplier/common-owner families were compared as sets and agreed exactly. The existing
suite returned 84 passed; Ruff and dependency checks passed. The pinned thesis revision
was used. These are conditional coverage checks, not an empirical outage or capacity test.

In release 0.3 the four cases were rebuilt and enumerated with `srl`: instances, names,
hashes and complete families equal the committed documents.

## Maintenance assessment (2026-10-15)

Baseline `13c6dc9` passed 31 tests, ruff and pip check. Focused regressions nevertheless
reproduced these defects:

- MILP enumeration discarded unresolved HiGHS statuses, and the worker called the run
  complete. The status now survives the generator and process boundary; unresolved solves
  give neither an exact count nor agreement with an infeasibility witness.
- Completeness accepted invalid families: `{∅}` could certify a nonempty basket complete.
  Families are now checked for minimality first. CLI verification rejects invalid or
  duplicate panels and a falsely claimed complete family with exit status 1.
- PowerShell 5.1 redirection produced UTF-16 JSON that the CLI could not load. JSON is now
  read as bytes, so the standard decoder handles UTF-8, UTF-8 BOM and UTF-16.
- Invalid instances admitted duplicate suppliers, qualifications outside V and fractional
  failure budgets. These are now input errors, checked independently of Python assertions.
- Reports called unresolved minima infeasible and failed when sensitivity runs were absent.
  They now preserve statuses and label unmeasured sensitivity results.
- The pre-reduction edge count omitted discarded superset edges. It now counts every
  generated scenario edge. Signature classes now also preserve each explicit failure set.

Verified on code `2df0db0`, Python 3.14.7, the frozen QBL hash in the manifest, and the
installed thesis revision `1bb9de4` (the installation's commit matches the pin):

- **69 tests passed**; ruff and pip check passed. The added tests cover solver-status fault
  injection through an actual worker process, invalid certificates, input validation,
  shell encodings, measurement definitions and saved-result reproduction.
- All **88** saved instances have the same minimum size/status, infeasibility witness and
  essential-supplier certificates when recomputed from the matrix. All **55** feasible
  instances retain certified minimal stored panels.
- All **84** established exact counts were reproduced with Berge; every regenerated panel
  was certified minimal, and independently encoded MILP exhaustion found no missing panel.
- The README's PowerShell build → solve → enumerate → verify sequence returned **1,284**
  distinct certified panels and a complete exhaustion check.
- The saved report rebuilds without changes. Raw data, benchmark results and published
  findings remain at their original revisions.

**Historical metadata discrepancy.** In **72 of 88** saved records,
`H_edges_before_reduction` omits scenario edges removed as supersets. All other recorded
structural fields agreed with recomputation. The count is fixed for future runs; the old
JSON retains its original measurements and provenance. No reported conclusion uses that
field.

**Not measured:** a fresh timed benchmark campaign; completion of the four censored cases;
new snapshots; memory use. The re-enumerations above validate counts, not old timings or
the exact panel identities of historical streams (those panels were not stored in the
suite JSON). Solver failure paths were injected in tests, not observed in these 88 solves.

## Portfolio release 0.2.0

The case data was regenerated from committed source `6db6053`. Its complete source
fingerprint, exact input hashes, installed thesis revision and clean-source flag are in
`site/cases.json`. This is separate from the historical seed-1 benchmark provenance.

| Real basket | k | Minimum size | Complete minimal-panel count |
| --- | ---: | ---: | ---: |
| canned products | 0 | 1 | 80 |
| canned products | 1 | 2 | 1,284 |
| poultry | 0 | 1 | 2 |
| poultry | 1 | 2 | 1 |
| scarce products | 0 | 3 | 2 |
| scarce products | 1 | infeasible | 0 |

Source: `python analysis/build_site.py`, frozen QBL and strict mappings. Every family is
fully stored, every panel has direct matrix minimality certificates, a separately encoded
MILP finds no omitted panel, and minimum sizes agree with the complete families. Outage
coverage is independently rechecked in `tests/test_portfolio.py`.

Checks:

- `python -m pytest -q`: **81 passed**; includes the original saved minima and report
  reproduction, public JSON contracts, CSV errors, data overrides and generated cases.
- `python -m ruff check .`: **All checks passed!**
- `python -m pip check`: **No broken requirements found.**
- `python analysis/check_browser.py`: retained/lost/infeasible coverage, alternative
  selection, supplier search, report and family downloads, keyboard access and a 390 px
  viewport. Desktop and mobile renderings were inspected locally using installed Edge.
- `python -m build --no-isolation`: wheel and source distribution built. The installed
  wheel's `srl` entry point was exercised from a temporary directory with no checkout on
  its import path: audit, build, solve, enumeration, schema and independent verification
  of 1,284 distinct panels.

The CI workflow repeats Python and installed-wheel checks on Windows and hosted Ubuntu,
and browser checks in Chromium. Pages deployment follows successful CI for that revision.

Not measured: a new timed main-suite campaign, operational cost/capacity/delivery outcomes,
longitudinal supplier identity, or real correlated disruptions. The paired outages are
labelled hypothetical stress tests. There is no claim of procurement adoption.

## Outcome explorer (2026-10-15)

Source: `python analysis/build_site.py` at `4a4b4ca` (clean source), frozen QBL and strict
mappings, `results/suite-seed1.json`. All 88 saved records were rebuilt from the matrix:
every saved minimum is certified minimal at its saved size, and every essential set and
infeasibility witness equals its recomputation. 84 records carry exact counts; 4 carry
lower bounds (every route stopped at 10 s). Spearman rank correlation between eligible
suppliers and minimal panels over the 55 feasible instances: 0.75.

Checks:

- `python -m pytest -q`: **84 passed**; `tests/test_outcomes.py` checks the export against
  the saved records, rejects an invalid saved minimum and matches the published data.
- `python -m ruff check .`: **All checks passed!** `python -m pip check`: **No broken
  requirements found.**
- `python analysis/check_browser.py`: the previous checks, plus 88 rows, 55 plotted
  instances, outcome and product filters, an empty filter, record selection, filtered CSV
  (88 rows) and record JSON downloads. Desktop and 390 px renderings inspected in Edge.

Not measured: no timing was rerun; the explorer shows the historical seed-1 runs.
