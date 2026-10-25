//! The command line: versioned documents, exact basket import, data selection, exit codes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use srl::json::{self, json, Value};

const SRL: &str = env!("CARGO_BIN_EXE_srl");

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("srl-cli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn srl(args: &[&str]) -> Output {
    Command::new(SRL).args(args).env_remove("SRL_DATA_DIR").env_remove("SRL_SNAPSHOT").output().unwrap()
}

fn read(p: &Path) -> Value {
    json::loads(&std::fs::read(p).unwrap()).unwrap()
}

fn write(p: &Path, v: &Value) {
    std::fs::write(p, v.to_string()).unwrap();
}

#[test]
fn analysis_arguments_fail_before_computation() {
    for args in [vec!["gaps", "--seeds", "0"], vec!["entities", "--seeds", "0"],
                 vec!["--snapshot", "QBL-2025-06-20.xlsx", "entities"]] {
        let o = srl(&args);
        assert_eq!(o.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&o.stderr).contains("panicked"));
    }
    let d = tmp("count-timeout");
    let p = d.join("i.json");
    write(&p, &json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}));
    for timeout in ["0", "-1", "NaN", "inf"] {
        assert_eq!(srl(&["count", p.to_str().unwrap(), "--timeout", timeout]).status.code(), Some(2));
    }
}

#[test]
fn versioned_pipeline_on_the_example_basket() {
    let d = tmp("pipeline");
    let (b, s, e, v) = (d.join("basket.json"), d.join("solution.json"), d.join("family.json"), d.join("verify.json"));
    let basket = root().join("examples/canned.csv");
    assert!(srl(&["build", "--basket", basket.to_str().unwrap(), "--k", "1", "-o", b.to_str().unwrap()]).status.success());
    assert!(srl(&["solve", b.to_str().unwrap(), "-o", s.to_str().unwrap()]).status.success());
    assert!(srl(&["enum", b.to_str().unwrap(), "--method", "berge", "-o", e.to_str().unwrap()]).status.success());
    let (B, S, E) = (read(&b), read(&s), read(&e));
    assert_eq!((B["schema_version"].as_str(), B["kind"].as_str(), B["variant"].as_str()), (Some("1.0"), Some("instance"), Some("strict")));
    assert_eq!(B["names"].as_object().unwrap().len(), B["V"].as_array().unwrap().len());
    assert_eq!((S["kind"].as_str(), S["status"].as_str()), (Some("solution"), Some("optimal")));
    assert_eq!(S["panel"].as_array().unwrap().len(), 2);
    assert_eq!(S["instance"]["names"], B["names"]);
    assert_eq!(E["panels"].as_array().unwrap().len(), 1284);
    assert_eq!((E["run"]["status"].as_str(), E["run"]["count"].as_u64()), (Some("complete"), Some(1284)));
    assert!(srl(&["verify", e.to_str().unwrap(), "-o", v.to_str().unwrap()]).status.success());
    let V = read(&v);
    assert_eq!((V["panels"].as_u64(), V["exhaustion"].as_str()), (Some(1284), Some("complete")));
    for m in ["fkb", "cuts", "mmcs"] {
        assert!(srl(&["enum", b.to_str().unwrap(), "--method", m, "-o", e.to_str().unwrap()]).status.success());
        assert_eq!(read(&e)["run"]["count"].as_u64(), Some(1284), "{m}");
    }
    assert!(srl(&["count", b.to_str().unwrap(), "-o", v.to_str().unwrap()]).status.success());
    assert_eq!(read(&v)["panels"].as_u64(), Some(1284));
}

#[test]
fn legacy_instance_gives_a_versioned_solution() {
    let d = tmp("legacy");
    let (p, o) = (d.join("legacy.json"), d.join("out.json"));
    write(&p, &json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}));
    assert!(srl(&["solve", p.to_str().unwrap(), "-o", o.to_str().unwrap()]).status.success());
    let S = read(&o);
    assert_eq!((S["schema_version"].as_str(), S["instance"]["kind"].as_str(), S["panel"].clone()), (Some("1.0"), Some("instance"), json!(["a"])));
    assert!(srl(&["enum", p.to_str().unwrap(), "--method", "exhaustive", "-o", o.to_str().unwrap()]).status.success());
    assert_eq!(read(&o)["kind"], "enumeration");
}

#[test]
fn reads_shell_redirected_json() {
    let d = tmp("encodings");
    let text = json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}).to_string();
    let p = d.join("i.json");
    let utf16: Vec<u8> = [0xff, 0xfe].into_iter().chain(text.encode_utf16().flat_map(u16::to_le_bytes)).collect();
    let bom: Vec<u8> = [0xef, 0xbb, 0xbf].into_iter().chain(text.bytes()).collect();
    for b in [text.as_bytes().to_vec(), bom, utf16] {
        std::fs::write(&p, b).unwrap();
        assert!(srl(&["solve", p.to_str().unwrap()]).status.success());
    }
}

