//! srl {acquire, audit, schema, build, solve, enum, count, verify, bench, report, site}.
//! The shell: parse arguments, read documents and data, call the library, write JSON.
//! Exit status 2 is an argument or input error, 1 a failed verification or build check.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use srl::audit::{self, Snapshot};
use srl::bench::{self, LIMITS};
use srl::certify::{self, Completeness};
use srl::contracts::{self, update, SCHEMA, SCHEMA_VERSION, VERSION};
use srl::instance::Instance;
use srl::json::{self, json, Map, Style, Value};
use srl::run::Clock;
use srl::solve::{self, Minimum};
use srl::{data, portfolio, qbl, quotient, report, verdicts, THESIS_REV};

const USAGE: &str = "usage: srl [--version] [--data-dir PATH] COMMAND ...

commands:
  acquire                          download the sources to data/fetched (network, curl)
  audit [-o FILE]                  Stage 0 audit of the frozen snapshot
  schema [-o FILE]                 JSON Schema for version 1.0 exchange documents
  build [PRODUCT ...] [--basket CSV] [--k N] [--variant strict|merged] [--fail ID,ID ...] [-o FILE]
                                   instance document for a basket
  solve FILE [-o FILE]             minimum panel, witnesses, essential suppliers
  enum FILE [--method berge|fkb|cuts|mmcs|exhaustive] [--timeout S] [--cap N] [-o FILE]
                                   minimal panels
  count FILE [--timeout S] [-o FILE]
                                   exact number of minimal panels by signature classes
  verify FILE [-o FILE]            certify an enumeration document
  bench --seed N -o FILE [--timeout S] [--no-variants]
                                   benchmark suite (docs/BENCHMARKS.md)
  report FILE [-o FILE]            findings tables from a saved suite
  site [--results FILE] [--root DIR]
                                   rebuild the public case data in DIR/site";

/// Results the public site presents.
const CAMPAIGN: &str = "results/suite-seed1-v0.3.json";

struct Args {
    cmd: String,
    pos: Vec<String>,
    opt: Vec<(String, String)>,
    flags: Vec<String>,
}

impl Args {
    fn get(&self, k: &str) -> Option<&str> {
        self.opt.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn all(&self, k: &str) -> Vec<&str> {
        self.opt.iter().filter(|(n, _)| n == k).map(|(_, v)| v.as_str()).collect()
    }

    fn out(&self) -> Option<PathBuf> {
        self.get("-o").map(PathBuf::from)
    }

    fn num<T: std::str::FromStr>(&self, k: &str, default: T) -> Result<T, String> {
        self.get(k).map_or(Ok(default), |v| v.parse().map_err(|_| format!("argument {k}: invalid value {v:?}")))
    }
}

const VALUED: [&str; 11] = ["-o", "--output", "--basket", "--k", "--variant", "--fail", "--method", "--timeout",
                            "--cap", "--seed", "--results"];

fn parse(argv: Vec<String>) -> Result<Option<Args>, String> {
    let mut it = argv.into_iter();
    let mut a = Args { cmd: String::new(), pos: Vec::new(), opt: Vec::new(), flags: Vec::new() };
    while let Some(x) = it.next() {
        let name = if x == "--output" { "-o".to_string() } else { x.clone() };
        if x == "--version" {
            println!("srl {VERSION}");
            return Ok(None);
        } else if x == "-h" || x == "--help" {
            println!("{USAGE}");
            return Ok(None);
        } else if x == "--data-dir" || x == "--root" || VALUED.contains(&x.as_str()) {
            let v = it.next().ok_or(format!("argument {x}: expected one argument"))?;
            a.opt.push((name, v));
        } else if x == "--no-variants" {
            a.flags.push(x);
        } else if x.starts_with("--") {
            return Err(format!("unrecognized arguments: {x}"));
        } else if a.cmd.is_empty() {
            a.cmd = x;
        } else {
            a.pos.push(x);
        }
    }
    if a.cmd.is_empty() {
        return Err("the following arguments are required: COMMAND".into());
    }
    Ok(Some(a))
}

enum Fail {
    Usage(String),
    Check(String),
    Silent,
}

impl From<String> for Fail {
    fn from(s: String) -> Self {
        Fail::Usage(s)
    }
}

impl From<&str> for Fail {
    fn from(s: &str) -> Self {
        Fail::Usage(s.into())
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let res = parse(argv).map_err(Fail::Usage).and_then(|a| match a {
        None => Ok(()),
        Some(a) => {
            if let Some(d) = a.get("--data-dir") {
                let d = std::fs::canonicalize(d).map_err(|e| Fail::Usage(format!("--data-dir {d}: {e}")))?;
                // SAFETY: set once, before any other thread exists.
                unsafe { std::env::set_var("SRL_DATA_DIR", d) };
            }
            command(&a)
        }
    });
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail::Usage(m)) => {
            eprintln!("{USAGE}\nsrl: error: {m}");
            ExitCode::from(2)
        }
        Err(Fail::Check(m)) => {
            eprintln!("{m}");
            ExitCode::from(1)
        }
        Err(Fail::Silent) => ExitCode::from(1),
    }
}

