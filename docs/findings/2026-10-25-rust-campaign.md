# Findings: the release 0.3 campaign (2026-10-25)

## Governing thought

The 0.2 findings stand, and the rerun removes their one censoring:
- Every one of the 88 families now has an exact count. The four that every 0.2 route left
  unfinished have 13,155, 14,700, 20,478 and 10,978 minimal panels.
- What blocks resilience is unchanged: 30 of 40 primary baskets fail at k = 1, each through
  a product with one qualified supplier.
- Compiled Berge finishes every family in at most 12 ms. FK-B is therefore never the
  fastest complete route, and kill condition K6 fires (88 of 88; 63 of 88 in 0.2).
- A counting theorem over supplier signature classes (MODEL, Theorem 4) gives every
  count without listing the panels, and agrees with enumeration on all 88 instances.

**Confidence: high for the mathematics, unchanged for external validity.** Every
certificate passed on every instance, and the Rust port reproduces the thesis code's
outputs exactly (VALIDATION, Release 0.3). The population is still 88 instances from one
snapshot, and qualification is still not capacity.

**Source.** Snapshot `b7b8414a…`; campaign `results/suite-seed1-v0.3.json`, code `55e8e4a`
(clean); thesis algorithms ported from `1bb9de4`. Same baskets and limits as 0.2
([protocol](../BENCHMARKS.md#release-03-campaign-rust)); wall time 168 s (0.2: 548 s).
Full tables: [results-seed1-v0.3.md](results-seed1-v0.3.md).

## 1. What did not change

On all 88 instances the minimum size and status, the infeasibility witness, the essential
suppliers and their witnesses, both sensitivity verdicts, the number of panels up to
signature symmetry, and the FK-A tree and DAG node counts equal the 0.2 records. The
findings of 0.2 §1 (single-qualified products decide resilience) and §4 (memoisation
saves a median 1.44× in tree nodes) are unaffected.

## 2. Exact counts where 0.2 had lower bounds

| Basket (class/rule/size) | k | Eligible suppliers | 0.2 lower bound | Exact count | Up to symmetry | Berge time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| program/overlap/8 | 0 | 82 | 411 | 13,155 | 88 | 3.4 ms |
| program/overlap/20 | 0 | 87 | 309 | 14,700 | 54 | 12.2 ms |
| naics/uniform/8 | 0 | 71 | 678 | 20,478 | 12 | 9.4 ms |
| naics/overlap/8 | 1 | 68 | 439 | 10,978 | 253 | 6.2 ms |

The lower bounds were 2–4% of the true families. The largest family, 20,478 panels, is
12 classes of interchangeable vendors.

Consequences for the registered tests:
- **H3 holds on the full population:** 45 of 50 feasible primary cases have at least two
  minimal panels (0.2: 41 of 46, with 4 unknown).
- **K4:** the median number of minimal panels over feasible primary cases is 12 (0.2: 11
  over the 46 then known).
- **H4 symmetry holds:** compression of at least 2× on 6 of 10 resilient k = 1 families
  (0.2: 5 of 9). The added family compresses 43× (10,978 panels, 253 class vectors).

## 3. Routes

| Route | Complete within 10 s | Slowest complete run | Median first panel |
| --- | ---: | ---: | ---: |
| Berge (thesis oracle) | 88 / 88 | 12 ms | 11 µs |
| MMCS | 83 / 88 (5 at the 10,000 cap) | 1.6 ms | 6.8 µs |
| Cut enumeration | 83 / 88 (5 timeouts) | 9.9 s | 2.6 µs |
| FK-B (thesis, incremental) | 79 / 88 (9 timeouts) | 3.9 s | 7.5 µs |

- The five MMCS stops are the five families above 10,000 panels; MMCS stopped at the cap,
  not the clock.
- FK-B finishes before Berge on 29 instances, all with Berge under 0.1 ms. On no
  instance is it faster than the faster of Berge and the cut route, so **K6 fires**: as an
  enumerator, the thesis's FK-B is less practical than the baselines on these instances.
  The thesis code's value here is certification, not speed: FK-A decides C = Tr(H) on 46
  of 46 families, and FK-B's stream checks the hypergraph against the other routes.
- The minimum takes a median of 2.0 µs and at most 25 µs. The 0.2 rival held that the
  single minimum answers most procurement questions; at this cost it does, and
  enumeration remains the tool only when the alternatives matter.

The 0.2 and 0.3 timings are of different implementations on the same laptop. They
measure this Python–Rust pair, not the algorithms.

## 4. Counting without listing

The class count reduces each instance to at most k + 1 members per signature class and
enumerates that. It agrees with the enumerated count on 88 of 88 instances and, where the
family is stored, with its class vectors on 83 of 83. The largest reduced family has 852
panels; the slowest count took 0.44 ms. Families far beyond listing size, such as
whole-programme baskets, are now countable exactly (`srl count`).

## Pre-registered tests, 0.2 and 0.3

| Test | 0.2 | 0.3 | Verdict in 0.3 |
| --- | --- | --- | --- |
| H1 overlap | 34.8% | 34.8% | holds |
| H2 resilience, size ≤ 8 | 10/24 | 10/24 | **fails** |
| H3 alternatives | 41/46, 4 unknown | 45/50 | holds |
| H4 memoisation, tree/DAG ≥ 2 | 15/46 | 15/46 | **fails** |
| H4 symmetry, k = 1 | 5/9 | 6/10 | holds |
| H5 robustness (`merged`) | H2 numerator 10 → 12 | same | **fails narrowly** |
| K4 median \|Tr\| | 11 | 12 | does not fire |
| K5 primary runs stopped | 28/240 | 19/320 | does not fire |
| K6 FK-B slower than the faster baseline everywhere | 63/88 | 88/88 | **fires** |

## Limits

As in 0.2, less the censoring: qualification is not supply; billing location is not
operating location; granularity is per token; one snapshot; one timed run per instance;
memory not measured. The sensitivity variants still measure feasibility and the minimum
only.
