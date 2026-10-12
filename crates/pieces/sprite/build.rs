//! Picks the character: `IRIS_CHARACTER=<id>` compiles `src/chars/<id>.rs` as `chars::current`.

fn main() {
    println!("cargo:rerun-if-env-changed=IRIS_CHARACTER");
    let id = std::env::var("IRIS_CHARACTER").unwrap_or_else(|_| "pikachu".into());
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = format!("{dir}/src/chars/{id}.rs");
    assert!(std::path::Path::new(&path).exists(), "no existe {path}");
    println!("cargo:rerun-if-changed={path}");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("current.rs"), format!("#[path = {path:?}]\npub mod current;\n")).unwrap();
}
