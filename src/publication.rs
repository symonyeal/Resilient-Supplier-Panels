//! Immutable analytical generations with one writer and recoverable fixed-path projection.
//! One file shared byte for byte by Chokepoint-Lab and Resilient-Supplier-Panels; `tests/publication_parity.rs` pins its hash in both.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, String>;
fn read(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!(
        "{}-{}.candidate",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
fn local(name: &str) -> Result<()> {
    if Path::new(name).components().count() != 1
        || !matches!(
            Path::new(name).components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err("Invalid generation entry".into());
    }
    Ok(())
}

pub struct Generation {
    out: PathBuf,
    store: PathBuf,
    stage: PathBuf,
    id: String,
    _lock: File,
}
impl Generation {
    pub fn new(out: &Path, kind: &str) -> Result<Self> {
        let parent = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let name = out
            .file_name()
            .ok_or("Output directory needs a name")?
            .to_str()
            .ok_or("Output name must be UTF-8")?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join(format!(".{name}.writer.lock")))
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("Output writer already active: {e}"))?;
        let store = parent.join(".releases").join(name).join(kind);
        fs::create_dir_all(&store).map_err(|e| e.to_string())?;
        let id = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let stage = store.join(&id).join("root");
        fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
        let generation = Self {
            out: out.to_path_buf(),
            store,
            stage,
            id,
            _lock: lock,
        };
        generation.recover()?;
        Ok(generation)
    }
    pub fn root(&self) -> &Path {
        &self.stage
    }
    pub fn require_new(&self) -> Result<()> {
        if self.out.exists() {
            Err("Run output already exists; choose a new directory".into())
        } else {
            Ok(())
        }
    }
    fn recover(&self) -> Result<()> {
        let path = self.store.join("pending.json");
        if !path.exists() {
            return Ok(());
        }
        let journal = read(&path)?;
        let id = journal["generation"].as_str().ok_or("Missing generation")?;
        local(id)?;
        let accepted = self.store.join("accepted.json");
        if !accepted.exists() || read(&accepted)?["generation"] != id {
            for row in journal["files"]
                .as_array()
                .ok_or("Missing projection inventory")?
            {
                let name = row["path"].as_str().ok_or("Missing projection path")?;
                local(name)?;
                if row["existed"] == true {
                    write(
                        &self.out.join(name),
                        &fs::read(self.store.join(id).join("rollback").join(name))
                            .map_err(|e| e.to_string())?,
                    )?;
                } else if self.out.join(name).exists() {
                    fs::remove_file(self.out.join(name)).map_err(|e| e.to_string())?;
                }
            }
        }
        fs::remove_file(path).map_err(|e| e.to_string())
    }
    pub fn accept_new(self) -> Result<()> {
        self.require_new()?;
        fs::rename(&self.stage, &self.out).map_err(|e| e.to_string())
    }
    pub fn project(self) -> Result<()> {
        fs::create_dir_all(&self.out).map_err(|e| e.to_string())?;
        let mut names = fs::read_dir(&self.stage)
            .map_err(|e| e.to_string())?
            .map(|x| {
                x.map(|e| e.file_name().to_string_lossy().into_owned())
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        names.sort();
        let rollback = self.stage.parent().unwrap().join("rollback");
        fs::create_dir_all(&rollback).map_err(|e| e.to_string())?;
        let mut inventory = Vec::new();
        let mut hashes = serde_json::Map::new();
        for name in &names {
            local(name)?;
            let existed = self.out.join(name).exists();
            if existed {
                fs::copy(self.out.join(name), rollback.join(name)).map_err(|e| e.to_string())?;
            }
            inventory.push(json!({"path":name,"existed":existed}));
            hashes.insert(
                name.clone(),
                json!(format!(
                    "{:x}",
                    Sha256::digest(fs::read(self.stage.join(name)).map_err(|e| e.to_string())?)
                )),
            );
        }
        write(
            &self.store.join("pending.json"),
            &serde_json::to_vec(&json!({"generation":self.id,"files":inventory})).unwrap(),
        )?;
        let result = (|| {
            for name in &names {
                write(
                    &self.out.join(name),
                    &fs::read(self.stage.join(name)).map_err(|e| e.to_string())?,
                )?;
            }
            write(
                &self.store.join("accepted.json"),
                &serde_json::to_vec(
                    &json!({"generation":self.id,"root":self.stage,"sha256":hashes}),
                )
                .unwrap(),
            )
        })();
        match result {
            Ok(()) => {
                self.recover()?;
                fs::remove_dir_all(self.stage.parent().unwrap()).map_err(|e| e.to_string())
            }
            Err(e) => {
                self.recover()?;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let base = std::env::var_os("PUBLICATION_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("target/publication-tests")
            });
        let root = base.join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
    #[test]
    fn one_writer_and_write_once_runs() {
        let out = fixture().join("run");
        let generation = Generation::new(&out, "run").unwrap();
        assert!(Generation::new(&out, "run").is_err());
        fs::write(generation.root().join("result.json"), b"accepted").unwrap();
        generation.accept_new().unwrap();
        let next = Generation::new(&out, "run").unwrap();
        assert!(next.require_new().is_err());
        assert_eq!(fs::read(out.join("result.json")).unwrap(), b"accepted");
    }
    #[test]
    fn an_unpublished_failed_producer_keeps_the_previous_generation() {
        let out = fixture().join("site");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("data.js"), b"old").unwrap();
        {
            let generation = Generation::new(&out, "site").unwrap();
            fs::write(generation.root().join("data.js"), b"partial").unwrap();
        }
        assert_eq!(fs::read(out.join("data.js")).unwrap(), b"old");
    }
    #[test]
    fn next_writer_recovers_an_interrupted_projection() {
        let out = fixture().join("site");
        fs::create_dir_all(&out).unwrap();
        let generation = Generation::new(&out, "site").unwrap();
        let rollback = generation.stage.parent().unwrap().join("rollback");
        fs::create_dir_all(&rollback).unwrap();
        fs::write(rollback.join("data.js"), b"old").unwrap();
        fs::write(out.join("data.js"), b"partial").unwrap();
        fs::write(
            generation.store.join("pending.json"),
            serde_json::to_vec(
                &json!({"generation":generation.id,"files":[{"path":"data.js","existed":true}]}),
            )
            .unwrap(),
        )
        .unwrap();
        drop(generation);
        let recovered = Generation::new(&out, "site").unwrap();
        assert_eq!(fs::read(out.join("data.js")).unwrap(), b"old");
        assert!(!recovered.store.join("pending.json").exists());
    }
}