fn snapshot() -> Result<(Vec<qbl::Row>, Snapshot), String> {
    let rows = qbl::read(&data::bytes("raw/QBL.xlsx")?)?;
    let s = audit::snapshot(&rows, &data::text("catalog/mappings.csv")?);
    Ok((rows, s))
}

fn file(a: &Args) -> Result<(Map<String, Value>, Instance), Fail> {
    let f = a.pos.first().ok_or("the following arguments are required: FILE".to_string())?;
    Ok(contracts::load(Path::new(f))?)
}

fn command(a: &Args) -> Result<(), Fail> {
    let out = a.out();
    let out = out.as_deref();
    match a.cmd.as_str() {
        "acquire" => println!("{}", json::dumps(&acquire()?, Style::Indent(1))),
        "audit" => {
            let (rows, s) = snapshot()?;
            let m = json::loads(&data::bytes("manifest.json")?)?;
            let r = m["QBL.xlsx"]["retrieved_utc"].as_str().ok_or("manifest without retrieved_utc")?.to_string();
            let mut A = audit::audit(&rows, &s);
            A["snapshot_age_days"] = json!(qbl::age_days(&r, &today()));
            A["stale"] = json!(qbl::stale(&r, &today()));
            contracts::write(&A, out)?;
        }
        "schema" => contracts::write(&json::loads(SCHEMA.as_bytes())?, out)?,
        "build" => build(a, out)?,
        "solve" => {
            let (d, I) = file(a)?;
            let m = solve::minimum(&I, &[], &Clock::unlimited());
            let T = match &m { Minimum::Optimal(T) => Some(T), _ => None };
            let W = T.map(|T| -> Value {
                certify::witnesses(&I, T).iter().map(|(v, w)| (I.V[*v].clone(),
                    w.as_ref().map_or(Value::Null, |w| bench::witness(&I, w)))).collect::<Map<_, _>>().into()
            });
            contracts::write(&json!({"schema_version": SCHEMA_VERSION, "kind": "solution", "software_version": VERSION,
                "instance": update(d, contracts::document(&I)), "status": bench::minimum_status(&m),
                "panel": T.map(|T| I.labels(T)), "witnesses": W, "infeasibility": bench::infeasibility_json(&I),
                "essential": bench::essential_json(&I)}), out)?;
        }
        "enum" => {
            let (d, I) = file(a)?;
            let method = a.get("--method").unwrap_or("berge");
            let (C, r) = if method == "exhaustive" {
                let C = solve::exhaustive(&I);
                let n = C.len();
                (C, json!({"method": "exhaustive", "status": "complete", "count": n, "stored": n}))
            } else if bench::ROUTES.contains(&method) {
                let timeout: f64 = a.num("--timeout", LIMITS.timeout_s)?;
                if !timeout.is_finite() || timeout <= 0.0 {
                    return Err(Fail::Usage("timeout must be finite and positive".into()));
                }
                bench::run(method, &I, timeout, a.num("--cap", LIMITS.cap)?, &LIMITS)
            } else {
                return Err(Fail::Usage(format!("argument --method: invalid choice: {method:?}")));
            };
            let mut o = update(d, contracts::document(&I));
            o.insert("kind".into(), json!("enumeration"));
            o.insert("run".into(), r);
            o.insert("panels".into(), json!(C.iter().map(|T| I.labels(T)).collect::<Vec<_>>()));
            contracts::write(&Value::Object(o), out)?;
        }
        "count" => {
            let (_, I) = file(a)?;
            let c = quotient::count(&I, &Clock::limit(a.num("--timeout", LIMITS.timeout_s)?));
            contracts::write(&match c {
                Ok(c) => json!({"status": "complete", "panels": bench::count_json(c.panels), "orbits": c.orbits,
                                "reduced_panels": c.reduced, "classes": quotient::classes(&I).len()}),
                Err(s) => json!({"status": s.name()}),
            }, out)?;
        }
        "verify" => {
            let (d, I) = file(a)?;
            let panels = d.get("panels").and_then(Value::as_array).ok_or("expected an enumeration document".to_string())?;
            let L: Vec<Vec<String>> = panels.iter().map(|T| T.as_array().map(|T| T.iter().filter_map(|v| v.as_str().map(String::from)).collect()))
                .collect::<Option<_>>().ok_or("panels must be lists of labels".to_string())?;
            let C: Vec<Option<Vec<usize>>> = L.iter().map(|T| I.index(T)).collect();
            let all = C.iter().all(|T| T.as_ref().is_some_and(|T| certify::minimal(&I, T)));
            let distinct = L.iter().map(|T| { let mut t = T.clone(); t.sort(); t }).collect::<std::collections::HashSet<_>>().len() == L.len();
            let r = &d["run"];
            let mut o = json!({"panels": L.len(), "all_feasible_and_minimal": all, "distinct_panels": distinct, "run": r["status"]});
            let mut ok = all && distinct;
            if ok && r["status"] == "complete" && r["count"].as_u64() == Some(L.len() as u64) {
                let C: Vec<Vec<usize>> = C.into_iter().map(Option::unwrap).collect();
                let e = match certify::complete(&I, &C) {
                    Completeness::Complete => "complete",
                    Completeness::Missing(_) => "missing",
                    Completeness::Invalid => "invalid",
                };
                o["exhaustion"] = json!(e);
                ok = e == "complete";
            }
            contracts::write(&o, out)?;
            if !ok {
                return Err(Fail::Silent);
            }
        }
        "bench" => {
            let seed: u64 = a.get("--seed").ok_or("the following arguments are required: --seed".to_string())?
                .parse().map_err(|_| "argument --seed: invalid int value".to_string())?;
            let path = out.ok_or("bench needs -o FILE; the saved campaigns in results/ are never overwritten".to_string())?;
            if path.exists() {
                return Err(Fail::Usage(format!("{} exists; a campaign is written once", path.display())));
            }
            let L = bench::Limits { timeout_s: a.num("--timeout", LIMITS.timeout_s)?, ..LIMITS };
            let (_, s) = snapshot()?;
            let res = bench::suite(seed, &L, &s, provenance(&L, seed)?, !a.flags.iter().any(|f| f == "--no-variants"),
                                   &mut |line| println!("{line}"));
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(path, json::dumps(&res, Style::Indent(1))).map_err(|e| e.to_string())?;
        }
        "report" => {
            let f = PathBuf::from(a.pos.first().ok_or("the following arguments are required: FILE".to_string())?);
            let R = json::loads(&std::fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?)?;
            let (rows, s) = snapshot()?;
            let md = report::markdown(&R, &f.file_name().unwrap().to_string_lossy(), verdicts::score(&R, &audit::audit(&rows, &s)));
            // results/suite-<name>.json -> docs/findings/results-<name>.md
            let dest = out.map(PathBuf::from).unwrap_or_else(|| {
                let root = f.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_default();
                let stem = f.file_stem().unwrap().to_string_lossy().replacen("suite-", "results-", 1);
                root.join(format!("docs/findings/{stem}.md"))
            });
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(&dest, md).map_err(|e| e.to_string())?;
            println!("{}", dest.display());
        }
        "site" => site(a)?,
        c => return Err(Fail::Usage(format!("argument COMMAND: invalid choice: {c:?}"))),
    }
    Ok(())
}

