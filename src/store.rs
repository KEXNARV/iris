//! STORE: the signed catalog, and what's installed from it (see `docs/store-spec.md`).
//!
//! Iris only trusts an index whose Ed25519 signature checks out against the key inside this
//! binary, and only installs a package whose sha256 is in it. Installed packages are unzipped
//! in `~/.local/share/iris/store/<name>/<version>/`; a version Iris turned off (revoked, or
//! failing three times) carries an `.off` file with the reason.

use std::io::Read;
use std::path::PathBuf;

use base64::Engine;
use sha2::{Digest, Sha256};

/// The public half of the store's signing key (the private half never leaves Kevin's machine).
const PUBLIC_KEY: [u8; 32] = [
    0x39, 0x37, 0x88, 0xe9, 0xab, 0x8a, 0xa2, 0x83, 0x43, 0xca, 0x8f, 0x30, 0x7f, 0x90, 0xa0, 0x24, 0x5c, 0x87, 0x4c, 0x5d, 0x3f, 0xca, 0x39, 0x55, 0x85, 0x8b,
    0x91, 0x8a, 0x5e, 0x51, 0x69, 0xb3,
];

/// One version of a package in the index.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    /// `buddy` or `piece`.
    pub kind: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub sha256: String,
    pub deps: Vec<String>,
}

impl Entry {
    pub fn major(&self) -> u64 {
        self.version.split('.').next().and_then(|m| m.parse().ok()).unwrap_or(0)
    }

