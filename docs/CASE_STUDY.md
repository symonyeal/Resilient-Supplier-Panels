# Can a small supplier panel retain qualification coverage after a failure?

[Interactive case](https://symonyeal.github.io/Supplier-Resilience-Lab/) ·
[Certified case data](../site/cases.json) · [Model and proofs](MODEL.md)

A basket of pinto beans, vegetarian beans and tomato sauce has 21 eligible suppliers on
the frozen USDA Qualified Bidders List. One supplier can cover it; two suffice to keep
coverage after any one fails. The practical decision is whether to require that backup
coverage, and which qualified alternatives to investigate.

## Decision and result

The basket uses these exact description-class tokens:

- `beans pinto can`: 12 qualified suppliers;
- `beans vegetarian can`: 12;
- `tomato sauce can`: 10.

| Coverage requirement | Minimum supplier count | Inclusion-minimal panels |
| --- | ---: | ---: |
| Current coverage, k = 0 | 1 | 80 |
| Survive any one supplier failure, k = 1 | 2 | 1,284 |

Source: [generated cases](../site/cases.json), frozen QBL retrieved 2026-10-15, strict
mapping, every eligible supplier retained. These are complete counts, checked independently
against the matrix. Software and source revisions are embedded in the generated data.

A minimum k = 0 panel loses all three products when its sole member fails. A minimum
k = 1 panel contains two suppliers qualified for all three products: either one can fail
without losing coverage. Losing both breaks that panel. The paired-outage control is a
hypothetical stress test beyond the single-failure requirement; it is not a claim about
ownership or a forecast of disruption.

The additional supplier buys a precise mathematical property: coverage after any single
supplier failure. This is not evidence about whether either supplier can deliver the
required quantity, price or lead time. Those questions require additional data.

## Alternatives and constraints

Inclusion-minimal means no supplier can be removed. It does not mean least cardinality.
The complete family therefore contains larger panels as well as optima. The demonstration
keeps an optimum, representatives by size, then panels with different memberships. It
does not rank suppliers by suitability, cost, quality or operational independence.

Two additional real cases show why the answer depends on the basket:

- The poultry basket has two minimal panels at k = 0 and exactly one at k = 1. Both
  suppliers are required for backup coverage.
- The scarce-product basket has two minimal panels at k = 0 and none at k = 1. A product
  with only one qualified supplier proves infeasibility, regardless of panel size.

For each case, the browser lets the reader change the requirement, panel and outage;
inspect surviving coverage by product; inspect all eligible suppliers; and export a
decision report or the complete family. Results are precomputed. No optimization runs
in the browser.

## Why trust the computation?

The qualification matrix defines feasibility. For uniform k-failure resilience, each
product must have at least k + 1 members of the panel qualified for it.

1. HiGHS solves the minimum-cardinality MILP.
2. Berge enumerates inclusion-minimal transversals of the scenario hypergraph.
3. Direct matrix checks certify every returned panel and give a removal witness for each
   member. This tests the hypergraph transformation independently.
4. A separately encoded MILP excludes every listed panel and searches for an omitted one.
   Infeasibility establishes exhaustion at the solver assurance level, not formal proof.
5. The optimum size must agree with the smallest member of the certified complete family.

The build stops if these checks fail or a case cannot be completely stored within its
limit. Input and mapping hashes, the thesis pin, software version, Git revision and a
source fingerprint accompany the results. The fingerprint uses LF-normalized Python
source so Windows and hosted CI check the same content.

## Engineering judgment

The original 88-instance experiment is retained at its original revision and limits.
Berge completed 84 runs; FK-B and MILP enumeration each completed 76. The single MILP
minimum took a median of 0.8 ms in that experiment. These are historical measurements,
not timings of the current website or a new benchmark campaign.

The measured result guides the interface: show the minimum and its explanation first;
offer enumeration when alternatives matter. Retain the thesis algorithms as a checked
research comparison. See [findings](findings/2026-10-15-supplier-resilience.md) and
[benchmark protocol](BENCHMARKS.md).

## Limits

Qualification is not production capacity, inventory, willingness to bid, cost or delivery
performance. Vendor addresses do not establish operating sites. Identities remain
snapshot row identities; 31 candidate pairs are unresolved. Product mappings contain
labelled interpretation choices. There is one qualification snapshot, no historical
backtest and no disruption probabilities. This is a transparent research case study for
screening coverage questions, not a purchasing recommendation.

Rebuild the cases with `python analysis/build_site.py`; verify their matrix certificates
with `python -m pytest tests/test_portfolio.py -q`.
