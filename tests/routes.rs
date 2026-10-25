//! Instance -> panels -> certificates on edge cases, and agreement of independent routes on
//! random instances. Expected values are worked by hand (comments) or come from exhaustive
//! search over the matrix, which shares no code with H, Berge, FK-B or MMCS.

use std::collections::BTreeSet;

use srl::bench::{self, LIMITS};
use srl::certify::{self, Completeness};
use srl::instance::Instance;
use srl::pyrand::Random;
use srl::run::Clock;
use srl::solve::{self, Minimum};
use srl::quotient;

type Family = BTreeSet<Vec<usize>>;

fn inst(V: &str, E: &[&str], k: i64, F: &[&str]) -> Instance {
    let s = |x: &str| x.chars().map(String::from).collect::<Vec<_>>();
    Instance::new(s(V), (0..E.len()).map(|j| format!("p{j}")).collect(), E.iter().map(|e| s(e)).collect(), k,
                  F.iter().map(|f| s(f)).collect()).unwrap()
}

fn fam(I: &Instance, C: &Family) -> BTreeSet<String> {
    C.iter().map(|T| I.labels(T).concat()).collect()
}

/// Every route's family, asserted equal; also the class count.
fn all_routes(I: &Instance) -> Family {
    let X: Family = solve::exhaustive(I).into_iter().collect();
    for m in bench::ROUTES {
        let mut C = Family::new();
        bench::route(m, I, &Clock::unlimited(), &LIMITS, &mut |T| { assert!(C.insert(T), "{m} repeated a panel"); true }).unwrap();
        assert_eq!(C, X, "{m} on {I:?}");
    }
    assert_eq!(quotient::count(I, &Clock::unlimited()).unwrap().panels, X.len() as u128);
    X
}

#[test]
fn empty_basket_has_only_the_empty_panel() {
    let I = inst("ab", &[], 0, &[]);
    assert_eq!(all_routes(&I), Family::from([vec![]]));
}

#[test]
fn empty_edge_is_infeasible_with_witness() {
    let I = inst("ab", &["a", ""], 0, &[]);
    assert!(all_routes(&I).is_empty());
    assert_eq!(certify::infeasibility(&I), Some((1, vec![])));
    assert_eq!(solve::minimum(&I, &[], &Clock::unlimited()), Minimum::Infeasible);
    assert_eq!(certify::essential(&I), None);
}

#[test]
fn single_supplier_product_is_essential_and_kills_resilience() {
    // E = {a, bc}: k=0 panels ab, ac with a essential; k=1 needs |E_p| ≥ 2, fails on p
    let I = inst("abc", &["a", "bc"], 0, &[]);
    assert_eq!(fam(&I, &all_routes(&I)), BTreeSet::from(["ab".into(), "ac".into()]));
    assert_eq!(certify::essential(&I).unwrap().keys().copied().collect::<Vec<_>>(), vec![0]);
    assert!(all_routes(&I.with_k(1)).is_empty());
    assert_eq!(certify::infeasibility(&I.with_k(1)).unwrap().0, 0);
}

#[test]
fn duplicate_and_superset_edges_leave_tr_unchanged() {
    let I = inst("abc", &["ab", "ab", "abc"], 0, &[]);
    assert_eq!(fam(&I, &all_routes(&I)), BTreeSet::from(["a".into(), "b".into()]));
}

#[test]
fn identical_signatures_both_needed_for_resilience() {
    // E_p = E_q = {a, b}: interchangeable for k=0 (panels a, b), both needed for k=1
    let I = inst("abc", &["ab", "ab"], 0, &[]);
    assert_eq!(fam(&I, &all_routes(&I)), BTreeSet::from(["a".into(), "b".into()]));
    assert_eq!(fam(&I, &all_routes(&I.with_k(1))), BTreeSet::from(["ab".into()]));
}

