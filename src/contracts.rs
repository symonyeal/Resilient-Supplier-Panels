//! Version 1.0 exchange documents; legacy instance JSON (no version, no kind) remains readable.

use std::path::Path;

use crate::instance::Instance;
use crate::json::{self, Map, Style, Value, json};
use crate::{THESIS_REV, catalog, data};

pub const SCHEMA_VERSION: &str = "1.0";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SCHEMA: &str = include_str!("../schema/document-1.0.json");

pub fn labels(v: &Value, what: &str) -> Result<Vec<String>, String> {
    v.as_array()
        .ok_or(format!("{what} must be a list of labels"))?
        .iter()
        .map(|x| {
            x.as_str()
                .map(String::from)
                .ok_or(format!("{what} must hold strings"))
        })
        .collect()
}

/// An exact nonnegative count; decimal strings preserve values above JSON's u64.
pub fn count(v: &Value) -> Result<u128, String> {
    if let Some(n) = v.as_u64() {
        return Ok(n as u128);
    }
    if let Some(s) = v
        .as_str()
        .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
    {
        return s.parse().map_err(|_| "Count exceeds u128".into());
    }
    Err("Count must be a nonnegative integer".into())
}

fn sets(v: &Value, what: &str) -> Result<Vec<Vec<String>>, String> {
    v.as_array()
        .ok_or(format!("{what} must be a list of sets"))?
        .iter()
        .map(|s| labels(s, what))
        .collect()
}

/// An instance or enumeration document and its instance, checking the version and the
/// matrix invariants.
pub fn load(path: &Path) -> Result<(Map<String, Value>, Instance), String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let Value::Object(d) = json::loads(&b)? else {
        return Err("expected an instance or enumeration JSON object".into());
    };
    let v = d
        .get("schema_version")
        .map_or(Some(SCHEMA_VERSION), Value::as_str);
    if v != Some(SCHEMA_VERSION) {
        return Err(format!(
            "unsupported schema version: {}",
            d["schema_version"]
        ));
    }
    if !matches!(
        d.get("kind").map_or(Some("instance"), Value::as_str),
        Some("instance" | "enumeration")
    ) {
        return Err("expected an instance or enumeration document".into());
    }
    let get = |k: &str| d.get(k).ok_or(format!("missing field {k}"));
    let k = get("k")?;
    let k = k
        .as_i64()
        .filter(|_| k.is_i64() || k.is_u64())
        .ok_or("k must be a nonnegative integer")?;
    let F = d.get("F").map_or(Ok(Vec::new()), |f| sets(f, "F"))?;
    let I = Instance::new(
        labels(get("V")?, "V")?,
        labels(get("P")?, "P")?,
        sets(get("E")?, "E")?,
        k,
        F,
    )?;
    Ok((d, I))
}

/// Content hashes and versions used to interpret the matrix.
pub fn provenance(dataset: &data::Dataset) -> Result<Map<String, Value>, String> {
    let mut m = Map::new();
    m.insert(
        "data_sha256".into(),
        json!(data::sha256(&dataset.workbook()?)),
    );
    m.insert(
        "mappings_sha256".into(),
        json!(data::sha256(&dataset.bytes("catalog/mappings.csv")?)),
    );
    m.insert("software_version".into(), json!(VERSION));
    m.insert("thesis_rev".into(), json!(THESIS_REV));
    Ok(m)
}

/// A versioned instance with ordered labels and sorted qualification and failure sets.
pub fn document(I: &Instance) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("schema_version".into(), json!(SCHEMA_VERSION));
    m.insert("kind".into(), json!("instance"));
    m.insert("V".into(), json!(I.V()));
    m.insert("P".into(), json!(I.P()));
    m.insert(
        "E".into(),
        json!(I.E().iter().map(|E| I.labels(E)).collect::<Vec<_>>()),
    );
    m.insert("k".into(), json!(I.k()));
    m.insert(
        "F".into(),
        json!(I.F().iter().map(|F| I.labels(F)).collect::<Vec<_>>()),
    );
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
    let P: Vec<String> = rows[1..]
        .iter()
        .map(|r| crate::qbl::strip(&r[0]).to_string())
        .collect();
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
            atomic(p, text.as_bytes(), false)
        }
        None => {
            use std::io::Write;
            std::io::stdout()
                .write_all(text.as_bytes())
                .map_err(|e| e.to_string())
        }
    }
}

/// One cooperating writer per output file; operating-system ownership ends on process exit.
pub fn claim(path: &Path) -> Result<std::fs::File, String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("writer.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("Output writer already active: {e}"))?;
    Ok(lock)
}
fn atomic(path: &Path, bytes: &[u8], once: bool) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let candidate = path.with_extension(format!(
        "{}-{}.candidate",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    if once {
        std::fs::hard_link(&candidate, path).map_err(|e| e.to_string())?;
        std::fs::remove_file(candidate).map_err(|e| e.to_string())
    } else {
        std::fs::rename(candidate, path).map_err(|e| e.to_string())
    }
}
/// Publish a complete campaign once; an existing filename is never replaced.
pub fn write_once(d: &Value, path: &Path) -> Result<(), String> {
    atomic(path, json::dumps(d, Style::Indent(1)).as_bytes(), true)
}
