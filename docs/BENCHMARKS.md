# Benchmarks

The protocol is fixed before the main suite runs. `srl/bench.py` implements it.

## Instances

All instances come from the strict matrix of the frozen snapshot
(`data/manifest.json`, SHA-256 `b7b8414a…8822`). Every eligible supplier of a selected product
is retained, so no instance is a restricted candidate universe.

- **Classes:** `description`, `program`, `naics`, `material` (`docs/DATA.md`).
- **Selection rules:** for each class and each size s ∈ {3, 5, 8, 12, 20} that the class can
  supply:
  - `uniform`: s products drawn uniformly without replacement;
  - `overlap`: start at a uniformly drawn product with at least 2 suppliers, then repeatedly
    add the product whose supplier set has the greatest Jaccard similarity to the union so
    far. Candidates are taken in seeded random order; `max` keeps the first maximum.
  - `scarce`: 3 products, each with at most 2 suppliers. These include natural infeasible
    cases.
- **Failure models:** k = 0 and k = 1 (any single supplier fails).
- **Primary baskets** (the population for H2 and H3): `uniform` ∪ `overlap`.
- **Scalability cases:** sizes 12 and 20.
- **Tiny instances:** support ≤ 16, which are also checked by exhaustive search.
- **Synthetic fixtures** live in `tests/` only and support no empirical claim.
- **Seeds:** seed 0 is calibration and is never reported as a finding; seed 1 is the main
  suite.

## Methods

Every route answers the same task, namely enumerating Tr(H) or producing its first K members:

| Route | Code | Output |
| --- | --- | --- |
| Berge | `fka.transversal.transversal` (thesis oracle) | all at the end |
| FK-B | `fkb` self-reduction, one equivalence test per panel | incremental |
| MILP enumeration | HiGHS, minimum avoiding supersets of found panels | incremental, nondecreasing size |
| Exhaustive | all subsets of the support, checked against the matrix | small supports only |

MILP enumeration is complete. Each optimum is minimal: a proper feasible subset would cost
less and still avoid every cut. Infeasibility of the cut problem means every feasible panel
contains a found panel.

The single minimum (`solve.milp`) is reported separately. It is not an enumeration baseline.

FK-A and `fk_a_memo` are not enumerators. They decide C = Tr(H) for the complete family C,
which gives a duality certificate, and they measure tree nodes against distinct
subproblems (H4).

## Measurements

| Quantity | Meaning |
| --- | --- |
| `t_build` | scenario hypergraph and reductions |
| `t_milp_min` | one minimum-cardinality solve |
| `t_first`, `t_K` | time to the first and the K-th (K = 10) output, from the child's own clock |
| `t_total` | total time to completion (complete runs only) |
| `t_certify`, `t_exhaustion` | certificate time, separate from solve time |
| `count` | panels received (exact only when the run is complete) |
| `stored` | panels kept (at most the cap) |
| `status` | `complete`, `timeout` or `cap` |

- A timeout is a censored observation.
- Equal counts before a timeout establish nothing about the families.
- A complete run whose total is under 1 s is repeated three times in all, and the spread is
  reported.
- Memory is not measured.
- Every enumeration runs in a fresh process, which is terminated at the limit.

## Limits and calibration

The initial limits were 60 s and 10,000 panels per run, and a campaign of about 15 minutes.

1. Calibration (seed 0) uses a 10 s limit and measures the share of runs that stop. Its
   result and the main-suite limits are recorded below before the main suite runs.
2. Limits change only here.

### Calibration record

Recorded 2026-10-15, before seed 1 ran. The log is `results/calibration-seed0.log`.

**Run.** Seed 0, 10 s limit, sensitivity reruns on. I stopped it after 10.5 min, by which
point it had covered 35 instances.
- Berge completed 29 of 35 runs, FK-B 27 and MILP enumeration 27. 11 of the 35 instances
  were infeasible, every one of them at k = 1.
- Timeouts occurred only in uniform baskets of size ≥ 12 and in programme baskets, whose
  E_p are largest (up to 50 eligible suppliers at size 3).
- The protocol cost about 18 s per instance. At 60 s the suite of 88 instances could not
  meet the 15-minute budget.

**Main-suite limits.**
- **Time limit:** 10 s per enumeration run. The budget forces this. Every run that stops is
  censored, and whether a 60 s limit would complete it is not measured.
- **Output cap:** 10,000 panels.
- **FK-A certificate:** only for complete families of at most 2,000 panels, within 200,000
  tree nodes. A larger family is reported as not FK-A-certified.
- **Sensitivity variants** (merged products; identity candidates merged into their match):
  one HiGHS solve each, giving feasibility and the minimum. Counts under the variants are
  not measured.
- **Repeats** of runs under 1 s happen in-process, which removes about 1 s of process start
  per repeat. The first timing of each run is still process-isolated.
