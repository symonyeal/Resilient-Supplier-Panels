# Sources

A method is validated by the source it implements. Cite only what this file records, and
add the entry in the change that uses it.

Status:
- **inspected**: read for this project, with what was read and when;
- **cited**: not yet read for this project.

## Data (register; hashes in `data/manifest.json`)

| Source | Used for | Status |
| --- | --- | --- |
| USDA AMS, *Qualified Bidders List* (QBL.xlsx) | the qualification matrix | **inspected** 2026-10-15. URL `https://www.ams.usda.gov/sites/default/files/media/QBL.xlsx`, linked from `https://www.ams.usda.gov/selling-food/solicitations`. Retrieved 07:16:47Z; SHA-256 `b7b8414a…8822`; 113,416 B; Last-Modified 2026-10-02; no date stated in the file. A current snapshot, with no historical releases linked. Terms are not stated; presumed public domain as a U.S. Government work (17 U.S.C. §105). This is a presumption, not a verified licence. |
| USDA AMS, *Vendor Qualification Requirements for Prospective Contractors* (CPPNewVendorQualificationRequirements.pdf), effective 2022-12-21 | meaning of a QBL row | **inspected** 2026-10-15, 10 pp. §I: the Contracting Officer determines "if a prospective contractor is qualified, including for which products, and add[s] approved vendors to the Qualified Bidders List/Qualified Products List". §II: plant survey or audit; nonmanufacturers qualify through subcontractors. A QBL vendor is therefore not necessarily the producer. SHA-256 `e42a883f…a9fc`. |
| USDA AMS, *Food Commodity Purchasing* reports page | whether award fields exist | **inspected** 2026-10-15: it links cumulative purchase summaries only. No join to QBL was established. |

## Methods

| Source | Used for | Status |
| --- | --- | --- |
| S. Y. Islam, *Tracing the Effects of Symmetry in Hypergraphs on the Fredman–Khachiyan Algorithm*, MSc thesis, Simon Fraser University, 2024; code `fka` 2.0.0 at `1bb9de48ab91629a7ae9d97488c2181b4098806d`, GPL-3.0-or-later | Berge oracle, FK-A, `fk_a_memo`, FK-B and its dualisation; ported to Rust in 0.3 (`src/thesis/`) | **inspected** 2026-10-15: `docs/architecture.md`, `src/fka/{hypergraph,transversal,algorithm,memo}.py`, `src/fkb/{algorithm,dualize}.py`, `docs/application-alternatives.md`; 2026-10-25, at `1bb9de4`, line by line for the port: `src/fka/{hypergraph,irredundant,transversal,algorithm,memo}.py`, `src/fkb/{algorithm,dualize}.py` |
| M. L. Fredman, L. Khachiyan, "On the complexity of dualization of monotone disjunctive normal forms", *J. Algorithms* 21 (1996) 618–628 | FK-A/FK-B, quasi-polynomial duality | cited (implemented in `fka`, `fkb`) |
| C. Berge, *Hypergraphs: Combinatorics of Finite Sets*, North-Holland, 1989 | transversal hypergraph, Berge multiplication | cited |
| T. Eiter, G. Gottlob, "Identifying the minimal transversals of a hypergraph and related problems", *SIAM J. Comput.* 24(6) (1995) 1278–1304 | Tr and duality equivalence | cited |
| A. Gainer-Dewar, P. Vera-Licona, "The minimal hitting set generation problem: algorithms and computation", *SIAM J. Discrete Math.* 31(1) (2017) 63–100 | benchmark practice for minimal-hitting-set enumerators | cited |
| K. Murakami, T. Uno, "Efficient algorithms for dualizing large-scale hypergraphs", *Discrete Appl. Math.* 170 (2014) 83–94 | MMCS, the specialist enumerator `src/mmcs.rs` (0.3); SHD and RS not included | cited; the implementation agrees with Berge on 350 random instances and every suite family |
| B. Tomlin, "On the value of mitigation and contingency strategies for managing supply chain disruption risks", *Management Science* 52(5) (2006) 639–657 | dual sourcing as a disruption strategy (context only) | cited |
| L. V. Snyder et al., "OR/MS models for supply chain disruptions: a review", *IIE Transactions* 48(2) (2016) 89–109 | disruption-model context | cited |
| Q. Huangfu, J. A. J. Hall, "Parallelizing the dual revised simplex method", *Math. Prog. Comp.* 10 (2018) 119–142; HiGHS via SciPy `milp` | MILP baseline in 0.2; not used from 0.3 | cited |
| M. Matsumoto, T. Nishimura, "Mersenne Twister: a 623-dimensionally equidistributed uniform pseudo-random number generator", *ACM TOMACS* 8(1) (1998) 3–30; CPython 3.14 `random` | basket draws reproducing the 0.2 suite (`src/pyrand.rs`) | `Random.sample`, `shuffle`, `choice`, `randint` and `_randbelow_with_getrandbits` **inspected** 2026-10-25 in the installed 3.14.7 library; seeding by `init_by_array` cited; draws equal Python's on five seeds |

## Common-owner case

- [Tyson Foods, 2025 Form 10-K Exhibit 21](https://www.sec.gov/Archives/edgar/data/100493/000010049325000095/tsn2025q410-kexx21.htm):
  inspected 2026-10-20. Lists Tyson Fresh Meats, Inc. as a subsidiary. Used only for
  the explicit corporate-group scenario in COMMON_OWNER_CASE.md; the USDA abbreviated
  name is a documented match. This establishes no shared plant or failure probability.

## Software delivery sources

- Release 0.2 packaging, retired in 0.3:
  [PyPA, Writing your pyproject.toml](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/):
  inspected 2026-10-15, build-system, project dependencies and command entry points;
  [Setuptools, Extending or Customizing](https://setuptools.pypa.io/en/latest/userguide/extension.html):
  inspected 2026-10-15; no custom backend is needed. Explicit package mappings include the
  canonical frozen data in the wheel.
- Crates (versions in `Cargo.lock`): `miniz_oxide` (raw deflate of the xlsx members),
  `roxmltree` (SpreadsheetML), `serde_json` (documents; `preserve_order`,
  `float_roundtrip`), `sha2` (hashes). Used through their documented public APIs;
  the reader's output equals the 0.2 reader's on all 983 rows.
- [GitHub, custom Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
  and [Pages REST API](https://docs.github.com/en/rest/pages/pages): inspected 2026-10-15,
  Actions deployment and workflow build configuration.
- [Playwright Python installation](https://playwright.dev/python/docs/intro): inspected
  2026-10-15, browser installation and browser automation; Edge locally, Chromium in CI.
