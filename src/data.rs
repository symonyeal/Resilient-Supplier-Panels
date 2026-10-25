//! Frozen data: compiled into the binary from data/, or read from SRL_DATA_DIR (a directory
//! with manifest.json, raw/<workbook> and catalog/mappings.csv) when that is set.
//! SRL_SNAPSHOT names the workbook (a manifest key); the default is the current QBL.xlsx.

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

fn dir() -> Option<PathBuf> {
    std::env::var_os("SRL_DATA_DIR").map(PathBuf::from)
}

/// The selected workbook's manifest key.
pub fn snapshot() -> String {
    std::env::var("SRL_SNAPSHOT").unwrap_or_else(|_| SNAPSHOTS[2].to_string())
}

/// Bytes of a data file by its path below data/.
pub fn bytes(name: &str) -> Result<Vec<u8>, String> {
    if let Some(d) = dir() {
        return std::fs::read(d.join(name)).map_err(|e| format!("{}: {e}", d.join(name).display()));
    }
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

/// SHA-256 of bytes, lowercase hex.
pub fn sha256(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
