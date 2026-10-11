//! FILES: where a recipe finds what its `use` lines name. On disk, a recipe's folder and the
//! store's installed packages; in the browser, the files Iris Studio hands over.

use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) trait Files: Send + Sync {
    /// A file next to the recipe (`file:` and `shape:` uses).
    fn get(&self, path: &str) -> Result<Vec<u8>, String>;

    /// The newest installed version of a store piece with that major: its format (`wasm` or
    /// `shape`) and its code.
    fn store(&self, name: &str, major: u64) -> Result<(String, Vec<u8>), String>;
}

/// On disk: next to the recipe, and the store's packages in `~/.local/share/iris/store`.
pub(crate) struct Disk {
    pub dir: PathBuf,
}

impl Files for Disk {
    fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        std::fs::read(self.dir.join(path)).map_err(|e| format!("«{path}»: {e}"))
    }

    fn store(&self, name: &str, major: u64) -> Result<(String, Vec<u8>), String> {
        let root = crate::paths::data().ok_or("no hay carpeta de datos")?.join("store").join(name);
        let mut best: Option<(Vec<u64>, PathBuf)> = None;
        for e in std::fs::read_dir(&root).map_err(|_| format!("«{name}» no está instalado: /store {name}"))?.flatten() {
            let v: Vec<u64> = e.file_name().to_string_lossy().split('.').filter_map(|p| p.parse().ok()).collect();
            if v.len() == 3 && v[0] == major && best.as_ref().is_none_or(|b| v > b.0) && !e.path().join(".off").exists() {
                best = Some((v, e.path()));
            }
        }
        let (_, dir) = best.ok_or(format!("no hay ninguna versión {major} de «{name}» instalada"))?;
        for (format, file) in [("wasm", "piece.wasm"), ("shape", "shape.toml")] {
            if let Ok(b) = std::fs::read(dir.join(file)) {
                return Ok((format.to_string(), b));
            }
        }
        Err(format!("«{name}» está instalado a medias"))
    }
}

/// In memory, by name; store pieces under `store/<name>@<major>/piece.wasm` or `…/shape.toml`.
#[derive(Default, Clone)]
pub(crate) struct Memory {
    pub files: HashMap<String, Vec<u8>>,
}

impl Files for Memory {
    fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        self.files.get(path).cloned().ok_or(format!("no tengo «{path}»"))
    }

    fn store(&self, name: &str, major: u64) -> Result<(String, Vec<u8>), String> {
        for (format, file) in [("wasm", "piece.wasm"), ("shape", "shape.toml")] {
            if let Some(b) = self.files.get(&format!("store/{name}@{major}/{file}")) {
                return Ok((format.to_string(), b.clone()));
            }
        }
        Err(format!("no tengo «{name}@{major}»"))
    }
}