#[test]
fn correlated_scenario() {
    // E_p = abc, a and b fail together (F = ab): edges abc and c, so Tr = {c}
    let I = inst("abc", &["abc"], 0, &["ab"]);
    assert_eq!(fam(&I, &all_routes(&I)), BTreeSet::from(["c".into()]));
    assert_eq!(certify::essential(&I).unwrap().keys().copied().collect::<Vec<_>>(), vec![2]);
}

#[test]
fn hand_example() {
    // E = ab, bc, cd: Tr = {ac, bc, bd} (ad misses bc); k = 1 forces abcd
    let I = inst("abcd", &["ab", "bc", "cd"], 0, &[]);
    assert_eq!(fam(&I, &all_routes(&I)), BTreeSet::from(["ac".into(), "bc".into(), "bd".into()]));
    assert_eq!(fam(&I, &all_routes(&I.with_k(1))), BTreeSet::from(["abcd".into()]));
    let W = certify::witnesses(&I, &[1, 3]);
    assert!(W.iter().all(|(v, w)| {
        let (p, F) = w.as_ref().unwrap();
        F.is_empty() && I.E[*p].iter().filter(|u| [1, 3].contains(u)).collect::<Vec<_>>() == [v]
    }));
}

fn random_instance(rng: &mut Random, n: usize, m: usize, k: usize, scenarios: bool) -> Instance {
    let V: Vec<String> = (0..n).map(|i| format!("s{i}")).collect();
    let E: Vec<Vec<String>> = (0..m).map(|_| { let r = rng.randint(1, (n / 2).max(1) as i64) as usize; rng.sample(&V, r) }).collect();
    let F: Vec<Vec<String>> = if scenarios {
        (0..rng.randint(1, 3)).map(|_| { let r = rng.randint(1, 3.min(n) as i64) as usize; rng.sample(&V, r) }).collect()
    } else { Vec::new() };
    Instance::new(V, (0..m).map(|j| format!("p{j}")).collect(), E, k as i64, F).unwrap()
}

fn certificates(I: &Instance, Tr: &Family) {
    assert!(Tr.iter().all(|T| certify::minimal(I, T)));
    let C: Vec<Vec<usize>> = Tr.iter().cloned().collect();
    assert_eq!(certify::complete(I, &C), Completeness::Complete);
    if let Some(T) = C.first() {
        assert_eq!(certify::complete(I, &C[1..]), Completeness::Missing(T.clone()));
        let inter: BTreeSet<usize> = I.support().into_iter().filter(|v| C.iter().all(|T| T.contains(v))).collect();
        assert_eq!(inter, certify::essential(I).unwrap().keys().copied().collect());
        let Minimum::Optimal(m) = solve::minimum(I, &[], &Clock::unlimited()) else { panic!("feasible") };
        assert_eq!(m.len(), C.iter().map(Vec::len).min().unwrap());
    } else {
        assert!(certify::infeasibility(I).is_some());
    }
}

#[test]
fn random_agreement_certificates_and_permutation_invariance() {
    let mut rng = Random::new(0);
    for trial in 0..200 {
        let n = rng.randint(3, 12) as usize;
        let m = rng.randint(1, 7) as usize;
        let I = random_instance(&mut rng, n, m, trial % 3, false);
        let Tr = all_routes(&I);
        certificates(&I, &Tr);
        // suppliers and products reordered
        let mut pi = I.V.clone();
        rng.shuffle(&mut pi);
        let E: Vec<Vec<String>> = I.E.iter().rev().map(|E| I.labels(E)).collect();
        let J = Instance::new(pi, I.P.iter().rev().cloned().collect(), E, I.k as i64, vec![]).unwrap();
        let TJ: BTreeSet<Vec<String>> = all_routes(&J).iter().map(|T| J.labels(T)).collect();
        assert_eq!(TJ, Tr.iter().map(|T| I.labels(T)).collect());
    }
}

#[test]
fn random_agreement_with_correlated_scenarios_and_k() {
    let mut rng = Random::new(1);
    for trial in 0..150 {
        let n = rng.randint(3, 11) as usize;
        let m = rng.randint(1, 6) as usize;
        let I = random_instance(&mut rng, n, m, trial % 2, true);
        let Tr = all_routes(&I);
        certificates(&I, &Tr);
    }
}