fn build(a: &Args, out: Option<&Path>) -> Result<(), Fail> {
    let mut P: Vec<String> = a.pos.clone();
    if let Some(b) = a.get("--basket") {
        if !P.is_empty() {
            return Err(Fail::Usage("use either product arguments or --basket".into()));
        }
        P = contracts::basket_csv(Path::new(b))?;
    }
    if P.is_empty() {
        return Err(Fail::Usage("provide products or --basket".into()));
    }
    let variant = a.get("--variant").unwrap_or("strict");
    if !["strict", "merged"].contains(&variant) {
        return Err(Fail::Usage(format!("argument --variant: invalid choice: {variant:?}")));
    }
    let k: i64 = a.num("--k", 0)?;
    let (_, s) = snapshot()?;
    let M = audit::matrix(&s, variant);
    let missing: Vec<&String> = P.iter().filter(|p| !M.contains_key(*p)).collect();
    if !missing.is_empty() {
        return Err(Fail::Check(format!("not a {variant} product: {}", report::repr(&json!(missing)))));
    }
    let E: Vec<Vec<String>> = P.iter().map(|p| M[p].1.iter().cloned().collect()).collect();
    let V: std::collections::BTreeSet<String> = E.iter().flatten().cloned().collect();
    let F: Vec<Vec<String>> = a.all("--fail").iter().map(|f| f.split(',').map(String::from).collect()).collect();
    let I = Instance::new(V.into_iter().collect(), P, E, k, F)?;
    let mut d = contracts::document(&I);
    d.insert("variant".into(), json!(variant));
    d.extend(contracts::provenance()?);
    let names: Map<String, Value> = I.V.iter().map(|v| (v.clone(), json!(s.S[v].name))).collect();
    d.insert("names".into(), Value::Object(names));
    Ok(contracts::write(&Value::Object(d), out)?)
}

