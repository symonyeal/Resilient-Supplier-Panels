# Working contract

Read before changing anything.
- [README](README.md) states what the project computes and claims.
- [Model](docs/MODEL.md) holds the theorems the code transcribes.
- [Decisions](docs/DECISIONS.md) explains why things are the way they are.
- [Validation](docs/VALIDATION.md) records what has been checked and how.

This file states the rules a change must not break. Codex reads the same file through
[AGENTS.md](AGENTS.md). Domain terms are defined in [GLOSSARY](GLOSSARY.md).

## Always on

| Rule | Reason |
| --- | --- |
| Report no outcome that was not read back from the result that produced it. | A requested tolerance is not a result. |
| Name a failed run by its status (`timeout`, `cap`, `infeasible`), never by its cause until the cause is shown. | Malformed data and a genuine infeasibility end the same way. |
| Cite only what [Sources](docs/SOURCES.md) records, and add the entry in the change that uses it. | A method is validated by the source it implements. |
| Add a dependency only after measurement. Use the system Python 3.14; no virtual environment. | numpy and scipy come first. |
| State what was measured and what was not. | "Not measured" is a finding. |
| A partial enumeration reports a lower bound; only a complete run supports a count, an essential set or "no other panel". | Panels seen before a timeout say nothing about the panels not seen. |
| Essentiality, infeasibility and minimality are certified from the matrix (`srl/certify.py`), never from H or a solver's report alone. | Two routes sharing one transformation cannot check that transformation. |
| `data/raw/` is frozen; acquisition writes to `data/fetched/`. A new snapshot is a new manifest entry and a rerun, never an overwrite. | Every result names the hash it ran on. |
| Interpretations of tokens live in `data/catalog/mappings.csv`, each with a justification; judgement calls say so and go to the merged variant. | The strict matrix must not drift toward whatever makes results attractive. |
| No reduction may delete minimal panels: deduplication, superset edges and support restriction only. | Dominance reductions keep an optimum and lose alternatives. |
| The main-suite limits change only in `docs/BENCHMARKS.md`, before the suite runs. | Limits tuned after seeing results select the outcome. |
| The thesis package (`fka`) is a pinned dependency; fix a thesis bug upstream, then repin. | A local copy of the solver diverges from the published reproduction tests. |

## The model is written once

- **The definition, and its two direct readings.** Feasibility, |T ∩ E_p| ≥ k + 1 together
  with the explicit scenarios, is defined once in `srl/certify.py`. The MILP rows in
  `srl/solve.py` and the completeness search in `certify.complete` are two independent
  encodings of the same definition, on purpose.
- **The scenario hypergraph.** H is built only in `srl/instance.py: hypergraph`, and Berge
  and FK-B consume it.
- **The independent check.** Exhaustive search reads the matrix, not H. Agreement between it
  and Berge or FK-B is what tests the transformation.

## Code

Code transcribes the mathematics:
- symbol names come from `docs/MODEL.md`;
- each public docstring states the problem it solves;
- each loop's condition is its stopping rule;
- comments cite the source.

Mathematical outcomes (`infeasible`, `timeout`) are return values, not exceptions.

## Before a change is finished

```text
python -m pytest -q
python -m ruff check .
python -m pip check
```

A behavioural change states its effect in numbers, with the instance and the revision.
Committed results in `results/` carry the code revision, the thesis revision and the data
hash that produced them.
