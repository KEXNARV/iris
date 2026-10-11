//! HTTP, through curl (it comes with Linux and with Windows 10 onward), the same way updates
//! already download: no TLS stack inside Iris.

use std::io::Write;
use std::process::{Command, Stdio};

/// The store's address; `IRIS_STORE_URL` points somewhere else (a local server, for testing).
pub fn base() -> String {
    crate::paths::var("STORE_URL").unwrap_or_else(|| "https://iris.knarvaez.com".into()).trim_end_matches('/').to_string()
}

/// A request to the store's API: (status, body). `body` is (content type, bytes).
pub fn call(method: &str, path: &str, token: Option<&str>, body: Option<(&str, &[u8])>) -> Result<(u16, Vec<u8>), String> {
    let out = std::env::temp_dir().join(format!("iris-net-{}-{}", std::process::id(), rand_tag()));
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--max-time", "60", "-X", method, "-o"]).arg(&out).args(["-w", "%{http_code}"]);
    if let Some(t) = token {
        cmd.args(["-H", &format!("Authorization: Bearer {t}")]);
    }
    if let Some((ct, _)) = body {
        cmd.args(["-H", &format!("Content-Type: {ct}"), "--data-binary", "@-"]);
    }
    cmd.arg(format!("{}{path}", base()));
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("no pude usar curl: {e}"))?;
    if let (Some((_, bytes)), Some(mut stdin)) = (body, child.stdin.take()) {
        stdin.write_all(bytes).map_err(|e| e.to_string())?;
    }
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let data = std::fs::read(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    if !o.status.success() {
        return Err(format!("sin conexión con la tienda: {}", String::from_utf8_lossy(&o.stderr).trim()));
    }
    let code = String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(0);
    Ok((code, data))
}

/// A JSON call: the answer parsed, or the server's error in plain words.
pub fn json(method: &str, path: &str, token: Option<&str>, body: Option<&serde_json::Value>) -> Result<(u16, serde_json::Value), String> {
    let text = body.map(|b| b.to_string());
    let (code, data) = call(method, path, token, text.as_deref().map(|t| ("application/json", t.as_bytes())))?;
    let v: serde_json::Value = serde_json::from_slice(&data).unwrap_or(serde_json::Value::Null);
    if code >= 400 {
        return Err(error_of(code, &v));
    }
    Ok((code, v))
}

/// What the server said went wrong.
pub fn error_of(code: u16, v: &serde_json::Value) -> String {
    match v.get("error").and_then(|e| e.as_str()) {
        Some(e) => e.to_string(),
        None if code == 401 => "tu sesión no es válida: iris login".into(),
        None => format!("la tienda respondió {code}"),
    }
}

fn rand_tag() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64)
}
