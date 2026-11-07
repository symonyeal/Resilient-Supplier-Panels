//! Independent counterexamples at mathematical, data and publication boundaries.
use std::path::PathBuf;
use srl::{data,contracts,certify,instance::Instance,json::json};

fn root(name:&str)->PathBuf {
    let base=std::env::var_os("SRL_TEST_ROOT").map(PathBuf::from).unwrap_or_else(||PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-files"));
    let path=base.join(format!("{}-{name}",std::process::id()));std::fs::create_dir_all(&path).unwrap();path
}
#[test]
fn captured_dataset_does_not_follow_later_file_edits() {
    let root=root("dataset");
    for name in ["raw/QBL.xlsx","catalog/mappings.csv","manifest.json"] {
        let path=root.join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,data::bytes(name).unwrap()).unwrap();
    }
    let selected=data::Dataset::select(Some(root.clone()),Some("QBL.xlsx")).unwrap();
    let original=selected.workbook().unwrap();std::fs::write(root.join("raw/QBL.xlsx"),b"changed").unwrap();
    assert_eq!(selected.workbook().unwrap(),original);
    assert!(data::Dataset::select(Some(root),Some("QBL.xlsx")).is_err());
}
#[test]
fn a_campaign_filename_cannot_be_overwritten_by_another_writer() {
    let path=root("campaign").join(format!("{}.json",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    contracts::write_once(&json!({"accepted":1}),&path).unwrap();let previous=std::fs::read(&path).unwrap();
    assert!(contracts::write_once(&json!({"accepted":2}),&path).is_err());assert_eq!(std::fs::read(path).unwrap(),previous);
}
#[test]
fn raw_panel_indices_and_duplicate_members_are_rejected() {
    let I=Instance::new(vec!["a".into()],vec!["p".into()],vec![vec!["a".into()]],0,vec![]).unwrap();
    assert!(!certify::feasible(&I,&[0,0]));assert!(!certify::feasible(&I,&[9]));
    assert!(I.index(&["a".into(),"a".into()]).is_none());
    assert!(Instance::new(vec!["a".into()],vec!["p".into()],vec![vec!["a".into(),"a".into()]],0,vec![]).is_err());
    assert!(Instance::new(vec![" ".into()],vec![],vec![],0,vec![]).is_err());
    assert!(!certify::feasible(&I.with_k(usize::MAX),&[0]));
}
