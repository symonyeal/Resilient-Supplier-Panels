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
| Name a failed run by its status (`timeout`, `cap`, `memory`, `infeasible`), never by its cause until the cause is shown. | Malformed data and a genuine infeasibility end the same way. |
| Cite only what [Sources](docs/SOURCES.md) records, and add the entry in the change that uses it. | A method is validated by the source it implements. |
| Add a crate only after measurement. | Four crates do the work: deflate, XML, JSON, SHA-256. |
| State what was measured and what was not. | "Not measured" is a finding. |
| A partial enumeration reports a lower bound; only a complete run or the class count (MODEL, Theorem 4) supports a count, and only a complete run an essential set or "no other panel". | Panels seen before a timeout say nothing about the panels not seen. |
| Essentiality, infeasibility and minimality are certified from the matrix (`src/certify.rs`), never from H or a solver's report alone. | Two routes sharing one transformation cannot check that transformation. |
| `data/raw/` is frozen; acquisition writes to `data/fetched/`. A new snapshot is a new manifest entry and a rerun, never an overwrite. | Every result names the hash it ran on. |
| Interpretations of tokens live in `data/catalog/mappings.csv`, each with a justification; judgement calls say so and go to the merged variant. | The strict matrix must not drift toward whatever makes results attractive. |
| No reduction may delete minimal panels: deduplication, superset edges, support restriction and the class reduction of Theorem 4 only. | Dominance reductions keep an optimum and lose alternatives. |
| Campaign limits change only in `docs/BENCHMARKS.md`, before the campaign runs; a campaign is a new file in `results/`, never an overwrite. | Limits tuned after seeing results select the outcome. |
| `src/thesis/` ports Hypergraph-Dualization at `1bb9de4` line for line; `tests/thesis.rs` must keep agreeing with the thesis outputs in `tests/reference/`. Fix a thesis bug upstream first, then port it. | A port that drifts from the published code no longer tests it. |

## The model is written once

- **The definition, and its two direct readings.** Feasibility, |T ∩ E_p| ≥ k + 1 together
  with the explicit scenarios, is defined once in `src/certify.rs`. The branch and bound in
  `src/solve.rs` and the completeness search in `certify::complete` are two independent
  encodings of the same definition, on purpose.
- **The scenario hypergraph.** H is built only in `src/instance.rs: hypergraph`; Berge,
  FK-B, MMCS and the class count consume it.
- **The independent check.** Exhaustive search reads the matrix, not H. Agreement between it
  and the routes through H is what tests the transformation.

## Code

Code transcribes the mathematics:
- symbol names come from `docs/MODEL.md`;
- each public doc comment states the problem it solves;
- each loop's condition is its stopping rule;
- comments cite the source.

Mathematical outcomes (`infeasible`, `timeout`) are return values, not panics. The core
(`bits` to `quotient`) computes on index sets and does no I/O; the shell (`data`,
`contracts`, `bench`, `report`, `portfolio`, `main.rs`) reads and writes.

## Before a change is finished

```text
cargo clippy --release --all-targets -- -D warnings
cargo test --release
cargo run --release -- site   # when src/ changed: the published provenance names the sources
```

A behavioural change states its effect in numbers, with the instance and the revision.
Committed results in `results/` carry the code revision, the thesis revision and the data
hash that produced them. `tests/parity.rs` repeats the full comparison with the 0.2 Python
outputs when `SRL_REF` names a captured reference directory.