#[test]
fn input_errors_exit_2_with_a_message() {
    let d = tmp("errors");
    let p = d.join("bad.json");
    for doc in [json!({"schema_version": "2.0", "V": [], "P": [], "E": [], "k": 0}), json!({"V": ["a", "a"], "P": ["p"], "E": [["a"]], "k": 0}),
                json!({"V": ["a"], "P": ["p"], "E": [["d"]], "k": 0}), json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0.5}),
                json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": true}), json!({"V": ["a"], "P": [], "E": [["a"]], "k": 0}), json!([1])] {
        write(&p, &doc);
        let o = srl(&["solve", p.to_str().unwrap()]);
        assert_eq!(o.status.code(), Some(2), "{doc}");
    }
    write(&p, &json!({"schema_version": "2.0", "V": [], "P": [], "E": [], "k": 0}));
    assert!(String::from_utf8_lossy(&srl(&["solve", p.to_str().unwrap()]).stderr).contains("unsupported schema version"));
    let o = srl(&["build", "beans pinto can", "--k", "-1"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("k must be a nonnegative integer"));
    write(&p, &json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0}));
    for t in ["0", "nan", "inf", "-1"] {
        assert_eq!(srl(&["enum", p.to_str().unwrap(), "--timeout", t]).status.code(), Some(2), "{t}");
    }
    assert_eq!(srl(&["enum", p.to_str().unwrap(), "--method", "milp"]).status.code(), Some(2));
    assert_eq!(srl(&["build", "not a product"]).status.code(), Some(1));
}

#[test]
fn invalid_basket_csv_is_rejected() {
    let d = tmp("csv");
    let p = d.join("basket.csv");
    for text in ["name\na\n", "product\n", "product\na\na\n", "product\na,b\n", "product\n\"\"\n"] {
        std::fs::write(&p, text).unwrap();
        assert_eq!(srl(&["build", "--basket", p.to_str().unwrap()]).status.code(), Some(2), "{text:?}");
    }
}

#[test]
fn verify_fails_for_invalid_or_incomplete_families() {
    let d = tmp("verify");
    let p = d.join("f.json");
    for panels in [json!([[]]), json!([["a"], ["a"]]), json!([["zz"]])] {
        let n = panels.as_array().unwrap().len();
        write(&p, &json!({"V": ["a"], "P": ["p"], "E": [["a"]], "k": 0, "panels": panels, "run": {"status": "complete", "count": n}}));
        let o = srl(&["verify", p.to_str().unwrap()]);
        assert_eq!(o.status.code(), Some(1));
        let out = json::loads(&o.stdout).unwrap();
        assert!(out["all_feasible_and_minimal"] == false || out["distinct_panels"] == false);
        assert_ne!(out.get("exhaustion").and_then(Value::as_str), Some("complete"));
    }
    write(&p, &json!({"V": ["a", "b"], "P": ["p"], "E": [["a", "b"]], "k": 0, "panels": [["a"]], "run": {"status": "complete", "count": 1}}));
    let o = srl(&["verify", p.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(json::loads(&o.stdout).unwrap()["exhaustion"], "missing");
}

#[test]
fn data_override_is_used_and_staleness_reported() {
    let d = tmp("override");
    for name in ["raw/QBL.xlsx", "manifest.json", "catalog/mappings.csv"] {
        let dst = d.join(name);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(root().join("data").join(name), dst).unwrap();
    }
    let mut m = read(&d.join("manifest.json"));
    m["QBL.xlsx"]["retrieved_utc"] = json!("1900-01-01T00:00:00Z");
    write(&d.join("manifest.json"), &m);
    let o = d.join("audit.json");
    assert!(srl(&["--data-dir", d.to_str().unwrap(), "audit", "-o", o.to_str().unwrap()]).status.success());
    assert_eq!(read(&o)["stale"], true);
    assert!(srl(&["audit", "-o", o.to_str().unwrap()]).status.success());
    assert_eq!((read(&o)["suppliers"].as_u64(), read(&o)["products"].as_u64()), (Some(981), Some(710)));
}

#[test]
fn snapshot_option_selects_a_manifest_workbook() {
    let d = tmp("snapshot");
    let o = d.join("audit.json");
    assert!(srl(&["--snapshot", "QBL-2025-06-20.xlsx", "audit", "-o", o.to_str().unwrap()]).status.success());
    assert_eq!((read(&o)["suppliers"].as_u64(), read(&o)["rows"].as_u64()), (Some(780), Some(784)));
    assert_eq!(srl(&["--snapshot", "QBL-1999.xlsx", "audit"]).status.code(), Some(2));
    // the public case data are refused before anything is written
    let o = srl(&["--snapshot", "QBL-2025-06-20.xlsx", "site", "--root", root().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("does not match the frozen manifest"));
}

#[test]
fn schema_is_the_published_one() {
    let o = srl(&["schema"]);
    assert!(o.status.success());
    assert_eq!(json::loads(&o.stdout).unwrap(), read(&root().join("schema/document-1.0.json")));
    assert_eq!(read(&root().join("schema/document-1.0.json")), read(&root().join("site/document-1.0.json")));
}

#[test]
fn bench_never_overwrites_a_saved_campaign() {
    let o = srl(&["bench", "--seed", "1", "-o", root().join("results/suite-seed1.json").to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(srl(&["bench", "--seed", "1"]).status.code(), Some(2));
}
