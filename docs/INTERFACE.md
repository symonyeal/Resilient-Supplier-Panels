# Command line and exchange interface

Version 0.3.0 is one program, `srl`, written in Rust. The frozen QBL, manifest, mappings
and JSON schema are compiled into it, so commands work outside the checkout. Build it with
`cargo install --locked --path .`, or use a release binary. From a checkout,
`cargo run --release -- <command>` is equivalent.

| Command | Does |
| --- | --- |
| `srl build [PRODUCT ...] [--basket CSV] [--k N] [--variant strict\|merged] [--fail ID,ID ...] [-o FILE]` | instance document for a basket |
| `srl solve FILE [-o FILE]` | minimum panel, witnesses, infeasibility witness, essential suppliers |
| `srl enum FILE [--method berge\|fkb\|cuts\|mmcs\|exhaustive] [--timeout S] [--cap N] [-o FILE]` | minimal panels |
| `srl count FILE [--timeout S] [-o FILE]` | exact number of minimal panels by signature classes |
| `srl verify FILE [-o FILE]` | certificates for an enumeration document |
| `srl audit [-o FILE]`, `srl schema [-o FILE]` | snapshot audit; the JSON Schema |
| `srl bench --seed N -o FILE [--timeout S] [--no-variants]` | benchmark campaign |
| `srl report FILE [-o FILE]` | findings tables from a saved campaign |
| `srl site [--results FILE] [--root DIR]` | public case data in `DIR/site` |
| `srl acquire` | download the sources to `data/fetched` (needs curl and network) |

## Files

`srl build --basket basket.csv --k 1 -o instance.json` accepts a UTF-8 CSV with exactly
one column, `product`, containing distinct exact product tokens. Quote tokens containing
commas. The example is [canned.csv](../examples/canned.csv).

`-o` / `--output` writes UTF-8 without relying on the shell's redirection encoding.
JSON input also accepts UTF-8 BOM and UTF-16. A document contains `schema_version: "1.0"`
and a `kind` of `instance`, `solution` or `enumeration`. The complete schema is
[document-1.0.json](../schema/document-1.0.json), also available through `srl schema`.
JSON is written as the 0.2 release wrote it (one-space indent, ASCII escapes).

Instances carry V, P, E, k, F, supplier names, the snapshot hash, mapping hash, software
version and thesis revision. Solutions embed their instance and expose the solve status,
panel, minimality witnesses, infeasibility witness and essential suppliers. Enumerations
retain the instance plus run status, received/stored counts and stored panels.

Unversioned legacy instances remain readable and are upgraded in newly written solutions.
Unsupported schema versions are rejected. Loading checks the mathematical invariants that
JSON Schema cannot express: aligned products and edges, distinct labels, a nonnegative
integer k, and qualification/failure sets contained in V.

## Outcomes

- `optimal`: the branch and bound proved a minimum; this is not a statement that supply
  will arrive.
- `infeasible`: a mathematical outcome, with a matrix witness where the model supplies one.
- Enumeration `complete`: terminated by exhaustion. Its count is exact, even when Berge
  stored fewer panels than it counted.
- `timeout` / `cap` / `memory`: partial enumeration and a lower bound on the family size.
  `memory` means Berge's working family reached its limit.
- `error`: a failed route, not a censored measurement.

`verify` certifies each stored panel. It runs the independent exhaustion search only for a
claimed complete family that is entirely stored, and reports it as `exhaustion`
(`complete`, `missing` or `invalid`). Invalid, duplicate or missing panels produce exit
status 1. Input and argument errors produce status 2; mathematical outcomes are reported in
JSON. A partial verification does not establish a complete count; `srl count` does.

Changes from 0.2: `verify` reports `exhaustion` where 0.2 reported `milp_exhaustion`;
`--method milp` is now `--method cuts`; `count` and `--method mmcs` are new; `bench` needs
an explicit `-o` and refuses to overwrite an existing campaign.

## Data and output locations

The default is the frozen data compiled into the program. Set `SRL_DATA_DIR`, or pass
`srl --data-dir PATH ...`, to use a directory with `manifest.json`, `raw/QBL.xlsx` and
`catalog/mappings.csv`. No implicit entity merge occurs. Preserve a new snapshot and its
manifest rather than replacing the frozen original.

Explicit output paths, benchmark results (`results/`) and acquisition downloads
(`data/fetched/`) are relative to the caller's working directory. `srl report
results/suite-<name>.json` writes `docs/findings/results-<name>.md` beside it unless `-o`
is given.

## Public demonstration

`cargo run --release -- site` rebuilds `site/data.js`, `site/cases.json` and the public
schema after certification, from the campaign `results/suite-seed1-v0.3.json`. Case data
includes full families, shortlists, matrix-derived outage results and provenance. Outages
are fixed hypothetical sets, not observed events. The page implements filtering and
presentation only.

The site opens from `site/index.html` without a server or external JavaScript libraries.
GitHub Actions publishes the same files on GitHub Pages. The HTML decision report contains
the selected panel, stress test, per-product coverage, original-panel witnesses and source
hashes. Full-family JSON can be passed directly to `srl verify`.
