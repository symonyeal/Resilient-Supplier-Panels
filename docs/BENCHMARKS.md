# Benchmarks

The protocol is fixed before the main suite runs. `src/bench.rs` implements it; release
0.2 ran it in Python (`srl/bench.py` at `99cd50c`).

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
- **Tiny instances:** support ≤ 18, which are also checked by exhaustive search (the spec's
  bound; an earlier draft of the code used 16).
- **Synthetic fixtures** live in `tests/` only and support no empirical claim.
- **Seeds:** seed 0 is calibration and is never reported as a finding; seed 1 is the main
  suite.

## Methods

Every route answers the same task, namely enumerating Tr(H) or producing its first K members:

| Route | Code | Output |
| --- | --- | --- |
| Berge | `thesis::berge` (thesis oracle; `fka.transversal` in 0.2) | all at the end |
| FK-B | `thesis::fkb::dualise`, one equivalence test per panel | incremental |
| Cut enumeration (`cuts`; `milp` in 0.2) | minimum avoiding supersets of found panels: branch and bound (HiGHS in 0.2) | incremental, nondecreasing size |
| MMCS (0.3) | `mmcs`, Murakami–Uno minimal hitting sets | incremental |
| Exhaustive | all subsets of the support, checked against the matrix | small supports only |

The class count (`quotient`, 0.3) is not an enumerator: it counts Tr(H) exactly from
signature classes (MODEL, Theorem 4).

Cut enumeration is complete. Each optimum is minimal: a proper feasible subset would cost
less and still avoid every cut. Infeasibility of the cut problem means every feasible panel
contains a found panel.

The single minimum (`solve::minimum`; `solve.milp` in 0.2) is reported separately. It is not
an enumeration baseline.

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

### Aborted first main run

A first seed-1 run on code `0b03a16` was stopped after 39 instances; its log is
`results/suite-seed1-aborted.log`. A review had found three defects in the harness:
- a crashed child was recorded as `timeout`;
- HiGHS statuses other than optimal or infeasible were read as optimal;
- the dirty flag counted untracked files.

None of these changes the limits, the basket selection or the seed. The main suite was rerun
whole on the corrected code, and only that rerun is reported.

### Superseded second main run

A complete seed-1 run on code `0156e5e` finished in 555 s; its log is
`results/suite-seed1-superseded.log`. Its tables showed that one material token held two
material numbers on one line (`100127 - beef can-24/24 oz 110477 - chicken pouch 36/10 oz`).
That token had been read as a single product with one supplier, so it falsely blocked k = 1
in five material baskets.

The token is now split in `mappings.csv`, by the same parse repair already applied to the
two programme-code pairs. `test_catalog.py` asserts that no such token remains. The limits,
the basket rules and the seed are unchanged. The suite was rerun whole, and only that run is
reported.

## Release 0.3 campaign (Rust)

Recorded 2026-10-25, before the run. The 0.2 measurements above stay as they are; this
campaign is a new file, `results/suite-seed1-v0.3.json`.

**Unchanged.** Seed 1 and the same 88 instances: the Rust port draws the baskets with
Python's Mersenne Twister and reproduces the 0.2 basket lists for seeds 0–5 exactly
(`tests/parity.rs`). Limits: 10 s and 10,000 panels per run, K = 10, FK-A certificate for
complete families of at most 2,000 panels within 200,000 tree nodes, exhaustive check for
supports of at most 18, three repeats of complete runs under 1 s, sensitivity variants by
one minimum solve each.

**Changed, with the reason.**
- **Routes.** `berge` and `fkb` are line-for-line ports of the thesis code (same output
  order, same FK-A/FK-B node counts on every reference instance). `cuts` replaces the HiGHS
  MILP enumeration: the same cut problem, solved exactly by branch and bound, because the
  port has no LP solver. `mmcs` (Murakami–Uno) is new: the specialist enumerator that
  `docs/OPEN.md` item 5 asked for.
- **Execution.** Each run is a thread with a deadline checked at every recursion step,
  not a process killed at the limit; a panic is `error`. Process start is no longer inside
  any measurement.
- **New limit.** Berge's working family is capped at 10,000,000 sets; reaching it ends the
  run with status `memory`, a censored observation like `timeout`.
- **New measurement.** `quotient`: the exact count of minimal panels by signature classes
  (MODEL, Theorem 4), under the same 10 s limit, compared with every complete family.
- **Certificates.** The minimum is solved exactly under the 10 s limit (status `timeout`
  if it does not finish). Completeness of a family is an exhaustive search over the
  matrix rather than a HiGHS solve; it has no time limit and its time is `t_exhaustion`.

Timings are comparable within this campaign only. The 0.2 Python timings and these were
taken on the same laptop but by different implementations; a speed-up between them is
reported as a measurement of this pair, not of the algorithms.
