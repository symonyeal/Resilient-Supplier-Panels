# Portfolio release 0.2.0

Approved scope: an industry-facing portfolio case study, authorised in conversation by
“Do it” on 2026-10-15. This extends the completed qualification model; it does not add
capacity, prices or evidence about actual deliveries.

## Reader and decision

A procurement analyst or technical reviewer can explore a real basket, see which supplier
panels retain qualification coverage under a stated outage, and trace each result to the
frozen USDA workbook and an independent matrix certificate.

## Design

1. Package the existing solver with an installed `srl` command. Declare runtime and
   development dependencies, include the frozen workbook, manifest, mappings and schemas
   in the wheel, and support an explicit data-directory override. Writes go to paths the
   caller controls. Keep the pinned thesis dependency and existing commands.
2. Introduce version 1.0 JSON contracts for instances, solutions and enumerations. Accept
   existing unversioned instances; reject unsupported versions. Preserve mathematical
   statuses and distinguish partial enumeration from exhaustion. Import baskets from a
   one-column CSV and export JSON without shell redirection.
3. Generate three small real-data cases, including the three canned products. For k = 0
   and k = 1, check the optimum, all enumerated panels and exhaustion independently from
   the matrix before exporting. Select a short, reproducible list of alternative panels.
   Precompute product coverage for each single-supplier outage and one clearly labelled
   hypothetical paired outage. The browser looks up results; it implements no solver.
4. Publish a buildless, accessible HTML/CSS/JS site on GitHub Pages. Show the lead case,
   controls for basket, failure budget, panel and outage, the qualification matrix, named
   alternatives, downloadable decision reports, benchmark evidence and limitations. It
   also opens directly from disk. No accounts, backend or external data calls are needed.
5. Lead the README with the demo and decision. Add a short case study, architecture,
   installation and reproduction instructions, and a changelog. Record source and build
   provenance rather than relabelling old timings as new measurements.
6. Run checks on Windows and hosted Ubuntu. Build wheel and source distributions; test
   the installed wheel away from the checkout. Publish release assets under `v0.2.0` only
   after these checks and the Pages workflow pass.

## Acceptance

- The canned case reproduces its measured family; no displayed fact is handwritten in
  place of a generated result.
- An analyst can change a panel and an outage, see which products lose coverage, reset
  the case and download the displayed decision with its source and assumptions.
- Browser checks cover controls, failure and infeasible states, downloads, keyboard
  access, small screens and errors. Generated cases are certified before publication.
- An installed wheel can audit, build, solve, enumerate and verify from another working
  directory without the checkout. Packaged data hashes equal the frozen originals.
- The existing result files and findings still reproduce. A fresh timed main campaign
  is outside this release; additional case-build timings are labelled separately.

## Boundaries

Supplier IDs remain snapshot row identities. The demo does not claim cross-release entity
resolution, plant independence, verified ownership links, cost savings or delivery
performance. Dates, unresolved identities and the qualification-only interpretation stay
visible. Prices, capacity and symmetry-aware enumeration are later, separate projects.
