# Decisions

Each entry gives the decision, the alternatives, the evidence and the date.

## Release 0.3 (Rust)

**The whole program is Rust; the Python package is retired** (2026-10-25).
- One binary carries the frozen data and every command. Rust gives the speed that settles
  the censored families and a single install step without Python, SciPy or Git.
- *Alternative:* Rust kernels bound into the Python package (PyO3). Rejected: the CLI is
  the public interface, and two languages would double the gate.
- The last Python revision is `99cd50c` (release `v0.2.0` is `9dc3544`); git history is
  its archive.

**The thesis algorithms are ported line for line, not imported** (2026-10-25).
- `src/thesis/` follows `fka`/`fkb` at `1bb9de4`: canonical term order, tie-breaks,
  conditions and the dualisation loop. The evidence is exact agreement with the thesis code
  on 300 random instances and the 88 seed-1 instances: Berge families, FK-B output order and
  node counts per pass, FK-A tree and DAG node counts, and the conflicting assignment of a
  non-dual pair. `tests/thesis.rs` keeps that agreement in CI from `tests/reference/`.
- A planted change of FK-A's tie-break (last maximal variable instead of the first) makes
  that test report 110 differences.
- *Alternative:* keep calling the Python package. Rejected with the Python package.

**HiGHS is replaced by an exact branch and bound, and the completeness check by an
exhaustive search** (2026-10-25).
- The minimum and the cut route solve the same 0-1 program as before. The minimum takes a
  median of 2 µs on the suite against HiGHS's 0.8 ms in 0.2, and the minima, statuses and
  infeasibility verdicts agree on all 88 instances and both sensitivity variants.
- The completeness search is written apart from the solver and proves exhaustion
  (MODEL, Theorem 3.3), where the 0.2 HiGHS search gave solver assurance.
- *Alternative:* the `highs` crate. Rejected: it needs CMake and a C++ build, and the
  problems here are small covering programs.

**Counting by signature classes** (2026-10-25).
- MODEL Theorem 4 counts minimal panels from the class vectors of a reduced instance with
  at most k + 1 members per class. It agrees with enumeration on all 88 instances and
  makes families of any size countable; `srl count` exposes it.

**The shortlist starts from the first optimum in label order** (2026-10-25).
- 0.2 started from whichever optimum HiGHS returned, so the published shortlist depended on
  the solver. With HiGHS's choices fed in, the 0.3 shortlists equal the 0.2 ones exactly;
  the deterministic choice changes the first panel in 4 of 6 case modes, and with it the
  canned basket's two-supplier stress pair. Counts, families and certificates are unchanged.

**Outputs keep the 0.2 bytes where they are deterministic** (2026-10-25).
- JSON is written as Python wrote it (one-space indent, ASCII escapes, float repr), and
  the 0.2 report rebuilds byte for byte except the line naming the command. Supplier
  witness maps are sorted by supplier; 0.2 printed them in Python's hash order.

## Taken

**Thesis integration is a pinned pip dependency, and the licence is GPL-3.0-or-later**
(2026-10-15; superseded in 0.3 by the Rust port, below; the licence stands).
- `fka` 2.0.0 at `1bb9de4` is GPL-3.0-or-later. This project imports it, so it is
  distributed under the same licence, with attribution in the README and `docs/SOURCES.md`.
- *Alternatives:* vendoring the solver, which would diverge from the published reproduction
  tests; reimplementing it, which would not test the thesis code at all.

**FK-B enumeration runs through an incremental adapter** (2026-10-15; kept in 0.3 as
`thesis::fkb::dualise`).
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

**Stdlib xlsx reader** (2026-10-15; in 0.3 a zip-directory reader with miniz_oxide and
roxmltree, same output on every row).
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

**Specialist enumerators (SHD, RS; Murakami–Uno)** (2026-10-15; MMCS added in 0.3).
- Not included in 0.2: no maintained Windows build was verified on 2026-10-15.
- The suite's question is whether thesis methods are practical against a baseline that is
  easy to reproduce, and Berge and MILP enumeration serve that.
- 0.3 adds MMCS, written from the published algorithm and checked against Berge; SHD and
  RS remain out.

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
