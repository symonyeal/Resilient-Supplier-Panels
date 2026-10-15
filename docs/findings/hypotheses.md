# Hypotheses (fixed 2026-10-15, before any timing)

**Question.** Does exact enumeration of minimal supplier panels tell a USDA procurement
analyst something that a single minimum-cardinality MILP does not? Here "the matrix" means
the strict QBL qualification matrix.

**Day-1 answer.**
- Yes for coverage that survives one failure (k = 1): on at least half of the feasible
  primary baskets there are at least 2 materially different minimal panels.
- The MILP remains the practical tool for finding the minimum.
- FK-B is not faster than Berge on any frozen-suite instance.

## Tree

```
Is enumeration useful beyond one MILP optimum?
├── H1  Overlap: does the matrix give real choices?
│       ≥ 25% of precise products have ≥ 3 suppliers.
├── H2  Resilience: is k = 1 feasible for meaningful baskets?
│       ≥ 50% of primary baskets of size ≤ 8 are k = 1 feasible;
│       infeasibility has a single-supplier product witness.
├── H3  Alternatives: |Tr(H)| ≥ 2 on ≥ 50% of feasible baskets,
│       and the essential suppliers are a strict subset of the minimum panel.
├── H4  Thesis techniques: fk_a_memo needs fewer distinct subproblems than the fk_a tree
│       has nodes, by ≥ 2x, on ≥ half of the certifiable instances.
│       Supplier signature classes compress |Tr(H)| by ≥ 2x on resilient baskets.
└── H5  Robustness: the H2 and H3 verdicts are unchanged under the merged mapping variant.
```

## Kill conditions

| # | Condition | Test | Fires if |
| --- | --- | --- | --- |
| K1 | Qualifications cannot be normalised reliably | audit: share of tokens in an unclassified vocabulary | > 50% of qualification incidences are unclassified |
| K2 | Overlap comes only from broad categories | H1 restricted to precise products | H1 fails on precise products but holds with broad ones |
| K3 | Robust baskets mostly infeasible | H2 | < 25% of size-≤ 8 baskets are k = 1 feasible |
| K4 | Useful baskets have trivial choices | H3 | the median of \|Tr(H)\| over feasible baskets is 1 |
| K5 | Output overwhelms enumeration | suite | > 50% of primary runs hit the cap or time limit |
| K6 | Thesis methods less practical than baselines | suite | FK-B is slower than Berge and MILP enumeration on every instance |
| K7 | Conclusions rest on unresolved identities | identity sensitivity | dropping flagged identity candidates changes the H2 or H3 verdict |

K6 firing is the expected outcome and is reported as such. It is not a reason to change the
suite.

**Steelman (rival).** One HiGHS solve per basket gives:
- the minimum panel size;
- the k = 1 increase;
- infeasibility.

The essential suppliers then follow in O(|P|) from the matrix without enumeration. The rival
wins if the alternative panels are either (a) too many to inspect or (b) differ only by
interchangeable suppliers, i.e. members of one signature class.

**Assumption ladder (weakest first).**
1. A token in the QBL is a qualification for exactly that product.
2. A row is a distinct legal supplier.
3. The workbook is current.
4. "Any one supplier fails" is the relevant disruption model.

Rung 1 is tested by H5 and rung 2 by K7. Rungs 3 and 4 are stated limits and are not tested.

## Evidence log

Scored in [2026-10-15-supplier-resilience.md](2026-10-15-supplier-resilience.md) from
`results/suite-seed1.json` by `srl/verdicts.py`.