    fn key(&self) -> Vec<u64> {
        self.version.split('.').filter_map(|p| p.parse().ok()).collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Index {
    pub serial: u64,
    pub packages: Vec<Entry>,
    pub revoked: Vec<String>,
}

impl Index {
    /// The newest version of each package, by name.
    pub fn latest(&self) -> Vec<&Entry> {
        let mut out: Vec<&Entry> = vec![];
        for e in &self.packages {
            if self.revoked.contains(&e.sha256) {
                continue;
            }
            match out.iter_mut().find(|o| o.name == e.name) {
                Some(o) if e.key() > o.key() => *o = e,
                Some(_) => {}
                None => out.push(e),
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// The newest version of `name`, with that major if given.
    pub fn find(&self, name: &str, major: Option<u64>) -> Option<&Entry> {
        self.packages
            .iter()
            .filter(|e| e.name == name && major.is_none_or(|m| e.major() == m) && !self.revoked.contains(&e.sha256))
            .max_by_key(|e| e.key())
    }
}

pub fn dir() -> Option<PathBuf> {
    Some(crate::paths::data()?.join("store"))
}

fn token_path() -> Option<PathBuf> {
    Some(crate::paths::config()?.join("store-token"))
}

/// The store session, if the user logged in (`iris login`).
pub fn token() -> Option<String> {
    let t = std::fs::read_to_string(token_path()?).ok()?;
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_string())
}

pub fn save_token(t: &str) -> std::io::Result<()> {
    let p = token_path().ok_or_else(|| std::io::Error::other("sin carpeta de configuración"))?;
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&p, format!("{t}\n"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn forget_token() {
    if let Some(p) = token_path() {
        let _ = std::fs::remove_file(p);
    }
}

pub fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// Checks an index's signature and reads it.
pub fn verify(body: &[u8], sig_b64: &str) -> Result<Index, String> {
    let sig = base64::engine::general_purpose::STANDARD.decode(sig_b64.trim()).map_err(|_| "la firma del índice no se lee")?;
    let sig: [u8; 64] = sig.try_into().map_err(|_| "la firma del índice tiene otro tamaño")?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(&PUBLIC_KEY).map_err(|e| e.to_string())?;
    key.verify_strict(body, &ed25519_dalek::Signature::from_bytes(&sig)).map_err(|_| "la firma del índice no es válida: no lo uso")?;
    parse_index(body)
}

fn parse_index(body: &[u8]) -> Result<Index, String> {
    let v: serde_json::Value = serde_json::from_slice(body).map_err(|e| format!("el índice no se lee: {e}"))?;
    let s = |e: &serde_json::Value, k: &str| e.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let packages = v
        .get("packages")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .map(|e| Entry {
                    name: s(e, "name"),
                    kind: s(e, "kind"),
                    version: s(e, "version"),
                    description: s(e, "description"),
                    author: s(e, "author"),
                    sha256: s(e, "sha256"),
                    deps: e.get("deps").and_then(|d| d.as_array()).map(|d| d.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    let revoked = v.get("revoked").and_then(|r| r.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    Ok(Index { serial: v.get("serial").and_then(|x| x.as_u64()).unwrap_or(0), packages, revoked })
}

/// The last good index saved, without going online.
pub fn cached() -> Option<Index> {
    let d = dir()?;
    let body = std::fs::read(d.join("index.json")).ok()?;
    let sig = std::fs::read_to_string(d.join("index.json.sig")).ok()?;
    verify(&body, &sig).ok()
}

/// Brings the index from the store, checks it and keeps it; turns off what it revokes. Returns
/// it and what got turned off.
pub fn refresh() -> Result<(Index, Vec<String>), String> {
    let (code, body) = crate::net::call("GET", "/api/index.json", None, None)?;
    if code == 404 {
        return Ok((Index::default(), vec![]));
    }
    if code != 200 {
        return Err(format!("la tienda respondió {code}"));
    }
    let (code, sig) = crate::net::call("GET", "/api/index.json.sig", None, None)?;
    if code != 200 {
        return Err("el índice llegó sin firma: no lo uso".into());
    }
    let index = verify(&body, &String::from_utf8_lossy(&sig))?;
    if let Some(old) = cached() {
        if index.serial < old.serial {
            return Err("la tienda mandó un índice más viejo que el que tengo: no lo uso".into());
        }
    }
    let d = dir().ok_or("sin carpeta de datos")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    std::fs::write(d.join("index.json"), &body).map_err(|e| e.to_string())?;
    std::fs::write(d.join("index.json.sig"), &sig).map_err(|e| e.to_string())?;
    let mut off = vec![];
    for (name, version, sha, _) in installed() {
        if index.revoked.contains(&sha) {
            turn_off(&name, &version, "la tienda la retiró");
            off.push(name);
        }
    }
    Ok((index, off))
}

/// What's installed: (name, version, sha256, kind), newest first per name; turned-off versions
/// out.
pub fn installed() -> Vec<(String, String, String, String)> {
    let mut out = vec![];
    let Some(d) = dir() else { return out };
    for e in std::fs::read_dir(&d).into_iter().flatten().flatten().filter(|e| e.path().is_dir()) {
        let name = e.file_name().to_string_lossy().to_string();
        for v in std::fs::read_dir(e.path()).into_iter().flatten().flatten() {
            let p = v.path();
            if p.join(".off").exists() || p.join(".preview").exists() {
                continue;
            }
            let sha = std::fs::read_to_string(p.join(".sha256")).unwrap_or_default().trim().to_string();
            let kind = std::fs::read_to_string(p.join("iris.toml"))
                .ok()
                .and_then(|t| t.parse::<toml::Table>().ok())
                .and_then(|t| Some(t.get("package")?.get("kind")?.as_str()?.to_string()))
                .unwrap_or_default();
            out.push((name.clone(), v.file_name().to_string_lossy().to_string(), sha, kind));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| version_key(&b.1).cmp(&version_key(&a.1))));
    out
}

/// The folder of the newest usable version of `name` (installed, or downloaded to look at it).
pub fn version_dir(name: &str) -> Option<PathBuf> {
    let d = dir()?.join(name);
    std::fs::read_dir(&d)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(".sha256").exists() && !p.join(".off").exists())
        .max_by_key(|p| version_key(&p.file_name().unwrap_or_default().to_string_lossy()))
}

fn version_key(v: &str) -> Vec<u64> {
    v.split('.').filter_map(|p| p.parse().ok()).collect()
}

/// Turns a version off, with the reason (it stays on disk; `/store remove` deletes it).
pub fn turn_off(name: &str, version: &str, why: &str) {
    if let Some(d) = dir() {
        let _ = std::fs::write(d.join(name).join(version).join(".off"), why);
    }
}

/// One more failure of a piece; at the third, it's turned off until a new version comes out.
/// Returns how many it has.
pub fn failed(name: &str) -> u32 {
    let Some(d) = dir() else { return 0 };
    let p = d.join(name).join(".failures");
    let n = std::fs::read_to_string(&p).ok().and_then(|s| s.trim().parse::<u32>().ok()).unwrap_or(0) + 1;
    let _ = std::fs::write(&p, n.to_string());
    if n >= 3 {
        if let Some((_, v, _, _)) = installed().into_iter().find(|i| i.0 == name) {
            turn_off(name, &v, "falló tres veces");
        }
    }
    n
}

/// Downloads, checks and unzips a version (and what it depends on). `preview`: only to look at
/// it, not counted as installed. Returns the names installed.
pub fn install(index: &Index, e: &Entry, preview: bool) -> Result<Vec<String>, String> {
    let mut done = vec![];
    install_one(index, e, preview, &mut done, 0)?;
    Ok(done)
}

fn install_one(index: &Index, e: &Entry, preview: bool, done: &mut Vec<String>, depth: usize) -> Result<(), String> {
    if depth > 8 {
        return Err("las dependencias dan vueltas".into());
    }
    for dep in &e.deps {
        let (name, major) = dep.split_once('@').ok_or(format!("dependencia rara: {dep}"))?;
        let d = index.find(name, major.parse().ok()).ok_or(format!("«{}» necesita «{dep}» y no está en la tienda", e.name))?;
        install_one(index, d, preview, done, depth + 1)?;
    }
    let target = dir().ok_or("sin carpeta de datos")?.join(&e.name).join(&e.version);
    let ready = std::fs::read_to_string(target.join(".sha256")).is_ok_and(|s| s.trim() == e.sha256);
    if !ready {
        let (code, bytes) = crate::net::call("GET", &format!("/api/pieces/{}/{}.iris", e.name, e.version), None, None)?;
        if code != 200 {
            return Err(format!("no pude bajar «{}» (la tienda respondió {code})", e.name));
        }
        if sha256_hex(&bytes) != e.sha256 {
            return Err(format!("«{}» llegó distinto de lo que firmó la tienda: no lo instalo", e.name));
        }
        unzip(&bytes, &target)?;
        std::fs::write(target.join(".sha256"), &e.sha256).map_err(|x| x.to_string())?;
    }
    if preview && !ready {
        let _ = std::fs::write(target.join(".preview"), "");
    } else if !preview {
        let _ = std::fs::remove_file(target.join(".preview"));
        let _ = std::fs::remove_file(target.join(".off"));
        let _ = std::fs::remove_file(target.join("..").join(".failures"));
    }
    done.push(e.name.clone());
    Ok(())
}

/// Unzips a package into `to`, only the files a package may carry.
fn unzip(bytes: &[u8], to: &std::path::Path) -> Result<(), String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("el paquete no se abre: {e}"))?;
    let tmp = to.with_extension("tmp");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.enclosed_name().ok_or("el paquete trae una ruta peligrosa")?;
        let name = name.file_name().ok_or("el paquete trae una ruta rara")?.to_owned();
        let mut data = vec![];
        f.by_ref().take(5 << 20).read_to_end(&mut data).map_err(|e| e.to_string())?;
        std::fs::write(tmp.join(name), data).map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_dir_all(to);
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

/// Deletes every version of `name`.
pub fn remove(name: &str) -> Result<(), String> {
    let d = dir().ok_or("sin carpeta de datos")?.join(name);
    if !d.exists() {
        return Err(format!("«{name}» no está instalado"));
    }
    std::fs::remove_dir_all(d).map_err(|e| e.to_string())
}

/// What has a newer version in the index.
pub fn updates(index: &Index) -> Vec<Entry> {
    let mut out = vec![];
    for (name, version, _, _) in installed() {
        if out.iter().any(|e: &Entry| e.name == name) {
            continue;
        }
        // Within the same major: a buddy asks for `name@major`, and a new major is another piece.
        let major = version.split('.').next().and_then(|m| m.parse().ok());
        if let Some(e) = index.find(&name, major) {
            if version_key(&e.version) > version_key(&version) {
                out.push(e.clone());
            }
        }
    }
    out
}

fn dismissed_path() -> Option<PathBuf> {
    Some(dir()?.join(".dismissed"))
}

/// The updates offered and turned down ("name version" per line): not offered again.
pub fn dismissed() -> Vec<String> {
    dismissed_path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| t.lines().map(str::to_string).collect()).unwrap_or_default()
}

pub fn dismiss(list: &[Entry]) {
    let Some(p) = dismissed_path() else { return };
    let mut all = dismissed();
    all.extend(list.iter().map(|e| format!("{} {}", e.name, e.version)));
    all.sort();
    all.dedup();
    let _ = std::fs::write(p, all.join("\n") + "\n");
}

/// Builds a `.iris` from a folder: `iris.toml` and the files its kind needs.
pub fn pack(dir: &std::path::Path, files: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let manifest = std::fs::read(dir.join("iris.toml")).map_err(|_| "falta iris.toml en la carpeta")?;
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in std::iter::once(("iris.toml", manifest)).chain(files.iter().map(|(n, d)| (*n, d.clone()))) {
        z.start_file(name, opt).map_err(|e| e.to_string())?;
        z.write_all(&data).map_err(|e| e.to_string())?;
    }
    Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_index_signed_with_the_store_key_is_accepted() {
        // Signed with the real key (openssl pkeyutl -sign -rawin).
        let body = br#"{"serial":1,"packages":[],"revoked":[]}"#;
        let sig = "FNKe28RyQD4ICuTBfx3MG8PkhKMRwkB5GDNE/RdEyrYdUQUKGfKCHzN/BmN6+rRnnLGdz6DiVpmr8bMUzCzRAw==";
        assert_eq!(verify(body, sig).unwrap().serial, 1);
        assert!(verify(br#"{"serial":2,"packages":[],"revoked":[]}"#, sig).is_err(), "one byte changed");
    }

    #[test]
    fn a_tampered_index_is_refused() {
        let body = br#"{"serial":1,"packages":[],"revoked":[]}"#;
        let sig = base64::engine::general_purpose::STANDARD.encode([7u8; 64]);
        assert!(verify(body, &sig).unwrap_err().contains("no es válida"));
        assert!(verify(body, "no-es-base64!").is_err());
    }

    #[test]
    fn picks_the_newest_per_major() {
        let e = |v: &str, sha: &str| Entry { name: "kirby".into(), kind: "buddy".into(), version: v.into(), description: String::new(), author: String::new(), sha256: sha.into(), deps: vec![] };
        let idx = Index { serial: 1, packages: vec![e("1.0.0", "a"), e("1.10.0", "b"), e("2.0.0", "c"), e("1.2.0", "d")], revoked: vec!["c".into()] };
        assert_eq!(idx.find("kirby", Some(1)).unwrap().version, "1.10.0");
        assert_eq!(idx.find("kirby", None).unwrap().version, "1.10.0", "the revoked 2.0.0 doesn't count");
        assert_eq!(idx.latest().len(), 1);
    }
}
