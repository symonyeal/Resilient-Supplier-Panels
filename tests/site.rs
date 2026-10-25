//! Saved campaigns and the published site: re-certified from the matrix, never rerun.

use std::path::Path;

use srl::certify::{self, Completeness};
use srl::instance::Instance;
use srl::json::{self, Value};
use srl::run::Clock;
use srl::solve::{self, Minimum};
use srl::{audit, bench, data, portfolio, qbl, report, verdicts};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> Value {
    json::loads(&std::fs::read(root().join(rel)).unwrap()).unwrap()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect()
}

fn snapshot() -> (Vec<qbl::Row>, audit::Snapshot) {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx").unwrap()).unwrap();
    let s = audit::snapshot(&rows, &data::text("catalog/mappings.csv").unwrap());
    (rows, s)
}

const CAMPAIGNS: [&str; 2] = ["results/suite-seed1.json", "results/suite-seed1-v0.3.json"];

#[test]
fn saved_minima_and_matrix_certificates() {
    let (_, s) = snapshot();
    let M = audit::matrix(&s, "strict");
    for c in CAMPAIGNS {
        for r in read(c)["instances"].as_array().unwrap() {
            let I = audit::basket(&M, &strs(&r["P"]), r["k"].as_u64().unwrap() as usize);
            let m = solve::minimum(&I, &[], &Clock::unlimited());
            assert_eq!(bench::minimum_status(&m), r["min"]["status"].as_str().unwrap());
            match m {
                Minimum::Optimal(T) => {
                    assert_eq!(Some(T.len() as u64), r["min"]["size"].as_u64());
                    assert!(certify::minimal(&I, &T) && certify::minimal(&I, &I.index(&strs(&r["min"]["panel"])).unwrap()));
                }
                _ => assert!(r["min"]["size"].is_null()),
            }
            assert_eq!(bench::infeasibility_json(&I), r["infeasibility"]);
            assert_eq!(bench::essential_json(&I), r["essential"]);
        }
    }
}

#[test]
fn saved_reports_rebuild_without_changes() {
    let (rows, s) = snapshot();
    let A = audit::audit(&rows, &s);
    for (c, md) in [(CAMPAIGNS[0], "docs/findings/results-seed1.md"), (CAMPAIGNS[1], "docs/findings/results-seed1-v0.3.md")] {
        let R = read(c);
        let name = Path::new(c).file_name().unwrap().to_string_lossy();
        let text = report::markdown(&R, &name, verdicts::score(&R, &A));
        let saved = std::fs::read_to_string(root().join(md)).unwrap().replace("\r\n", "\n");
        assert!(text == saved, "{md} differs from its rebuild");
    }
}

#[test]
fn published_cases_have_matrix_certificates_and_exhaustion() {
    let D = read("site/cases.json");
    for c in D["cases"].as_array().unwrap() {
        for (k, mode) in c["modes"].as_object().unwrap() {
            let d = &mode["family"];
            let E: Vec<Vec<String>> = d["E"].as_array().unwrap().iter().map(strs).collect();
            let I = Instance::new(strs(&d["V"]), strs(&d["P"]), E.clone(), k.parse().unwrap(), vec![]).unwrap();
            let C: Vec<Vec<usize>> = d["panels"].as_array().unwrap().iter().map(|T| I.index(&strs(T)).unwrap()).collect();
            assert_eq!(Some(C.len() as u64), mode["count"].as_u64());
            assert_eq!(mode["count"], d["run"]["count"]);
            assert!(C.iter().all(|T| certify::minimal(&I, T)));
            assert_eq!(certify::complete(&I, &C), Completeness::Complete);
            assert_eq!(C.iter().map(Vec::len).min().map(|m| m as u64), mode["minimum_size"].as_u64());
            assert_eq!(bench::essential_json(&I), mode["essential"]);
            for p in mode["panels"].as_array().unwrap() {
                let T = I.index(&strs(&p["members"])).unwrap();
                assert!(C.contains(&T));
                for o in c["outages"].as_array().unwrap() {
                    let J = Instance::new(I.V.clone(), I.P.clone(), E.clone(), 0, vec![strs(&o["failed"])]).unwrap();
                    let res = &p["outages"][o["id"].as_str().unwrap()];
                    assert_eq!(certify::feasible(&J, &T), res["covered"] == true);
                    let F = I.index(&strs(&o["failed"])).unwrap();
                    for (Ep, got) in I.E.iter().zip(res["suppliers"].as_array().unwrap()) {
                        let want: Vec<usize> = Ep.iter().copied().filter(|v| T.contains(v) && !F.contains(v)).collect();
                        assert_eq!(strs(got), I.labels(&want));
                    }
                }
            }
        }
    }
}

#[test]
fn published_data_matches_files_source_and_campaign() {
    let D = read("site/cases.json");
    let p = &D["provenance"];
    assert_eq!(p["data_sha256"].as_str().unwrap(), data::sha256(&std::fs::read(root().join("data/raw/QBL.xlsx")).unwrap()));
    assert_eq!(p["mappings_sha256"].as_str().unwrap(), data::sha256(&std::fs::read(root().join("data/catalog/mappings.csv")).unwrap()));
    assert_eq!(p["code_sha256"].as_str().unwrap(), portfolio::code_sha256(root()).unwrap(),
               "the site was built from other sources: run srl site");
    let js = std::fs::read_to_string(root().join("site/data.js")).unwrap();
    let js = js.trim().strip_prefix("window.DATA=").unwrap().strip_suffix(';').unwrap();
    assert_eq!(json::loads(js.as_bytes()).unwrap(), D);
    let (_, s) = snapshot();
    let E = portfolio::evidence(&read(CAMPAIGNS[1]), &audit::matrix(&s, "strict"), &portfolio::names(&s)).unwrap();
    assert_eq!(D["evidence"], E);
    assert_eq!((E["checks"]["matrix_records"].as_u64(), E["checks"]["partial"].as_u64()), (Some(88), Some(0)));
}

#[test]
fn evidence_keeps_every_record_and_honest_partial_counts() {
    let (_, s) = snapshot();
    let M = audit::matrix(&s, "strict");
    let R = read(CAMPAIGNS[0]);
    let E = portfolio::evidence(&R, &M, &portfolio::names(&s)).unwrap();
    let (a, b) = (E["records"].as_array().unwrap(), R["instances"].as_array().unwrap());
    assert_eq!((a.len(), b.len()), (88, 88));
    for (x, y) in a.iter().zip(b) {
        for key in ["class", "rule", "size", "k", "P", "min", "essential", "infeasibility", "runs", "count_exact"] {
            assert_eq!(x[key], y[key]);
        }
        assert_eq!(x["count_lower_bound"].as_u64(), y["runs"].as_array().unwrap().iter().map(|r| r["count"].as_u64().unwrap()).max());
        assert!(x.get("H_edges_before_reduction").is_none());
    }
    let partial: Vec<&Value> = a.iter().filter(|x| x["count_exact"].is_null()).collect();
    assert!(partial.len() == 4 && partial.iter().all(|x| x["count_lower_bound"].as_u64() > Some(0)));
    let mut bad = R.clone();
    bad["instances"][0]["min"]["panel"] = serde_json::json!([]);
    assert!(portfolio::evidence(&bad, &M, &portfolio::names(&s)).unwrap_err().contains("saved minimum"));
}
