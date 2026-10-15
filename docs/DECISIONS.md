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

## Tested and refused

**Specialist enumerators (SHD, RS; Murakami–Uno)** (2026-10-15).
- Not included. No maintained Windows build was verified in this session.
- The suite's question is whether thesis methods are practical against a baseline that is
  easy to reproduce, and Berge and MILP enumeration serve that.
- Open for Stage 3.

**Dashboard** (2026-10-15).
- Deferred until verified findings exist.
