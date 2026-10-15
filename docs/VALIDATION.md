# Validation

This file lists what must be checked, how, and what has been checked. A check is a
certificate (feasibility, a witness, agreement of independent routes, a completeness
search), never an output snapshot.

Assurance levels:
- **D**: direct certificate from the matrix.
- **X**: exhaustive agreement.
- **A**: agreement of independent routes.
- **S**: solver result (HiGHS), not a formal proof.
- **F**: FK-A duality decision (thesis code).

## Mathematics (tests, every commit)

| Claim | Check | Level | Status |
| --- | --- | --- | --- |
| Tr on a hand-worked instance: {ac, bc, bd}; k = 1 forces abcd | `test_panels.py` | hand | pass |
| Empty basket → {∅}; empty edge → infeasible with witness | `test_routes.py` | D | pass |
| Single-supplier product is essential and blocks k = 1 | `test_routes.py` | D, X | pass |
| Duplicate and superset edges removed; Tr unchanged; provenance kept | `test_routes.py` | X | pass |
| Identical signatures: interchangeable at k = 0, both needed at k = 1 | `test_routes.py` | X | pass |
| Correlated scenario F = {a, b} | `test_routes.py` | X | pass |
| Berge = FK-B stream = MILP enumeration = exhaustive on 60 random instances, k ∈ {0, 1, 2} | `test_routes.py` | X, A | pass |
| Every panel is minimal by matrix witness; essential set = intersection of Tr; MILP min = min size | `test_routes.py` | D, X | pass |
| Completeness search accepts Tr and returns the panel removed from it | `test_routes.py` | S | pass |
| Permutation invariance (suppliers and products reordered) | `test_routes.py` | X | pass |
| FK-B stream = thesis `dualise` | `test_routes.py` | A | pass |
| Timeout yields partial output, all certified; cap stops streams; Berge counts exactly | `test_bench.py` (slow) | D | pass |
| Report never prints a partial count as exact | `test_bench.py` | — | pass |

## Data (tests, every commit)

| Claim | Check | Status |
| --- | --- | --- |
| Frozen snapshot hash = manifest | `test_qbl.py` | pass |
| Reader matches an independent reader (openpyxl literals) on row 2 | `test_qbl.py` | pass |
| Multiline cells split; repeated rows merge; name collisions stay distinct | `test_qbl.py` | pass |
| Missing names or products reported, not dropped silently | `test_qbl.py` | pass |
| Stale-snapshot flag (> 90 days) | `test_qbl.py` | pass |
| Every snapshot token has a dictionary entry and one merged image | `test_catalog.py` | pass |

## Empirical results (per suite run)

Every instance record in `results/suite-seed*.json` carries:
- the minimum panel and a minimality witness for each member (D);
- either an infeasibility witness, agreeing with HiGHS, or the essential suppliers with
  their witnesses (D);
- for each route, every returned panel certified minimal (D);
- for complete families:
  - agreement between routes (A);
  - MILP exhaustion (S);
  - exhaustive agreement where the support is at most 18 (X);
  - an FK-A duality decision within 200,000 nodes (F);
  - the essential set equal to the intersection of the family (D, A).

### Seed 1 (code `09cb123`, clean; data hash = manifest; 88 instances)

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
