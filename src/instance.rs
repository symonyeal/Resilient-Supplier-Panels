//! Instance and scenario hypergraph (docs/MODEL.md).
//!
//! V suppliers, P basket, E_p ⊆ V the suppliers qualified for p. Failure model: every F with
//! |F| ≤ k, together with the explicit scenarios in F (∅ always included). A panel T is
//! feasible iff T ∩ (E_p \ F) ≠ ∅ for all p, F; then
//!     H = min{E_p \ F},   Tr(H) = the inclusion-minimal feasible panels  (MODEL, Thm 1).
//!
//! Sets of suppliers are sorted index vectors into V; labels appear only at the boundary.

use std::collections::HashMap;

use crate::bits::Bits;
use crate::thesis::{minimal, Mbf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    pub V: Vec<String>,
    pub P: Vec<String>,
    /// E[p] ⊆ V, ascending indices.
    pub E: Vec<Vec<usize>>,
    pub k: usize,
    /// Explicit failure scenarios, ascending indices.
    pub F: Vec<Vec<usize>>,
}

/// A failure scenario relevant to product p: (p, the failed suppliers).
pub type Scenario = (usize, Vec<usize>);

impl Instance {
    /// The instance, or the invariant it violates: distinct labels, one supplier set per
    /// product, qualification and failure sets inside V, k ≥ 0.
    pub fn new(V: Vec<String>, P: Vec<String>, E: Vec<Vec<String>>, k: i64, F: Vec<Vec<String>>)
               -> Result<Self, String> {
        if k < 0 {
            return Err("k must be a nonnegative integer".into());
        }
        let idx: HashMap<&str, usize> = V.iter().enumerate().map(|(i, v)| (v.as_str(), i)).collect();
        let mut seen = std::collections::HashSet::new();
        if idx.len() != V.len() || !P.iter().all(|p| seen.insert(p)) {
            return Err("V and P must have distinct labels".into());
        }
        if P.len() != E.len() {
            return Err("each product must have one supplier set".into());
        }
        let index = |S: &Vec<String>| -> Result<Vec<usize>, String> {
            let mut s: Vec<usize> = S.iter().map(|v| idx.get(v.as_str()).copied())
                .collect::<Option<_>>()
                .ok_or("qualification and failure sets must be subsets of V")?;
            s.sort_unstable();
            s.dedup();
            Ok(s)
        };
        let E = E.iter().map(index).collect::<Result<_, _>>()?;
        let F = F.iter().map(index).collect::<Result<_, _>>()?;
        Ok(Instance { V, P, E, k: k as usize, F })
    }

    pub fn with_k(&self, k: usize) -> Self {
        Instance { k, ..self.clone() }
    }

    /// Suppliers in some E_p, in V's order: the only ones a minimal panel can use.
    pub fn support(&self) -> Vec<usize> {
        let mut u = vec![false; self.V.len()];
        for e in &self.E {
            for &v in e {
                u[v] = true;
            }
        }
        (0..self.V.len()).filter(|&v| u[v]).collect()
    }

    /// Labels of a set of indices, sorted as strings.
    pub fn labels(&self, S: &[usize]) -> Vec<String> {
        let mut l: Vec<String> = S.iter().map(|&v| self.V[v].clone()).collect();
        l.sort();
        l
    }

    /// Indices of labels; None if some label is not in V.
    pub fn index(&self, S: &[String]) -> Option<Vec<usize>> {
        let mut s: Vec<usize> = S.iter().map(|x| self.V.iter().position(|v| v == x))
            .collect::<Option<_>>()?;
        s.sort_unstable();
        Some(s)
    }

    /// The failed sets that matter for product p: k-subsets of E_p (or E_p itself if
    /// |E_p| ≤ k), taken in label order, and each explicit F ∩ E_p. Supersets of these give
    /// only subsets of edges.
    pub fn scenarios(&self, p: usize) -> Vec<Vec<usize>> {
        let mut Ep = self.E[p].clone();
        Ep.sort_by(|a, b| self.V[*a].cmp(&self.V[*b]));
        let mut out = if Ep.len() <= self.k { vec![Ep.clone()] } else { combinations(&Ep, self.k) };
        for F in &self.F {
            out.push(self.E[p].iter().copied().filter(|v| F.contains(v)).collect());
        }
        out
    }
}

