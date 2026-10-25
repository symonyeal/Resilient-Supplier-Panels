# A common-owner failure case

Two approved suppliers can belong to the same corporate group. This case uses the
frozen USDA list's chicken legs and pork basket to examine that risk with the existing
explicit-failure model. It changes no qualification or supplier mapping.

The USDA rows identify `r0911` as Tyson Foods Inc and `r0912` as Tyson Fresh Meats.
[Tyson Foods' 2025 Exhibit 21](https://www.sec.gov/Archives/edgar/data/100493/000010049325000095/tsn2025q410-kexx21.htm)
lists Tyson Fresh Meats, Inc. among its subsidiaries. The abbreviated USDA name is
matched to that legal name for this scenario. Other rows called only Tyson are not
included: their legal identity is not established by that label alone.

The scenario assumes both named suppliers' approved operations are unavailable
together. Ownership is sourced; the outage is assumed. No shared plant, outage
history, probability, capacity or ability to deliver is inferred.

## Comparison

Measured on 20 October 2026:

| Failure requirement | Complete minimal panels | Smallest panel |
| --- | ---: | ---: |
| Approvals only | 32 | 2 |
| Any one supplier | 168 | 4 |
| The named group alone | 21 | 2 |
| Any one supplier and the named group | 168 | 4 |

The group scenario removes 11 approval-only panels. Adding it to the one-supplier
requirement leaves the entire panel family unchanged, not merely its count. Each
of these two group members qualifies for a different product in this basket, so
one-supplier protection already covers the assumed joint loss product by product.
This is a neutral result for adding that scenario, not evidence that ownership
risk never matters for other baskets.

`results/common-owner/comparison.json` records the source, mapping, assumptions,
code revision and frozen qualification-data hashes. Instance and complete-panel
documents sit beside it. The cases compare approvals only, any one supplier lost,
the named group alone, and both failure requirements together. The last comparison
can be unchanged: a common-owner scenario does not necessarily add a constraint
when it removes at most one supplier for each product already protected against
any one loss. The calculated results, rather than an assumed benefit, decide this.

Enumeration reads the qualification matrix directly. Every returned panel is
independently checked for feasibility and minimality, and a separate exhaustive search
finds no missing panel, which proves each family complete.

## Reproduce

From the installed project, use the existing interface for each case:

```text
srl build "chicken legs chilled-bulk" pork --k 0 -o approvals.json
srl build "chicken legs chilled-bulk" pork --k 1 -o one_supplier.json
srl build "chicken legs chilled-bulk" pork --k 0 --fail r0911,r0912 -o group_only.json
srl build "chicken legs chilled-bulk" pork --k 1 --fail r0911,r0912 -o common_owner.json
srl enum common_owner.json --method exhaustive -o panels.json
srl verify panels.json
```

Repeat the last two commands for each instance to compare the complete panel lists.
Write local output into a project-owned work directory. A different USDA snapshot
needs new row-name checks and new results; these IDs are specific to the frozen list.

This case establishes conditional approval coverage. A purchasing decision still
needs capacity, prices, delivery performance and confirmed counterparty identities.
