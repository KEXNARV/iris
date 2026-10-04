//! Guarda con qué commit se compiló: así Jarvis sabe si en GitHub hay algo más nuevo.

use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=JARVIS_COMMIT={commit}");
    // Recompilar al cambiar de commit, aunque no cambie el código.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
