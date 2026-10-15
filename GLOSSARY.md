# Glossary

Domain terms only. Implementation lives elsewhere.

**Supplier.** A row of the Qualified Bidders List (QBL): a vendor that USDA AMS lists as
qualified to bid. It is not a plant, a parent company or a contract holder.

**Supplier identity.** The key (normalised vendor name, normalised billing address, 5-digit
ZIP). Rows with equal keys are one supplier. Rows with equal names but different addresses
are distinct suppliers, flagged as *identity candidates*. A shared name, address or owner is
never merged without evidence. A billing address is not a manufacturing location.

**Qualified product.** A product token as written in a QBL Product cell, after harmless
normalisation of whitespace, case and the spacing around " - ". The token is used at its
written granularity. Approval for a category is not approval for its members.

**Granularity class.** Each token belongs to one class, set by an explicit rule
(`docs/DATA.md`):
- `material`: a numbered USDA material;
- `description`: a material description with a pack or form;
- `program`: an AMS commodity programme code;
- `naics`: a NAICS-prefixed commodity code;
- `broad`: a category with no specific product.

**Qualification.** The pair (supplier, qualified product) read from one cell.

**Product basket.** A finite set P of qualified products that a decision requires, drawn from
one granularity class.

**Supplier panel.** A set T of suppliers. It is not a purchase allocation, an order or a
promise of production.

**Failure scenario.** A set F of suppliers assumed unavailable. A *failure model* is a family
of scenarios that always contains ∅. "Any k" is the family {F : |F| ≤ k}.

**Qualification coverage.** T covers P if every p ∈ P has a supplier in T qualified for p.

**Resilient qualification coverage.** T covers P under every scenario of the failure model:
T ∩ (E_p \ F) ≠ ∅ for all p and F. For "any k" this means |T ∩ E_p| ≥ k + 1.

**Inclusion-minimal panel.** A covering panel from which no supplier can be removed. Equal
to an element of Tr(H). These are *minimal panels*.

**Minimum-cardinality panel.** A covering panel of least size. Every minimum panel is
minimal; the converse fails.

**Essential supplier.** A supplier in every minimal panel. Equivalently, coverage is
impossible without it.

**Complete enumeration.** A run that returned all of Tr(H) and terminated by its own
stopping rule. Only a complete run supports a count, a full essential set or a claim about
all alternatives.

**Partial enumeration.** A run stopped by a time or output limit. Each returned panel is
valid. The count, the essential set and the absence of other panels are unknown.

**Verification certificate.** Evidence, checkable from the qualification matrix alone, for
one claim: a feasibility check, a minimality witness for each panel member, a product
witness of infeasibility or essentiality, or a completeness search. Its *assurance level*
says how it was obtained (direct check, exhaustive, agreement between independent methods,
solver result).
