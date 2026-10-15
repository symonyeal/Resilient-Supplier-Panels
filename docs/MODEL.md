# Model

## Data and assumptions

- V is a finite set of suppliers (QBL identities, `GLOSSARY.md`).
- P is a finite basket.
- E_p ⊆ V is the set of suppliers qualified for p, for p ∈ P.
- 𝓕 ⊆ 2^V is a failure model with ∅ ∈ 𝓕.

The implemented models are:
- 𝓕_k = {F : |F| ≤ k};
- 𝓕_k ∪ 𝓕_0, where 𝓕_0 is an explicit list of correlated scenarios.

Assumptions:
- **A1.** Qualification is binary and read from the matrix. There is no capacity, price or
  lead time.
- **A2.** A failed supplier provides nothing, and a surviving one provides every
  qualification it holds.
- **A3.** Failure scenarios are worst-case sets. No probability is attached.

**Definition.** A panel T ⊆ V is *feasible* if T ∩ (E_p \ F) ≠ ∅ for all p ∈ P and F ∈ 𝓕. It
is *minimal* if it is feasible and no proper subset is feasible. It is *minimum* if it is
feasible and of least cardinality.

## Theorem 1 (scenario hypergraph)

Let H = {E_p \ F : p ∈ P, F ∈ 𝓕}. Then the minimal feasible panels are exactly Tr(H), the
inclusion-minimal transversals of H.

*Proof.* By definition, T is feasible iff T meets every edge of H, i.e. iff T is a
transversal of H. Minimality is the same order condition on both sides. ∎

Edge cases:
1. If some edge is empty, no T meets it. Then Tr(H) = ∅ and the instance is infeasible. The
   witness is a pair (p, F) with E_p ⊆ F.
2. If P = ∅, then H = ∅, every T is feasible, and Tr(H) = {∅}: the empty panel is feasible
   and minimal.
3. Duplicate edges impose the same condition twice, so removing them leaves the
   transversals unchanged.
4. If e ⊆ e′ are both edges, every set meeting e meets e′. Hence Tr(H) = Tr(min H), where
   min H keeps the inclusion-minimal edges.
5. A supplier in no edge belongs to no minimal transversal: if v ∉ ∪H and T is a
   transversal containing v, then T \ {v} is also a transversal. Restricting to the support
   ∪H therefore preserves Tr(H).

These are the only reductions applied (`srl/instance.py`). Each preserves the whole family
Tr(H), not merely its optimum. Provenance maps every kept edge to all the pairs (p, F) that
generate it.

No supplier-dominance reduction is applied. Deleting a supplier whose qualifications are a
subset of another's preserves the minimum but deletes minimal panels.

## Theorem 2 (uniform k-failure resilience)

For 𝓕 = 𝓕_k, T is feasible iff |T ∩ E_p| ≥ k + 1 for every p ∈ P.

*Proof.*
- (⇐) Let |F| ≤ k. Then |T ∩ (E_p \ F)| ≥ |T ∩ E_p| − |F| ≥ 1.
- (⇒) Suppose |T ∩ E_p| ≤ k for some p. Take F = T ∩ E_p ∈ 𝓕_k. Then
  T ∩ (E_p \ F) = ∅. ∎

Hence, for 𝓕_k, it suffices to take the edges E_p \ F with F ⊆ E_p and |F| = min(k, |E_p|).
Any other F gives a superset of one of these edges. For k = 1 these are the |E_p| sets
E_p \ {v}. If |E_p| ≤ k, the edge is empty, and the instance is infeasible by Theorem 1(1).

The MILP and the certificates use the cardinality form directly. Only Berge and FK-B need H.

## Certificates (Theorem 3)

Let T be feasible.

1. *Minimality.* T is minimal iff every v ∈ T has a witness (p, F) ∈ P × 𝓕 with
   T ∩ (E_p \ F) = {v}. Under 𝓕_k this says |T ∩ E_p| = k + 1 with v ∈ E_p, and
   F = (T ∩ E_p) \ {v}.

   *Proof.* T \ {v} is infeasible iff some (p, F) has T ∩ (E_p \ F) ⊆ {v}. Since T is
   feasible, the intersection is nonempty, so it equals {v}. For 𝓕_k, Theorem 2 makes this
   |(T \ {v}) ∩ E_p| ≤ k. Since |T ∩ E_p| ≥ k + 1, equality holds and v ∈ E_p. Finally, T is
   minimal iff T \ {v} is infeasible for every v, because feasibility is monotone. ∎

2. *Essentiality.* v lies in every minimal panel iff V \ {v} is infeasible, i.e. iff some
   (p, F) has E_p \ F = {v}. Under 𝓕_k this means |E_p| = k + 1 with v ∈ E_p. This assumes
   the instance itself is feasible.

   *Proof.*
   - (⇐) Every feasible panel meets E_p \ F = {v}.
   - (⇒) If V \ {v} is feasible, it contains a minimal panel, and that panel avoids v. ∎

   Essentiality therefore needs no enumeration. A supplier seen in every panel of a
   *partial* run is not thereby essential. Only this witness, or a complete family, decides
   it.

3. *Completeness.* A family 𝒞 of minimal panels equals Tr(H) iff no feasible T contains no
   member of 𝒞. `certify.complete` searches for such a T with a separately encoded MILP, in
   the cardinality form with the constraints Σ_{v∈S}(1 − x_v) ≥ 1 for S ∈ 𝒞. If a T is found,
   it is shrunk to a minimal panel, which lies outside 𝒞.

## MILP enumeration

Repeat: solve min Σ x_v subject to feasibility and Σ_{v∈S} x_v ≤ |S| − 1 for every found S.
Stop when the problem is infeasible.

- Each optimum T is minimal. A feasible T′ ⊊ T contains no found S, because T contains none,
  and T′ is cheaper.
- At termination, every feasible set contains a found panel, so every minimal panel has been
  found.
- Panels are produced in nondecreasing size.

## What the model does not represent

Capacity, inventory, willingness to bid, price, lead time, geography, common ownership,
upstream inputs, failure probability and actual service in a disruption. See
`docs/DATA.md` and the README's limits.
