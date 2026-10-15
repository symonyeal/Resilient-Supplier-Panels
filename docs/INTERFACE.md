# Package and exchange interface

Version 0.2.0 requires Python 3.14 and Git for the pinned thesis dependency. The release
wheel includes the frozen QBL, manifest, mappings and JSON schema; commands work outside
the checkout. `python -m srl` is equivalent to the installed `srl` entry point and works
when Python's Scripts directory is not on PATH.

## Files

`srl build --basket basket.csv --k 1 -o instance.json` accepts a UTF-8 CSV with exactly
one column, `product`, containing distinct exact product tokens. Quote tokens containing
commas. The example is [canned.csv](../examples/canned.csv).

`-o` / `--output` writes UTF-8 without relying on the shell's redirection encoding.
JSON input also accepts UTF-8 BOM and UTF-16. A document contains `schema_version: "1.0"`
and a `kind` of `instance`, `solution` or `enumeration`. The complete schema is
[document-1.0.json](../srl/schemas/document-1.0.json), also available through `srl schema`.

Instances carry V, P, E, k, F, supplier names, the snapshot hash, mapping hash, software
version and thesis revision. Solutions embed their instance and expose the solve status,
panel, minimality witnesses, infeasibility witness and essential suppliers. Enumerations
retain the instance plus run status, received/stored counts and stored panels.

Unversioned legacy instances remain readable and are upgraded in newly written solutions.
Unsupported schema versions are rejected. `Instance` checks the mathematical invariants
that JSON Schema cannot express: aligned products and edges, distinct labels, nonnegative
integral k, and qualification/failure sets contained in V.

## Outcomes

- `optimal`: HiGHS returned an optimum; this is not a statement that supply will arrive.
- `infeasible`: a mathematical outcome, with a matrix witness where the model supplies one.
- `highs status s`: unresolved solver outcome; no optimum or exhaustion is inferred.
- Enumeration `complete`: terminated by exhaustion. Its count is exact, even when Berge
  stored fewer panels than it counted.
- `timeout` / `cap`: partial enumeration and a lower bound on the family size.
- `error`: worker failure, not a censored measurement.

`verify` certifies each stored panel. It runs independent MILP exhaustion only for a
claimed complete family that is entirely stored. Invalid, duplicate or missing panels
produce exit status 1. Input/argument errors produce status 2; mathematical outcomes are
reported in JSON. A partial verification does not establish a complete count.

## Data and output locations

The default is the frozen data inside the installed package, or `data/` in a source
checkout. Set `SRL_DATA_DIR`, or pass `srl --data-dir PATH ...`, to use a directory with
`manifest.json`, `raw/QBL.xlsx` and `catalog/mappings.csv`. No implicit entity merge occurs.
Preserve a new snapshot and its manifest rather than replacing the frozen original.

Explicit output paths, benchmark results (`results/`) and acquisition downloads
(`data/fetched/`) are relative to the caller's working directory. Package data is read-only.
For a saved benchmark, use `srl report results.json -o report.md` to choose the output.

## Public demonstration

`python analysis/build_site.py` rebuilds `site/data.js`, `site/cases.json` and the public
schema after certification. Case data includes full families, shortlists, matrix-derived
outage results and provenance. Outages are fixed hypothetical sets, not observed events.
The page implements filtering and presentation only.

The site opens from `site/index.html` without a server or external JavaScript libraries.
GitHub Actions publishes the same files on GitHub Pages. The HTML decision report contains
the selected panel, stress test, per-product coverage, original-panel witnesses and source
hashes. Full-family JSON can be passed directly to `srl verify`.