/// The r-subsets of `s` in lexicographic order of positions (itertools.combinations).
pub fn combinations<T: Clone>(s: &[T], r: usize) -> Vec<Vec<T>> {
    let n = s.len();
    if r > n {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut ix: Vec<usize> = (0..r).collect();
    loop {
        out.push(ix.iter().map(|&i| s[i].clone()).collect());
        let Some(i) = (0..r).rev().find(|&i| ix[i] != i + n - r) else { return out };
        ix[i] += 1;
        for j in i + 1..r {
            ix[j] = ix[j - 1] + 1;
        }
    }
}

/// H on the support, deduplicated and superset-reduced (both preserve Tr exactly).
///
/// OUTPUT  H ...... edges over support positions, vertex i = labels[i]
///         labels . support indices into V
///         prov ... per kept edge, every (p, F) generating it, F in label order
pub struct Scenarios<const W: usize> {
    pub H: Mbf<W>,
    pub labels: Vec<usize>,
    pub prov: Vec<Vec<Scenario>>,
}

/// The scenario hypergraph of I (see `Scenarios`).
pub fn hypergraph<const W: usize>(I: &Instance) -> Scenarios<W> {
    let labels = I.support();
    let mut pos = vec![usize::MAX; I.V.len()];
    for (i, &v) in labels.iter().enumerate() {
        pos[v] = i;
    }
    let mut prov: HashMap<Bits<W>, Vec<Scenario>> = HashMap::new();
    for p in 0..I.P.len() {
        for F in I.scenarios(p) {
            let e = Bits::of(I.E[p].iter().filter(|v| !F.contains(v)).map(|&v| pos[v]));
            prov.entry(e).or_default().push((p, F));
        }
    }
    let H = Mbf { n: labels.len(), edges: minimal(prov.keys().copied().collect()) };
    let prov = H.edges.iter().map(|e| prov[e].clone()).collect();
    Scenarios { H, labels, prov }
}

/// Bitset t over the support -> ascending supplier indices.
pub fn decode<const W: usize>(labels: &[usize], t: &Bits<W>) -> Vec<usize> {
    let mut s: Vec<usize> = t.iter().map(|i| labels[i]).collect();
    s.sort_unstable();
    s
}

/// Supplier indices -> bitset over support positions.
pub fn encode<const W: usize>(labels: &[usize], S: &[usize]) -> Bits<W> {
    Bits::of(S.iter().map(|v| labels.iter().position(|x| x == v).expect("in the support")))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn inst(V: &str, E: &[&str], k: i64, F: &[&str]) -> Instance {
        let s = |x: &str| x.chars().map(String::from).collect::<Vec<_>>();
        Instance::new(s(V), (0..E.len()).map(|j| format!("p{j}")).collect(),
                      E.iter().map(|e| s(e)).collect(), k, F.iter().map(|f| s(f)).collect())
            .unwrap()
    }

    #[test]
    fn duplicate_and_superset_edges_removed_with_provenance() {
        let I = inst("abc", &["ab", "ab", "abc"], 0, &[]);
        let S = hypergraph::<1>(&I);
        assert_eq!(S.H.len(), 1);
        assert_eq!(S.prov[0].iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn combinations_match_itertools() {
        assert_eq!(combinations(&[1, 2, 3], 2), vec![vec![1, 2], vec![1, 3], vec![2, 3]]);
        assert_eq!(combinations(&[1, 2], 0), vec![Vec::<i32>::new()]);
    }

    #[test]
    fn rejects_data_outside_the_model() {
        let s = |x: &str| x.chars().map(String::from).collect::<Vec<_>>();
        let ok = |V: &str, P: Vec<String>, E: &[&str], k, F: &[&str]| {
            Instance::new(s(V), P, E.iter().map(|e| s(e)).collect(), k,
                          F.iter().map(|f| s(f)).collect()).is_ok()
        };
        let p = vec!["p".to_string()];
        assert!(ok("abc", p.clone(), &["abc"], 0, &[]));
        assert!(!ok("aa", p.clone(), &["a"], 0, &[]));
        assert!(!ok("abc", p.clone(), &["d"], 0, &[]));
        assert!(!ok("abc", p.clone(), &["a"], 0, &["d"]));
        assert!(!ok("abc", vec![], &["a"], 0, &[]));
        assert!(!ok("abc", vec!["p".into(), "p".into()], &["a", "b"], 0, &[]));
        assert!(!ok("abc", p, &["a"], -1, &[]));
    }
}
