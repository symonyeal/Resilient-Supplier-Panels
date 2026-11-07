//! Packaged frozen data and immutable Dataset selection. Only Dataset::select reads
//! SRL_DATA_DIR/SRL_SNAPSHOT; one command then uses the same captured bytes throughout.

use std::path::PathBuf;

/// The QBL workbooks in date order; the last is the current snapshot.
pub const SNAPSHOTS: [&str; 3] = ["QBL-2025-06-20.xlsx", "QBL-2025-11-22.xlsx", "QBL.xlsx"];

const QBL: [&[u8]; 3] = [include_bytes!("../data/raw/QBL-2025-06-20.xlsx"),
                         include_bytes!("../data/raw/QBL-2025-11-22.xlsx"),
                         include_bytes!("../data/raw/QBL.xlsx")];
const MAPPINGS: &[u8] = include_bytes!("../data/catalog/mappings.csv");
const IDENTITIES: &[u8] = include_bytes!("../data/catalog/identities.csv");
const OWNERS: &[u8] = include_bytes!("../data/catalog/owners.csv");
const USASPENDING: &[u8] = include_bytes!("../data/catalog/usaspending.json");
const MANIFEST: &[u8] = include_bytes!("../data/manifest.json");

/// The selected workbook's manifest key.
pub fn snapshot() -> String {
    SNAPSHOTS[2].to_string()
}

/// Packaged bytes of a data file by its path below data/.
pub fn bytes(name: &str) -> Result<Vec<u8>, String> {
    if let Some(i) = name.strip_prefix("raw/").and_then(|w| SNAPSHOTS.iter().position(|s| *s == w)) {
        return Ok(QBL[i].to_vec());
    }
    match name {
        "catalog/mappings.csv" => Ok(MAPPINGS.to_vec()),
        "catalog/identities.csv" => Ok(IDENTITIES.to_vec()),
        "catalog/owners.csv" => Ok(OWNERS.to_vec()),
        "catalog/usaspending.json" => Ok(USASPENDING.to_vec()),
        "manifest.json" => Ok(MANIFEST.to_vec()),
        _ => Err(format!("{name} is not packaged data")),
    }
}

/// Bytes of the selected workbook.
pub fn workbook() -> Result<Vec<u8>, String> {
    bytes(&format!("raw/{}", snapshot()))
}

pub fn text(name: &str) -> Result<String, String> {
    String::from_utf8(bytes(name)?).map_err(|e| format!("{name}: {e}"))
}

/// One immutable input selection. Environment options are read only at this boundary.
pub struct Dataset { snapshot:String, files:std::collections::BTreeMap<String,Vec<u8>> }
impl Dataset {
    pub fn select(dir:Option<PathBuf>,snapshot:Option<&str>)->Result<Self,String> {
        let dir=dir.or_else(||std::env::var_os("SRL_DATA_DIR").map(PathBuf::from));
        let snapshot=snapshot.map(str::to_owned).or_else(||std::env::var("SRL_SNAPSHOT").ok()).unwrap_or_else(crate::data::snapshot);
        if !SNAPSHOTS.contains(&snapshot.as_str()){return Err(format!("--snapshot {snapshot}: not one of {}",SNAPSHOTS.join(", ")));}
        let required=[format!("raw/{snapshot}"),"catalog/mappings.csv".into(),"manifest.json".into()];
        let names=SNAPSHOTS.iter().map(|s|format!("raw/{s}")).chain(["manifest.json","catalog/mappings.csv","catalog/identities.csv","catalog/owners.csv","catalog/usaspending.json"].map(str::to_owned));
        let mut files=std::collections::BTreeMap::new();
        for name in names {
            let content=if let Some(dir)=&dir {std::fs::read(dir.join(&name)).map_err(|e|format!("{}: {e}",dir.join(&name).display()))}else{bytes(&name)};
            match content {Ok(b)=>{files.insert(name,b);},Err(e) if required.contains(&name)=>return Err(e),Err(_)=>{}}
        }
        let dataset=Self{snapshot,files};
        let manifest=crate::json::loads(&dataset.bytes("manifest.json")?)?;
        if manifest[dataset.snapshot.as_str()]["sha256"]!=sha256(&dataset.workbook()?).as_str(){return Err("data does not match the frozen manifest".into());}
        Ok(dataset)
    }
    pub fn snapshot(&self)->&str {&self.snapshot}
    pub fn bytes(&self,name:&str)->Result<Vec<u8>,String>{self.files.get(name).cloned().ok_or_else(||format!("{name} is absent from the selected dataset"))}
    pub fn text(&self,name:&str)->Result<String,String>{String::from_utf8(self.bytes(name)?).map_err(|e|format!("{name}: {e}"))}
    pub fn workbook(&self)->Result<Vec<u8>,String>{self.bytes(&format!("raw/{}",self.snapshot))}
}

/// SHA-256 of bytes, lowercase hex.
pub fn sha256(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
