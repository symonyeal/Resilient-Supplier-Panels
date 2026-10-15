# Findings: resilient qualified-supplier panels on the USDA QBL (2026-10-15)

## Governing thought

On the public USDA qualification data, the decision-relevant answers are cheap and exact,
and enumeration adds alternatives only where single-qualified products do not decide the
case:
- A one-line matrix witness and a millisecond MILP give the minimum panel, whether it
  survives one supplier failure, and which suppliers are essential.
- Exact enumeration adds real alternative panels where the basket overlaps.
- Berge is the practical complete enumerator here. FK-B is useful only for fast first
  outputs. Thesis memoisation saves a median of 1.44× in tree nodes.

**Confidence: medium.**
- The mathematics is certified on every instance, with no failed check.
- External validity is limited:
  - the population is 88 instances from one snapshot;
  - n = 9 for the k = 1 alternatives;
  - granularity is judged per token;
  - qualification is not capacity.

**What would change the answer:** a richer qualification feed in which most products have
at least 2 qualified suppliers, or capacity data that turns coverage into allocation.

**Source.**
- Snapshot `b7b8414a…` (`data/manifest.json`).
- Main suite seed 1, code `09cb123` (clean), thesis `1bb9de4`.
- 88 instances: 4 granularity classes × {uniform, overlap} × sizes {3, 5, 8, 12, 20}, plus
  4 scarce baskets, each at k ∈ {0, 1}.
- Limits 10 s and 10,000 panels per run; wall time 548 s.
- Full tables: [results-seed1.md](results-seed1.md). Protocol: [BENCHMARKS](../BENCHMARKS.md).

## 1. Resilience to any single supplier failure is mostly decided by single-qualified products

| Measure (primary baskets) | Value |
| --- | --- |
| k = 1 feasible, all sizes | 10 of 40 |
| k = 1 feasible, size ≤ 8 (H2) | 10 of 24 (41.7%) |
| k = 1 feasible, sizes 12 and 20 | 0 of 16 |
| extra suppliers needed when k = 1 is feasible | median 2, range 1–3 |
| extra suppliers, range of k = 0 → k = 1 minimum | 1 → 2 up to 3 → 6 |

- Every infeasible basket has a product with exactly one qualified supplier, read directly
  off the matrix. HiGHS agreed with that witness on 88 of 88 instances.
- 217 of 451 precise products (48.1%) have exactly one qualified supplier (`docs/DATA.md`).
  A random basket of 8 therefore almost surely contains one.
- 9 of the 10 feasible primary k = 1 baskets come from the `overlap` rule.

