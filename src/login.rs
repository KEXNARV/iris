//! `/login`: la cuenta de Claude con la que trabaja el motor. `claude -p` no entiende /login,
//! así que Iris suelta la pantalla, corre `claude auth login` en la misma terminal y vuelve.

use std::process::{Command, Stdio};

/// La sesión de Claude Code en este equipo.
pub enum Cuenta {
    Conectada(Option<String>),
    SinSesion,
}

/// `claude auth status` tarda ~0,1 s: sale con 0 si hay sesión y 1 si no. `None`: no hay
/// `claude` instalado.
pub fn cuenta() -> Option<Cuenta> {
    let out = Command::new("claude").args(["auth", "status", "--json"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    if json["loggedIn"].as_bool().unwrap_or(out.status.success()) {
        Some(Cuenta::Conectada(json["email"].as_str().map(String::from)))
    } else {
        Some(Cuenta::SinSesion)
    }
}

/// Corre el inicio de sesión con la terminal entera: abre el navegador y, si hace falta, pide
/// pegar el código. La pantalla de Iris tiene que estar soltada antes de llamarla.
pub fn iniciar() -> std::io::Result<bool> {
    println!("\n  Iris · iniciar sesión en Claude\n");
    Command::new("claude").args(["auth", "login"]).status().map(|s| s.success())
}
