# Open issues

Each item ends in the command that continues it. Commands run from the repository root.

1. **New snapshot.** Fetch the current QBL and compare it with the frozen hash. If it
   differs, add a manifest entry and rerun the suite. Never overwrite `data/raw/`.
   `python -m srl acquire`
2. **Censored programme baskets.** Some k = 1 programme baskets stop at 10 s on every route.
   One option is a single longer run (60 s) on those instances only. Another is a
   symmetry-aware enumerator that lists one panel per class multiset and reconstructs the
   labelled panels (Stage 3).
   `python -m srl build <products...> --k 1 > i.json; python -m srl enum i.json --method berge --timeout 60`
3. **Identity candidates.** 31 candidate pairs remain unresolved. Resolving them needs
   evidence such as a SAM UEI or an FDA facility number, not name similarity.
   `python -m srl audit`
4. **Correlated scenarios.** `Instance.F` and `--fail` support parent-company or shared-plant
   failures. No such scenario is used yet, because no ownership data was verified.
   `python -m srl build <products...> --fail rA,rB > i.json; python -m srl solve i.json`
5. **Specialist enumerator.** SHD or RS (Murakami–Uno) is not yet included; see DECISIONS.
6. **Later snapshots.** Coverage persistence can be measured prospectively across real
   releases. A historical backtest cannot be built from one workbook.
7. **Capacity and price.** Award data has not been reconciled. Monetary or capacity
   objectives wait for comparable data.