**Operational reading.** The binding constraint is the qualification register, not panel
selection. The products that block backup coverage are named in the table in
[results-seed1.md](results-seed1.md#k--1-is-feasible-for-10-of-40-primary-baskets-where-feasible-it-adds-a-median-of-20-suppliers-range-13).
These are the products on which qualifying a second vendor would buy resilience.

## 2. Where choice exists, there are real alternatives, and essential suppliers are few

| Measure | Value |
| --- | --- |
| feasible primary cases with an exact count | 46 of 50 (4 censored) |
| … with at least 2 minimal panels (H3) | 41 of 46 |
| median number of minimal panels (K4) | 11 |
| essential set strictly smaller than the minimum panel | 45 of 50 |
| k = 1 cases with at least 2 panels up to signature symmetry | 5 of 9 |
| largest complete family | 10,406 panels (programme, uniform, size 5, k = 0) |

- Essential suppliers come from the matrix witness |E_p| = k + 1. They need no enumeration.
  They equal the intersection of the complete family on 50 of 50 checked instances.
- Signature symmetry compresses the families heavily:
  - 3,600 labelled panels form 28 classes (description, uniform, size 20);
  - 1,704 form 8 (programme, overlap, size 5, k = 1).

  Many alternatives are relabellings of interchangeable vendors with identical
  qualifications. They are distinct legal entities, but they are not structurally
  different choices.

## 3. Berge is the practical complete enumerator; FK-B only gives early output

| Route | Runs complete within 10 s | Remark |
| --- | --- | --- |
| Berge (thesis oracle) | 84 / 88 | up to 10,406 panels in 6.7 s; output arrives only at the end |
| FK-B (thesis, incremental) | 76 / 88 | never completed a family beyond 240 panels; first panel in ≤ 1.2 ms on every instance |
| MILP enumeration (HiGHS) | 76 / 88 | in nondecreasing size; first panel in median 2.9 ms (max 11 ms) |

- FK-B finished before Berge on 21 instances. On all 21, Berge itself finished in at most
  0.7 ms.
- On the 11 feasible instances where Berge took more than 0.1 s, FK-B's first panel came
  first on 11 of 11. Its full family never completed on any of them.
- The single minimum (MILP) took a median of 0.8 ms and at most 8.9 ms.
- Primary runs stopped by the limit (K5): 28 of 240. Those counts are lower bounds.

## 4. Thesis memoisation helps modestly on real structure

- FK-A certified C = Tr(H) on 46 of 46 families of at most 2,000 panels, and `fk_a_memo`
  agreed on all 46.
- Tree nodes over distinct subproblems: median 1.44, at least 2 on 15 of 46, and at most
  8.5 (595 → 70; programme, overlap, size 5, k = 1). Wall time was 1.38 s for the tree
  against 0.88 s with memoisation, summed over the 46.
- H4's memo threshold (≥ 2× on at least half of the instances) fails. H4's symmetry
  threshold holds: compression of at least 2× on 5 of 9 k = 1 families.
- Symmetry here is output compression. No symmetry-aware enumerator was run, so no
  computational saving from symmetry is claimed.

## Pre-registered tests

| Test | Threshold | Result | Verdict |
| --- | --- | --- | --- |
| H1 overlap | ≥ 25% of precise products have ≥ 3 suppliers | 34.8% | holds |
| H2 resilience | ≥ 50% of size-≤ 8 baskets k = 1 feasible | 41.7% | **fails** |
| H3 alternatives | \|Tr\| ≥ 2 on ≥ 50% of feasible cases | 41/46; essential ⊊ minimum on 45/50 | holds |
| H4 memoisation | tree/DAG ≥ 2 on ≥ half | 15/46, median 1.44 | **fails** |
| H4 symmetry | compression ≥ 2 on k = 1 | 5/9 | holds |
| H5 robustness | H2 and H3 verdicts unchanged under `merged` | H2 numerator 10 → 12 of 24 (50%) | **fails narrowly**: H2 crosses its threshold |
| Day-1 answer | ≥ 2 materially different k = 1 panels on ≥ half of the feasible baskets | 5/9 | holds (n = 9) |
| Day-1 answer | FK-B no faster than Berge anywhere | faster on 21/88, all with Berge ≤ 0.7 ms | **revised**: wins only on trivial instances |

| Kill condition | Fires if | Result | Fired |
| --- | --- | --- | --- |
| K1 normalisation unreliable | > 50% of incidences unclassified | 29.6% unclassified | no |
| K2 overlap only from broad categories | H1 fails on precise products | H1 measured on precise products | no |
| K3 robust baskets mostly infeasible | < 25% k = 1 feasible | 41.7% | no |
| K4 trivial choices | median \|Tr\| = 1 | 11 | no |
| K5 output overwhelms enumeration | > 50% of primary runs stopped | 28/240 | no |
| K6 thesis methods always worse | FK-B slower than the faster baseline on every instance | 63/88 | no (but see §3) |
| K7 identity-dependent conclusions | identity merge changes the H2 or H3 verdict | H2: 10 → 8 of 24, still fails; 2 feasibility flips; H3 not measured under variants | no for H2 |

**Sensitivity details.**
- Merging programme codes and named equivalents (`merged`) turns 4 infeasible primary cases
  feasible and changes the minimum in 8 more.
- Merging the 31 identity candidates turns 2 feasible k = 1 cases infeasible. Two "different"
  suppliers with the same name stop counting as backup.
- Both directions are as expected, and both are reported, not chosen between. Counts of
  minimal panels under the variants were not measured.

**Deviation from pre-registration.** K7 was written as "dropping" identity candidates. The
run merges each candidate into its match, which is the interpretation that bears on
redundancy. Dropping would delete qualifications.

## Rival explanation

The rival held that a single MILP answers the procurement questions. **It is mostly right.**

What the MILP and the matrix give at millisecond cost:
- the minimum panel;
- the k = 1 increase;
- infeasibility, with the blocking product;
- the essential suppliers.

What enumeration adds:
- the alternatives (a median of 11 minimal panels);
- the symmetry structure, which shows that most alternatives are interchangeable vendors.

The rival's condition (b), "alternatives differ only by interchangeable suppliers", holds
for the largest families: 3,600 panels form 28 classes. **Recommended practice:** use the
MILP and matrix witnesses as the default; enumerate with Berge, reported up to signature
symmetry, when the alternatives matter; use FK-B only when a first panel is wanted from a
family too large to finish.

## Limits on these results

- **Qualification is not supply.** Capacity, inventory, willingness to bid, price, lead time
  and plant location are all absent. A QBL vendor may be a nonmanufacturer using
  subcontractors (qualification requirements, §II).
- **Billing location is not operating location.** No geographic or upstream independence is
  modelled.
- **Granularity is per token.** 228 commodity and 31 broad tokens are excluded from the
  precise baskets, and 10 mappings in `merged` are judgement calls.
- **One current snapshot.** There is no history, so persistence of coverage over time is
  unmeasured.
- **Censoring.** 4 feasible primary cases have unknown counts. Censored runs give lower
  bounds only.
- **Not measured:** memory; limits longer than 10 s; any symmetry-aware enumeration;
  specialist enumerators.