fn git(root: &Path, args: &[&str]) -> String {
    Command::new("git").args(args).current_dir(root).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn cpu() -> String {
    std::env::var("PROCESSOR_IDENTIFIER").ok().or_else(|| {
        std::fs::read_to_string("/proc/cpuinfo").ok()?.lines()
            .find(|l| l.starts_with("model name"))?.split(':').nth(1).map(|s| s.trim().to_string())
    }).unwrap_or_else(|| "not measured".into())
}

/// Everything a result needs to be reproduced and dated.
fn provenance(L: &bench::Limits, seed: u64) -> Result<Value, String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let sha = data::sha256(&data::bytes("raw/QBL.xlsx")?);
    let frozen = json::loads(&data::bytes("manifest.json")?)?;
    Ok(json!({"data_sha256": sha, "data_matches_manifest": frozen["QBL.xlsx"]["sha256"] == sha.as_str(),
              "code_rev": git(&root, &["rev-parse", "HEAD"]),
              "dirty": !git(&root, &["status", "--porcelain", "--untracked-files=no"]).is_empty(),
              "thesis_rev": THESIS_REV, "seed": seed, "limits": L.json(),
              "versions": {"srl": VERSION}, "rustc": env!("SRL_RUSTC"),
              "os": format!("{} {}", std::env::consts::OS, std::env::consts::ARCH), "cpu": cpu(),
              "cpus": std::thread::available_parallelism().map_or(0, |n| n.get()), "memory": "not measured"}))
}

