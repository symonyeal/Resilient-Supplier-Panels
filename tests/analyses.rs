//! Saved scale results are checked against their matrices, independently of the solver.
use std::path::Path;

use srl::{certify, contracts, json, instance::Instance};

#[test]
fn whole_class_solutions_have_matrix_witnesses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(root.join("results/whole-classes-v0.4/summary.json")).unwrap();
    let rows = json::loads(&bytes).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 8);
    for r in rows.as_array().unwrap() {
        let file = format!("results/whole-classes-v0.4/{}-min{}.json",
                           r["class"].as_str().unwrap(), r["min_suppliers"].as_u64().unwrap());
        let (_, I): (_, Instance) = contracts::load(&root.join(file)).unwrap();
        let sol = &r["solution"];
        match sol["status"].as_str().unwrap() {
            "infeasible" => {
                assert!(certify::infeasibility(&I).is_some());
                assert_eq!(r["count"]["panels"], 0);
            }
            "optimal" => {
                let labels: Vec<String> = sol["panel"].as_array().unwrap().iter()
                    .map(|v| v.as_str().unwrap().to_string()).collect();
                let T = I.index(&labels).unwrap();
                assert!(certify::feasible(&I, &T));
                assert!(certify::minimal(&I, &T));
            }
            "timeout" => assert!(sol["panel"].is_null()),
            status => panic!("unexpected status {status}"),
        }
        if r["count"]["status"] != "complete" {
            assert!(r["count"]["panels"].is_null());
        }
    }
}
