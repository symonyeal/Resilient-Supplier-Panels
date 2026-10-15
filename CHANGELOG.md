# Changelog

## Unreleased

- Outcome explorer for all 88 seed-1 instances: filters by class, rule, failure budget,
  outcome and product; a log-log exhibit of minimal panels against eligible suppliers;
  per-instance products, minimum panel, essential suppliers, infeasibility witness and
  route runs; filtered CSV and per-record JSON downloads.
- Saved minima, essential sets and infeasibility witnesses are re-certified from the
  matrix before export. Partial enumerations are exported as lower bounds.

## 0.2.0 — 2026-10-15

- Public interactive case study with three certified USDA baskets, panel comparison,
  hypothetical outages, source inspection and downloadable reports and full families.
- Installable package and `srl` entry point; frozen runtime data and schema included.
- Version 1.0 JSON contracts, CSV basket input, explicit output paths and configurable
  data directory. Legacy instances remain readable.
- Windows and hosted Ubuntu checks, wheel installation smoke test, real-browser tests
  and GitHub Pages deployment.
- Retains the original benchmark measurements, qualification model and pinned thesis code.

## 0.1.0 — 2026-10-15

- Frozen QBL reader, product mappings, scenario hypergraph, MILP and enumeration routes,
  matrix certificates, benchmark suite and findings.
- Maintenance fixes for unresolved solver statuses, invalid completeness certificates,
  PowerShell encodings, measurement definitions and incomplete reports. See
  [validation](docs/VALIDATION.md).
