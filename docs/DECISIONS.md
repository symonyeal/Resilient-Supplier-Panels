# Decisions

Each entry gives the decision, the alternatives, the evidence and the date.

## Taken

**Thesis integration is a pinned pip dependency, and the licence is GPL-3.0-or-later**
(2026-10-15).
- `fka` 2.0.0 at `1bb9de4` is GPL-3.0-or-later. This project imports it, so it is
  distributed under the same licence, with attribution in the README and `docs/SOURCES.md`.
- *Alternatives:* vendoring the solver, which would diverge from the published reproduction
  tests; reimplementing it, which would not test the thesis code at all.

**FK-B enumeration runs through an incremental adapter** (2026-10-15).
- `fkb.dualize.dualise` returns only at the end. `srl.solve.fkb_stream` restates its loop
  (initial guess, FK-B equivalence test, maximal false point, new clause) from the package's
  public functions and yields each panel as it is found.
- `tests/test_routes.py::test_fkb_stream_equals_thesis_dualise` asserts that the two routes
  give equal families.
- This adapter is needed for time-to-first-K and for partial output under a time limit.

**Strict mapping is exact-token, and baskets are drawn within one granularity class**
(2026-10-15).
- The workbook mixes four vocabularies (`docs/DATA.md`). Equating them requires judgement,
  so cross-vocabulary equivalence appears only in the `merged` sensitivity variant.

**Stdlib xlsx reader** (2026-10-15).
- The workbook has one sheet of shared strings and numbers. About 30 lines of `zipfile` and
  ElementTree read it, which avoids a dependency. openpyxl was used once, in discovery, as an
  independent reader; `tests/test_qbl.py` holds the literals it gave.

**Essentiality by matrix witness, not by enumeration** (2026-10-15).
- v is essential iff V \ {v} is infeasible (MODEL, Theorem 3.2). This is O(Σ|E_p|), it is
  exact, and it does not depend on any enumeration completing.

**Enumeration exhaustion must retain the solver's stopping status** (2026-10-15).
- A generator ending does not establish that HiGHS proved infeasibility. `milp_stream`
  returns its status; the worker propagates it. Only infeasibility of the cut problem
  produces `complete`. Unresolved sensitivity feasibility and solver/witness agreement
  are `None`, rather than false verdicts.
- Completeness is checked only after every supplied panel is certified minimal. An
  invalid panel must never become an exclusion cut that falsely proves exhaustion.
- The maintenance recheck is recorded in [VALIDATION](VALIDATION.md). Historical suite
  timings and metadata retain their original revision; no timed suite was rerun.

## Tested and refused

**Specialist enumerators (SHD, RS; Murakami–Uno)** (2026-10-15).
- Not included. No maintained Windows build was verified in this session.
- The suite's question is whether thesis methods are practical against a baseline that is
  easy to reproduce, and Berge and MILP enumeration serve that.
- Open for Stage 3.

**Dashboard** (2026-10-15).
- Deferred until verified findings exist.
- Superseded by release 0.2.0 after the findings and independent recheck. The public site
  presents precomputed certified cases; it runs no solver and asserts no delivery outcomes.

## Portfolio release 0.2.0

**A static public case study, with the existing solver as its checked core** (2026-10-15).
- The industry-facing portfolio needs an inspectable decision and reproducible evidence.
  Three small real baskets show alternatives, forced backup suppliers and infeasibility.
- Browser interactions select certified outage results; they do not restate the model in
  JavaScript. No backend, accounts or external JavaScript libraries are needed.
- Source hashes use LF-normalized Python. The frozen workbook and mapping bytes are
  hashed exactly. Build provenance distinguishes new case generation from historical
  benchmark timings.
- The wheel maps the canonical data directory into package resources at build time;
  there is one tracked copy of each frozen input. Runtime output paths belong to the caller.
- Versioned JSON documents retain legacy-instance compatibility. Input errors, solver
  outcomes, enumeration limits and certificate failures remain distinct.
- Pages deployment follows successful Windows, hosted Ubuntu and browser checks for the
  same revision. Release wheels are exercised outside the checkout before publication.

**Every saved outcome is public, re-certified rather than rerun** (2026-10-15).
- The site lists all 88 seed-1 records, not only summaries, so a reader can inspect any
  instance behind a headline number. Timings remain the historical measurements.
- `portfolio.outcome` rebuilds each basket from the frozen matrix and refuses export when a
  saved minimum is not certified minimal, or the essential set or infeasibility witness
  differs. A record without a complete run exports `max_r count_r` as a lower bound.
- The rank correlation shown (Spearman, eligible suppliers against minimal panels) is
  computed at build time over feasible instances, partial runs at their bound; the page
  computes nothing the model computes.
- Plotly basic 4.1.1 (MIT) is vendored, not loaded from a CDN, so the page keeps working
  offline and from `file://`, as the static-site decision requires. SHA-256
  `99784e3abfb07b142a15e521a439e6d9addceadb3f79553fd24c72774f0649ba`.
