//! Version 1.0 exchange documents; legacy instance JSON (no version, no kind) remains readable.

use std::path::Path;

use crate::instance::Instance;
use crate::json::{self, json, Map, Style, Value};
use crate::{catalog, data, THESIS_REV};

pub const SCHEMA_VERSION: &str = "1.0";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SCHEMA: &str = include_str!("../schema/document-1.0.json");

fn labels(v: &Value, what: &str) -> Result<Vec<String>, String> {
    v.as_array().ok_or(format!("{what} must be a list of labels"))?.iter()
        .map(|x| x.as_str().map(String::from).ok_or(format!("{what} must hold strings")))
        .collect()
}

fn sets(v: &Value, what: &str) -> Result<Vec<Vec<String>>, String> {
    v.as_array().ok_or(format!("{what} must be a list of sets"))?.iter().map(|s| labels(s, what)).collect()
}

/// An instance or enumeration document and its instance, checking the version and the
/// matrix invariants.
pub fn load(path: &Path) -> Result<(Map<String, Value>, Instance), String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let Value::Object(d) = json::loads(&b)? else {
        return Err("expected an instance or enumeration JSON object".into());
    };
    let v = d.get("schema_version").map_or(Some(SCHEMA_VERSION), Value::as_str);
    if v != Some(SCHEMA_VERSION) {
        return Err(format!("unsupported schema version: {}", d["schema_version"]));
    }
    if !matches!(d.get("kind").map_or(Some("instance"), Value::as_str), Some("instance" | "enumeration")) {
        return Err("expected an instance or enumeration document".into());
    }
    let get = |k: &str| d.get(k).ok_or(format!("missing field {k}"));
    let k = get("k")?;
    let k = k.as_i64().filter(|_| k.is_i64() || k.is_u64()).ok_or("k must be a nonnegative integer")?;
    let F = d.get("F").map_or(Ok(Vec::new()), |f| sets(f, "F"))?;
    let I = Instance::new(labels(get("V")?, "V")?, labels(get("P")?, "P")?, sets(get("E")?, "E")?, k, F)?;
    Ok((d, I))
}

/// Content hashes and versions used to interpret the matrix.
pub fn provenance() -> Result<Map<String, Value>, String> {
    let mut m = Map::new();
    m.insert("data_sha256".into(), json!(data::sha256(&data::bytes("raw/QBL.xlsx")?)));
    m.insert("mappings_sha256".into(), json!(data::sha256(&data::bytes("catalog/mappings.csv")?)));
    m.insert("software_version".into(), json!(VERSION));
    m.insert("thesis_rev".into(), json!(THESIS_REV));
    Ok(m)
}

/// A versioned instance with ordered labels and sorted qualification and failure sets.
pub fn document(I: &Instance) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("schema_version".into(), json!(SCHEMA_VERSION));
    m.insert("kind".into(), json!("instance"));
    m.insert("V".into(), json!(I.V));
    m.insert("P".into(), json!(I.P));
    m.insert("E".into(), json!(I.E.iter().map(|E| I.labels(E)).collect::<Vec<_>>()));
    m.insert("k".into(), json!(I.k));
    m.insert("F".into(), json!(I.F.iter().map(|F| I.labels(F)).collect::<Vec<_>>()));
    m
}

/// `base` updated by `over`: existing keys keep their place (Python `dict(d, **over)`).
pub fn update(mut base: Map<String, Value>, over: Map<String, Value>) -> Map<String, Value> {
    for (k, v) in over {
        base.insert(k, v);
    }
    base
}

/// Distinct, nonempty exact product tokens from a UTF-8 CSV with one column named product.
pub fn basket_csv(path: &Path) -> Result<Vec<String>, String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8(b.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&b).to_vec())
        .map_err(|e| e.to_string())?;
    let rows = catalog::csv(&text);
    if rows.first().map(|h| h == &["product"]) != Some(true) {
        return Err("basket CSV must have exactly one column named product".into());
    }
    if rows[1..].iter().any(|r| r.len() != 1) {
        return Err("each basket CSV row must have exactly one product".into());
    }
    let P: Vec<String> = rows[1..].iter().map(|r| crate::qbl::strip(&r[0]).to_string()).collect();
    let distinct = P.iter().collect::<std::collections::HashSet<_>>().len() == P.len();
    if P.is_empty() || P.iter().any(String::is_empty) || !distinct {
        return Err("basket CSV must contain distinct, nonempty products".into());
    }
    Ok(P)
}

/// JSON as UTF-8 with a final newline, to `path` or standard output.
pub fn write(d: &Value, path: Option<&Path>) -> Result<(), String> {
    let text = json::dumps(d, Style::Indent(1)) + "\n";
    match path {
        Some(p) => {
            if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(p, text).map_err(|e| format!("{}: {e}", p.display()))
        }
        None => {
            use std::io::Write;
            std::io::stdout().write_all(text.as_bytes()).map_err(|e| e.to_string())
        }
    }
}
