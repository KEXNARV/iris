//! `/login`: the Claude account the engine works with. `claude -p` doesn't understand /login,
//! so Iris releases the screen, runs `claude auth login` in the same terminal and comes back.

use std::process::{Command, Stdio};

/// The Claude Code session on this machine.
pub enum Account {
    Connected(Option<String>),
    NoSession,
}

/// `claude auth status` takes ~0.1 s: exits with 0 if there is a session and 1 if not. `None`:
/// `claude` is not installed.
pub fn account() -> Option<Account> {
    let out = Command::new("claude").args(["auth", "status", "--json"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    if json["loggedIn"].as_bool().unwrap_or(out.status.success()) {
        Some(Account::Connected(json["email"].as_str().map(String::from)))
    } else {
        Some(Account::NoSession)
    }
}

/// Runs the login with the whole terminal: opens the browser and, if needed, asks to paste
/// the code. Iris's screen must be released before calling it.
pub fn log_in() -> std::io::Result<bool> {
    println!("\n  Iris · iniciar sesión en Claude\n");
    Command::new("claude").args(["auth", "login"]).status().map(|s| s.success())
}