/// SHA-256 over the crate sources (path, then LF-normalised bytes) in path order.
pub fn code_sha256(root: &Path) -> Result<String, String> {
    let mut files = vec![PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock"), PathBuf::from("build.rs")];
    let mut stack = vec![PathBuf::from("src")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(root.join(&d)).map_err(|e| e.to_string())? {
            let p = d.join(e.map_err(|e| e.to_string())?.file_name());
            if root.join(&p).is_dir() { stack.push(p) } else if p.extension().is_some_and(|x| x == "rs") { files.push(p) }
        }
    }
    let name = |p: &PathBuf| p.to_string_lossy().replace('\\', "/");
    files.sort_by_key(name);
    let mut buf = Vec::new();
    for p in &files {
        buf.extend(name(p).as_bytes());
        let b = std::fs::read(root.join(p)).map_err(|e| format!("{}: {e}", p.display()))?;
        buf.extend(String::from_utf8_lossy(&b).replace("\r\n", "\n").as_bytes());
    }
    Ok(data::sha256(&buf))
}

/// Writes the public case data only after their certificates pass.
fn site(a: &Args) -> Result<(), Fail> {
    let root = PathBuf::from(a.get("--root").unwrap_or("."));
    let results = root.join(a.get("--results").unwrap_or(CAMPAIGN));
    let (_, s) = snapshot()?;
    let M = audit::matrix(&s, "strict");
    let names = portfolio::names(&s);
    let manifest = json::loads(&data::bytes("manifest.json")?)?;
    let mut prov = contracts::provenance()?;
    prov.insert("code_rev".into(), json!(git(&root, &["rev-parse", "HEAD"])));
    prov.insert("code_sha256".into(), json!(code_sha256(&root)?));
    prov.insert("dirty".into(), json!(!git(&root, &["status", "--porcelain", "--", "src", "Cargo.toml", "Cargo.lock", "build.rs"]).is_empty()));
    prov.insert("identity_candidates".into(), json!(portfolio::identity_candidates(&s)));
    prov.insert("snapshot".into(), manifest["QBL.xlsx"].clone());
    if prov["data_sha256"] != prov["snapshot"]["sha256"] {
        return Err(Fail::Check("public case data does not match the frozen manifest".into()));
    }
    let R = json::loads(&std::fs::read(&results).map_err(|e| format!("{}: {e}", results.display()))?)?;
    let payload = json!({"schema_version": "1.0", "kind": "portfolio", "provenance": prov,
                         "cases": portfolio::cases(&M, &names).map_err(Fail::Check)?,
                         "evidence": portfolio::evidence(&R, &M, &names).map_err(Fail::Check)?});
    let dest = root.join("site");
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let text = json::dumps(&payload, Style::Compact);
    std::fs::write(dest.join("data.js"), format!("window.DATA={text};\n")).map_err(|e| e.to_string())?;
    std::fs::write(dest.join("cases.json"), format!("{text}\n")).map_err(|e| e.to_string())?;
    std::fs::write(dest.join("document-1.0.json"), SCHEMA).map_err(|e| e.to_string())?;
    for c in payload["cases"].as_array().unwrap() {
        let m: Vec<String> = c["modes"].as_object().unwrap().iter()
            .map(|(k, m)| format!("k={k}: {} minimum {} count {}", m["status"].as_str().unwrap(), m["minimum_size"], m["count"])).collect();
        println!("{} {}", c["id"].as_str().unwrap(), m.join("; "));
    }
    Ok(())
}

/// Today's UTC date, ISO.
fn today() -> String {
    now()[..10].to_string()
}

/// The current UTC time, ISO with seconds.
fn now() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let (z, s) = (t.div_euclid(86400) + 719468, t.rem_euclid(86400));
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

/// Fetch every manifest source with curl into data/fetched; report whether each matches the
/// frozen snapshot. Never writes data/raw.
fn acquire() -> Result<Value, String> {
    let frozen = json::loads(&data::bytes("manifest.json")?)?;
    let dir = PathBuf::from("data/fetched");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut got = Map::new();
    for (name, m) in frozen.as_object().unwrap() {
        let url = m["url"].as_str().ok_or("manifest entry without url")?;
        let path = dir.join(url.rsplit('/').next().unwrap());
        let headers = dir.join(format!("{name}.headers"));
        let st = Command::new("curl").args(["-sSL", "--max-time", "60", "-A", "Mozilla/5.0 (srl acquire)", "-D"])
            .arg(&headers).arg("-o").arg(&path).arg(url).status().map_err(|e| format!("curl: {e}"))?;
        if !st.success() {
            return Err(format!("curl failed for {url}"));
        }
        let body = std::fs::read(&path).map_err(|e| e.to_string())?;
        let lm = std::fs::read_to_string(&headers).unwrap_or_default().lines().rev()
            .find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case("last-modified")).map(|(_, v)| v.trim().to_string()));
        let sha = data::sha256(&body);
        got.insert(name.clone(), json!({"url": url, "retrieved_utc": now(), "sha256": sha, "bytes": body.len(),
                                        "last_modified": lm, "matches_frozen": m["sha256"] == sha.as_str()}));
    }
    let got = Value::Object(got);
    std::fs::write(dir.join("manifest.json"), json::dumps(&got, Style::Indent(1))).map_err(|e| e.to_string())?;
    Ok(got)
}
