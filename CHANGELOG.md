# Changelog

## 0.4.0 — 2026-10-25

- Two archived USDA workbooks embedded alongside the original; `--snapshot` selects one,
  and `srl change` measures observed qualification changes under the same catalog.
- `srl gaps` ranks second-approval opportunities over seeded baskets and reports exact
  uniform shares plus a greedy approval sequence.
- `srl entities` reports identity sensitivities, explicit entity outages and candidate
  recipient award totals. Six duplicate pairs supported, 13 distinct-row entity
  scenarios, 12 unresolved; strict supplier identities are unchanged.
- Frozen evidence for 13,798 distinct AMS awards and 341 candidate supplier-to-UEI matches.
  Published amounts reconcile to raw records. No unit-price or capacity claim.
- Whole-class baskets via `build --class` and `--min-suppliers`: up to 75,180,897,792
  minimal panels counted exactly. One programme-class count times out at 10 seconds.
- [Findings and review log](docs/findings/2026-10-25-open-work.md) record the remaining
  evidence requirements. The original campaign is unchanged.

## 0.3.0 — 2026-10-25

- Rewritten in Rust: one `srl` binary with the frozen data compiled in; Windows and Linux
  binaries on the release. The Python package is retired (last revision `99cd50c`).
- The Berge, FK-A and FK-B code is ported line for line from the thesis code at
  `1bb9de4` and tested against its outputs on 300 random and 88 suite instances.
- HiGHS replaced by an exact branch and bound (minimum, cut enumeration) and an
  exhaustive completeness search, which proves exhaustion rather than solver assurance.
- New: the MMCS enumerator (`--method mmcs`), exact counts by signature classes
  (`srl count`, MODEL Theorem 4), and status `memory` for Berge's working-set limit.
- Release 0.3 campaign (`results/suite-seed1-v0.3.json`): every one of the 88 families has
  an exact count, including the four censored in 0.2; K6 fires. See
  [findings](docs/findings/2026-10-25-rust-campaign.md).
- The site presents the 0.3 campaign. The case shortlists start from the first optimum in
  label order instead of the solver's choice.
- Interface: `verify` reports `exhaustion` (was `milp_exhaustion`); `--method milp` is now
  `--method cuts`; `bench` needs `-o` and never overwrites a campaign.

## After 0.2.0 (first released in 0.3.0)

- Outcome explorer for all 88 seed-1 instances: filters by class, rule, failure budget,
  outcome and product; a log-log exhibit of minimal panels against eligible suppliers;
  per-instance products, minimum panel, essential suppliers, infeasibility witness and
  route runs; filtered CSV and per-record JSON downloads.
- Saved minima, essential sets and infeasibility witnesses are re-certified from the
  matrix before export. Partial enumerations are exported as lower bounds.
- Site copy rewritten answer-first: findings as headlines, figures read from the data,
  one-line notes; marketing strip and repeated caveats removed.

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
