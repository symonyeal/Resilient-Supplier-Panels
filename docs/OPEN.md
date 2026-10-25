# Open work

The 0.4 work is recorded in the [findings and review log](findings/2026-10-25-open-work.md).

## Settled in 0.3

- **Censored programme baskets** (0.2 item 2). The four families every 0.2 route left
  incomplete have 13,155, 14,700, 20,478 and 10,978 minimal panels: Berge in Rust finishes
  them in milliseconds, and the class count of MODEL Theorem 4 gives the same numbers.
- **Specialist enumerator** (0.2 item 5). MMCS is a route; it completes 83 of 88 runs and
  stops only at the 10,000-panel cap.

## Completed in 0.4

- Three frozen workbook observations and `srl change`, with fixed-basket and
  within-snapshot comparisons under one strict catalog.
- `srl gaps`: ranked second-approval opportunities over 40,000 primary draws,
  exact uniform shares and a greedy approval sequence.
- Evidence entries for all 31 identity candidates: six supported duplicate-row
  sensitivities, 13 distinct-row entity scenarios, 12 unresolved. Strict identities
  remain unchanged; `srl entities` reports the sensitivities separately.
- Fifteen sourced owner groups (44 member entries), transitive entity scenarios and
  their product/basket exposure. Joint outages are assumptions.
- Retained and reconciled 13,798 AMS awards: candidate UEI links for 341 supplier rows.
  Recipient amounts are contextual totals, with no unit-price or capacity claim.
- Whole-class scale runs: all four full classes infeasible at k = 1. Restricted to
  products with two suppliers, exact counts reach 75,180,897,792. One count timed out.

## Evidence still needed

1. **Twelve unresolved identity pairs.** Obtain a source connecting each QBL row to an
   entity and facility. A common registration or two billing addresses do not establish
   one site. Update `catalog/identities.csv`, then run `srl entities --seeds 1000`.
2. **Comparable prices and capacity.** Obtain matched quantities, units, delivery periods
   and facility capacity before adding monetary objectives or capacity constraints.
   `python analysis/check_evidence.py` checks the available contextual evidence.
3. **Programme-class count.** Thirty products with two suppliers have a minimum resilient
   panel of 31; their number of panels remains unestablished. A separate 60-second
   follow-up also timed out (`results/program-min2-count60-v0.4.json`). Improve the
   signature-count search before another campaign; retain the censored observations.
4. **Independent entity verification.** Audit candidate UEI joins and reported parents
   against authoritative facility records. Date ownership changes separately from
   workbook observations. No outage probabilities are available.
