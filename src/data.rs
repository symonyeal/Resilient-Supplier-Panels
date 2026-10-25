//! Frozen data: compiled into the binary from data/, or read from SRL_DATA_DIR (a directory
//! with manifest.json, raw/QBL.xlsx and catalog/mappings.csv) when that is set.

use std::path::PathBuf;

const QBL: &[u8] = include_bytes!("../data/raw/QBL.xlsx");
const MAPPINGS: &[u8] = include_bytes!("../data/catalog/mappings.csv");
const MANIFEST: &[u8] = include_bytes!("../data/manifest.json");

fn dir() -> Option<PathBuf> {
    std::env::var_os("SRL_DATA_DIR").map(PathBuf::from)
}

/// Bytes of a data file by its path below data/.
pub fn bytes(name: &str) -> Result<Vec<u8>, String> {
    if let Some(d) = dir() {
        return std::fs::read(d.join(name)).map_err(|e| format!("{}: {e}", d.join(name).display()));
    }
    match name {
        "raw/QBL.xlsx" => Ok(QBL.to_vec()),
        "catalog/mappings.csv" => Ok(MAPPINGS.to_vec()),
        "manifest.json" => Ok(MANIFEST.to_vec()),
        _ => Err(format!("{name} is not packaged data")),
    }
}

pub fn text(name: &str) -> Result<String, String> {
    String::from_utf8(bytes(name)?).map_err(|e| format!("{name}: {e}"))
}

/// SHA-256 of bytes, lowercase hex.
pub fn sha256(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