#[test]
fn signature_classes_break_on_scenario_membership() {
    // all pairs except ab are feasible; cross-class pairs and cd are two orbits
    let I = inst("abcd", &["abcd"], 1, &["ab"]);
    let C: Vec<Vec<usize>> = solve::exhaustive(&I);
    let (st, sig) = bench::stats(&I);
    assert_eq!(bench::orbits(&C, &sig), 2);
    assert_eq!(sig, vec![vec![0, 1], vec![2, 3]]);
    assert_eq!(st["signature_classes"], 2);
}

#[test]
fn stats_count_edges_before_superset_reduction() {
    let (st, _) = bench::stats(&inst("abc", &["ab", "ab", "abc"], 0, &[]));
    assert_eq!((st["H_edges_before_reduction"].as_u64(), st["H_edges_after_reduction"].as_u64()), (Some(3), Some(1)));
}

#[test]
fn completeness_rejects_invalid_families() {
    let I = inst("abc", &["abc"], 0, &[]);
    assert_eq!(certify::complete(&I, &[vec![]]), Completeness::Invalid);
    assert_eq!(certify::complete(&I, &[vec![0, 1]]), Completeness::Invalid);
    let J = inst("", &[], 0, &[]);
    assert_eq!(certify::complete(&J, &[]), Completeness::Missing(vec![]));
    assert_eq!(certify::complete(&J, &[vec![]]), Completeness::Complete);
}

/// Three products with 30 disjoint suppliers each, k = 1: |Tr| = C(30, 2)^3 = 435^3.
fn big() -> Instance {
    let V: Vec<String> = (0..90).map(|i| format!("s{i:02}")).collect();
    let E = (0..3).map(|j| V[30 * j..30 * j + 30].to_vec()).collect();
    Instance::new(V, vec!["a".into(), "b".into(), "c".into()], E, 1, vec![]).unwrap()
}

#[test]
fn timeout_is_partial_and_every_returned_panel_is_certified() {
    let I = big();
    for m in ["fkb", "cuts"] {
        let (panels, r) = bench::run(m, &I, 0.5, 1_000_000, &LIMITS);
        assert_eq!(r["status"], "timeout", "{m}");
        assert!(r["t_total"].is_null());
        assert!(panels.iter().all(|T| certify::minimal(&I, T)));
    }
    let q = quotient::count(&I, &Clock::unlimited()).unwrap();
    assert_eq!((q.panels, q.orbits), (435u128.pow(3), 1));
}

#[test]
fn cap_stops_a_stream_and_berge_counts_exactly() {
    let (_, r) = bench::run("cuts", &big(), 30.0, 3, &LIMITS);
    assert_eq!((r["status"].as_str(), r["count"].as_u64(), r["stored"].as_u64()), (Some("cap"), Some(4), Some(3)));
    let (_, r) = bench::run("mmcs", &big(), 30.0, 3, &LIMITS);
    assert_eq!((r["status"].as_str(), r["count"].as_u64(), r["stored"].as_u64()), (Some("cap"), Some(4), Some(3)));
    let (_, r) = bench::run("berge", &inst("abcd", &["ab", "bc", "cd"], 0, &[]), 30.0, 2, &LIMITS);
    assert_eq!((r["status"].as_str(), r["count"].as_u64(), r["stored"].as_u64()), (Some("complete"), Some(3), Some(2)));
}

#[test]
fn a_failed_route_is_an_error_not_a_censored_timeout() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let (_, r) = bench::run("no such method", &inst("ab", &["ab"], 0, &[]), 30.0, 10, &LIMITS);
    std::panic::set_hook(prev);
    assert_eq!((r["status"].as_str(), r["count"].as_u64()), (Some("error"), Some(0)));
}

#[test]
fn berge_working_set_limit_is_reported_as_memory() {
    let L = bench::Limits { berge_sets: 1000, ..LIMITS };
    let (_, r) = bench::run("berge", &big(), 30.0, 10, &L);
    assert_eq!(r["status"], "memory");
}
