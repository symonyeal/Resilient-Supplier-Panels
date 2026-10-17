# Resilient-Supplier-Panels: design

Approved in conversation 2026-10-15. Scope: Stages 0–2. Stage 3 and any site are out of scope.

## Decision

Which inclusion-minimal panels of USDA AMS qualified suppliers cover a product basket, and
which still cover it after any specified supplier failures? This is qualification coverage
only. It says nothing about capacity, price, lead time, willingness to bid or continuity of
supply.

## Data (Stage 0, measured on the snapshot of 2026-10-15)

`QBL.xlsx`, retrieved 2026-10-15T07:16:47Z, SHA-256
`b7b8414a821ab02b3a272d3f1c6173c0bcbbf55ac8a4c7d458114e5218b28822`, 113,416 B. HTTP
Last-Modified is 2026-10-02; the workbook states no date. It has one sheet `QBL` with 983 data
rows and the columns Vendor Name, Billing Address, City, State, ZIP Code, Size, Product.

- **Product cells.** 346 are multiline. Normalising whitespace and case leaves 722 tokens,
  drawn from four vocabularies:
  - USDA Foods material descriptions;
  - program codes (`WF - Wheat Flour`);
  - NAICS-prefixed commodity codes (`311999 DEB …`);
  - material numbers (`100193 - …`).
- **Broad categories** also occur (`TEFAP Fresh Produce Box`, 144 suppliers).
- **Suppliers per token.** 384 tokens have one supplier; 206 have three or more.
- **Symmetry.** There are 508 distinct qualification signatures, and 106 of them are shared by
  581 suppliers.

Consequences:
- The primary matrix uses one exact normalised token per qualification.
- Tokens are grouped into granularity classes by an explicit and auditable rule.
- Baskets are drawn within one class.
- Mappings across vocabularies or classes are a labelled sensitivity variant.

## Model

Let V be the suppliers, P a basket, E_p ⊆ V the suppliers qualified for p, and F a family of
failed-supplier sets with ∅ ∈ F.

- A panel T is feasible iff T ∩ (E_p \ F) ≠ ∅ for all p ∈ P and all F ∈ F.
- Let H = {E_p \ F}. Then Tr(H) is exactly the set of inclusion-minimal feasible panels.
- For the uniform model "any k failures", feasibility is equivalent to |T ∩ E_p| ≥ k + 1 for
  all p.

Proofs and edge cases are in `docs/MODEL.md`. Reductions used:
- deduplication and removing superset edges, which preserve Tr exactly;
- restriction to the support, which preserves Tr exactly.

No dominance reduction is used.

## Package `srl`

| Module | Interface |
| --- | --- |
| `acquire` | `fetch(url, dir) -> manifest entry` (network; never imported by tests) |
| `qbl` | `read(path) -> rows`; `records(rows) -> (suppliers, qualifications, issues)` |
| `instance` | `Instance(V, P, E, k \| F)`; `basket(...)`; `hypergraph(I) -> (H, provenance)` |
| `solve` | `milp(I)`; `exhaustive(I)`; `enumerate(I, method, timeout, cap)` with methods berge, fkb, milp |
| `certify` | `feasible`, `witnesses`, `essential`, `infeasibility`, `complete` (all from the matrix, not H) |
| `bench` | frozen suite, process-isolated runs, result JSON |
| `report` | findings tables from saved JSON only |
| `cli` | `python -m srl {audit,build,solve,enum,verify,bench,report}`; `acquire` is separate |

Thesis integration:
- **Dependency.** `fka` (GPL-3.0-or-later) is pinned at `1bb9de48ab91629a7ae9d97488c2181b4098806d`
  as a pip git dependency, and this project is GPL-3.0-or-later.
- **Enumeration.** `fka.transversal.transversal` (Berge) and FK-B dualisation. FK-B is driven
  through an incremental generator over the public `fkb.dualize` steps; a test asserts that
  it equals `dualise`.
- **Duality decision.** `fka.algorithm.fk_a` and `fka.memo.fk_a_memo` certify C = Tr(H) and
  measure the tree against the memoised DAG.

## Verification

Each level is independent of the producing route:
1. Per-panel feasibility and a minimality witness, read from the matrix.
2. Essentiality: v is essential iff V \ {v} is infeasible, with a product witness.
3. Infeasibility witness: a product p with |E_p| ≤ k.
4. Exhaustive agreement on supports of at most 18.
5. Agreement between Berge, FK-B and MILP enumeration.
6. An FK-A duality certificate.
7. MILP exhaustion: no feasible T contains none of the reported panels. This is a separate
   cardinality encoding, reported as a solver result, not a proof.

## Tests (public interfaces)

1. Workbook → records and provenance.
2. Records → instance.
3. Instance → panels.
4. Panel → certificate.
5. Benchmark result → report: complete and partial runs stay distinct, and timeouts are
   censored.

## Benchmarks

- **Basket sizes.** 3, 5, 8, 12 and 20, with k ∈ {0, 1}.
- **Selection rules,** applied within each class:
  - `uniform`: s products drawn at random;
  - `overlap`: greedy maximum Jaccard overlap from a seeded start;
  - `scarce`: products with at most 2 suppliers.
- **Seeds.** Seed 0 is the calibration set; seed 1 is the frozen main suite.
- **Limits.** 60 s and 10,000 panels per run, and about 15 minutes per campaign. Limits change
  only after calibration, and any change is recorded.
- **Timing.** Construction, solve, certification and report time are measured separately.
  Time to first output, time to K outputs, total time, output count and termination status
  are recorded for each run.

## Hypotheses

H1–H5 and kill conditions are fixed in `docs/findings/hypotheses.md` before any timing.

Rival: the MILP alone answers the procurement questions, and enumeration adds little.

## Out of scope

Prices, capacity, award reconciliation, parent-company scenarios, a dashboard, and Rust.
