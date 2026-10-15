# Supplier-Resilience-Lab

Which inclusion-minimal panels of USDA-qualified food suppliers cover a product basket and
keep that coverage after any single supplier fails?

The project answers this exactly:
- on the public USDA AMS *Qualified Bidders List* (frozen snapshot of 2026-10-15);
- with the hypergraph-transversal algorithms of the author's MSc thesis;
- against MILP and Berge baselines;
- with every result certified from the qualification matrix.

It establishes **qualification coverage only**: not capacity, price, lead time or supply
continuity.

Let V be the suppliers, P the basket, E_p the suppliers qualified for p, and 𝓕 the failure
scenarios (∅ ∈ 𝓕). Then

```text
T feasible  ⇔  T ∩ (E_p \ F) ≠ ∅  for all p ∈ P, F ∈ 𝓕
minimal feasible panels = Tr(H),   H = { E_p \ F }
any k failures:  T feasible ⇔ |T ∩ E_p| ≥ k + 1 for all p
```

Proofs, edge cases and certificates are in [docs/MODEL.md](docs/MODEL.md).

## Strongest measured finding

On 88 basket instances (seed 1, [findings](docs/findings/2026-10-15-supplier-resilience.md)):

- **Single-supplier products decide resilience.** Surviving any one supplier failure was
  possible for 10 of 40 primary baskets, and for none of size 12 or 20. Every failure is
  explained by a product that has exactly one qualified supplier. 48% of precise products
  have one.
- **The MILP is fast and decisive.** The minimum panel took a median of 0.8 ms. Where
  k = 1 is feasible it needs a median of 2 more suppliers (range 1–3).
- **Alternatives are real but highly symmetric.** 41 of 46 exactly counted cases have at
  least 2 minimal panels (median 11). Families of up to 3,600 panels collapse to 28
  classes of interchangeable vendors.
- **Berge is the practical complete enumerator.** It completed 84 of 88 runs, the largest
  being 10,406 panels in 6.7 s. FK-B completed 76 of 88 and gives the earliest first panel
  (≤ 1.2 ms). Thesis memoisation cuts FK-A tree nodes by a median of 1.44×, at most 8.5×.

**Verdict.** MILP and matrix witnesses are the practical default. FK-family enumeration is
mainly a research comparison here, useful when one needs alternatives quickly from a
family too large to finish.

## Example

A basket of three canned products, required to survive any one supplier failure:

```text
python -m srl build "beans pinto can" "beans vegetarian can" "tomato sauce can" --k 1 > b.json
python -m srl solve b.json        # optimal: 2 suppliers; no essential supplier
python -m srl enum b.json --method berge > e.json
python -m srl verify e.json       # 1284 minimal panels, all certified; MILP exhaustion: complete
```

The basket has 21 eligible suppliers and |E_p| = 12, 12, 10. The minimum k = 1 panel is two
vendors qualified for all three products. Berge finds all 1,284 minimal panels in 0.38 s.

## Commands

| Command | Does |
| --- | --- |
| `python -m srl acquire` | fetch the sources to `data/fetched/` and compare them with the frozen hash (network) |
| `python -m srl audit` | Stage 0 audit of the frozen snapshot, including its age |
| `python -m srl build P… [--k K] [--variant merged] [--fail ids]` | instance JSON for a basket |
| `python -m srl solve I.json` | minimum panel, witnesses, infeasibility and essential suppliers |
| `python -m srl enum I.json --method berge\|fkb\|milp\|exhaustive` | minimal panels, process-isolated, time limit and cap |
| `python -m srl verify E.json` | re-certify saved panels and, if complete, run MILP exhaustion |
| `python -m srl bench --seed 1` | the frozen suite (about 9 min) → `results/suite-seed1.json` |
| `python -m srl report results/suite-seed1.json` | rebuild [results-seed1.md](docs/findings/results-seed1.md) from saved results |

Gate: `python -m pytest -q; python -m ruff check .; python -m pip check`.

## Layout

| Model | Module | Source |
| --- | --- | --- |
| QBL reader, identities, provenance | `srl/qbl.py` | USDA AMS QBL; vendor qualification requirements |
| product dictionary, granularity | `srl/catalog.py`, `data/catalog/mappings.csv` | [DATA](docs/DATA.md) |
| scenario hypergraph, reductions | `srl/instance.py` | MODEL Thm 1–2 |
| MILP, Berge, FK-B, MILP enumeration, exhaustive | `srl/solve.py` | Berge 1989; Fredman–Khachiyan 1996 via `fka`/`fkb` |
| certificates | `srl/certify.py` | MODEL Thm 3 |
| suite, report, pre-registered tests | `srl/bench.py`, `srl/report.py`, `srl/verdicts.py` | [BENCHMARKS](docs/BENCHMARKS.md) |

Further documentation:
- [GLOSSARY](GLOSSARY.md): terms.
- [SOURCES](docs/SOURCES.md): provenance.
- [VALIDATION](docs/VALIDATION.md): checks.
- [DECISIONS](docs/DECISIONS.md): trade-offs.
- [OPEN](docs/OPEN.md): continuation.

## Limits

- Qualification is not production capacity, inventory, willingness to bid, price or lead
  time.
- A QBL vendor can be a nonmanufacturer that uses subcontractors.
- Billing addresses are not plants, and no geographic, ownership or upstream independence is
  modelled.
- 31 supplier identity candidates are unresolved.
- Product granularity is judged per token.
- The project uses a single current snapshot, so it contains no backtest.
- Disruption is worst-case over scenarios, with no probabilities.

## Licence and attribution

GPL-3.0-or-later (see [LICENSE](LICENSE)), because this project imports `fka`/`fkb` from
[Hypergraph-Transversal-Thesis](https://github.com/symonyeal/Hypergraph-Transversal-Thesis)
(S. Y. Islam, MSc thesis, SFU 2024), which is GPL-3.0-or-later. The pinned revision is
`1bb9de48ab91629a7ae9d97488c2181b4098806d`.

USDA data is a U.S. Government work. No terms are stated in the file; the data is presumed
public domain.
