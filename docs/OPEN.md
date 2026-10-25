# Open work

Ordered by what a finding would change. Each item ends in the command that continues it,
run from the repository root.

## Settled in 0.3

- **Censored programme baskets** (0.2 item 2). The four families every 0.2 route left
  incomplete have 13,155, 14,700, 20,478 and 10,978 minimal panels: Berge in Rust finishes
  them in milliseconds, and the class count of MODEL Theorem 4 gives the same numbers.
- **Specialist enumerator** (0.2 item 5). MMCS is a route; it completes 83 of 88 runs and
  stops only at the 10,000-panel cap.

## Next

1. **A second snapshot.** The strongest limit on every finding is one workbook. Fetch the
   current QBL; if its hash differs, add a manifest entry, rerun the campaign on both, and
   measure how many single-source products gained or lost a second supplier. This turns
   "30 of 40 baskets have no backup" into a statement about change over time.
   `srl acquire`
2. **Qualification gaps, ranked.** Every infeasible basket names its blocking products.
   Across the whole strict matrix, rank the products whose second approval would make the
   most sampled baskets resilient: the direct action the findings point to. Needs only the
   frozen data and the existing certificates.
   `srl audit` (single-supplier products), then a new `srl gaps` command
3. **Identity candidates.** 31 candidate pairs remain unresolved. Resolving them needs
   evidence such as a SAM UEI or an FDA facility number, not name similarity. The
   sensitivity variant already shows the stakes: merging them makes 2 primary baskets
   infeasible.
   `srl audit`
4. **Correlated scenarios beyond one group.** `--fail` supports parent-company or
   shared-plant failures, and the common-owner case uses one sourced relationship. More
   need verified ownership data, one sourced entry per group.
   `srl build <products...> --fail rA,rB -o i.json; srl solve i.json`
5. **Capacity and price.** Award data has not been reconciled with QBL rows. Monetary or
   capacity objectives wait for comparable data; the branch and bound already takes unit
   weights and would take costs.
6. **Larger baskets.** Every sampled basket now completes. Whole-programme baskets (all
   products of a class) are the next scale test; the class count handles families far
   beyond listing size.
   `srl build <all products of a class> --k 1 -o big.json; srl count big.json`
