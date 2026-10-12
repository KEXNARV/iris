//! The scene's theme, from `IRIS_SCENE`: its ten colors (same roles as Dream Land's: glow,
//! white, accent, cloud, orange, peach, gray, door, drops, dark), the weather of its sky and
//! the shape of its orbit.

fn main() {
    println!("cargo:rerun-if-env-changed=IRIS_SCENE");
    let scene = std::env::var("IRIS_SCENE").unwrap_or_else(|_| "dreamland".into());
    // (colors, ambient, orbit). Ambient: 0 twinkling stars, 1 snow, 2 bubbles, 3 petals,
    // 4 embers, 5 rain, 6 leaves, 7 falling code, 8 fireflies, 9 sparkles, 10 sand.
    // Orbit: 0 stars, 1 hearts, 2 diamonds, 3 rings.
    let (c, ambient, orbit): ([&str; 10], u8, u8) = match scene.as_str() {
        "dreamland" => (["#ffe14a", "#ffffff", "#ffa3cf", "#d9d4ff", "#ff9a3a", "#ffd9b0", "#8a8fa8", "#3a2a6a", "#6a7dff", "#1a1030"], 0, 0),
        "meadow" => (["#ffe66d", "#ffffff", "#ff9ac1", "#f2fff0", "#ffb347", "#ffe0b5", "#9aa89a", "#5a3a1a", "#7ec8ff", "#16260f"], 3, 0),
        "ocean" => (["#9fe8ff", "#ffffff", "#ff9e80", "#d8f6ff", "#ffb36b", "#ffe2c4", "#7a98a8", "#123a5a", "#4fb4ff", "#081a2a"], 2, 3),
        "snow" => (["#e0f4ff", "#ffffff", "#a6c8ff", "#eef6ff", "#ffc48a", "#ffe8d6", "#9aa8b8", "#2a3a5a", "#8ec5ff", "#0e1626"], 1, 0),
        "volcano" => (["#ffb347", "#fff2d0", "#ff5a3a", "#ffd8c8", "#ff7a1a", "#ffd0a0", "#8a6a5a", "#3a1408", "#ffcf4a", "#1a0804"], 4, 2),
        "space" => (["#c9b6ff", "#ffffff", "#7ef9ff", "#e4dcff", "#ffb36b", "#ffe2c4", "#8a8fa8", "#241a4a", "#7aa2ff", "#0a0820"], 0, 3),
        "forest" => (["#b8f27a", "#fffbe8", "#ffcf6a", "#eaffd8", "#e8822a", "#ffe0b5", "#8a9a7a", "#3a2a12", "#7ec8ff", "#0e1a08"], 6, 2),
        "city" => (["#ff4fd8", "#ffffff", "#3df0ff", "#e8dcff", "#ff9a3a", "#ffd9b0", "#7a7a9a", "#1a1a3a", "#5ab0ff", "#08081a"], 5, 2),
        "code" => (["#4dff88", "#eaffea", "#b0ffcb", "#d8ffe4", "#c8ff4a", "#e8ffd0", "#5a8a6a", "#0a2a14", "#4dd8ff", "#020a04"], 7, 3),
        "candy" => (["#ffc2e2", "#ffffff", "#9ef0e0", "#fff0f8", "#ffb0a0", "#ffe6d6", "#b8a0b8", "#7a3a5a", "#a0c8ff", "#2a1020"], 9, 1),
        "desert" => (["#ffd27a", "#fffaf0", "#ff9e5a", "#fff2dc", "#e8822a", "#ffe0b5", "#a89878", "#5a3a1a", "#7ec8ff", "#1e1408"], 10, 2),
        "cave" => (["#b28dff", "#ffffff", "#7dfcff", "#e0d8ff", "#ffb36b", "#ffe2c4", "#7a7898", "#1e1838", "#6aa0ff", "#08061a"], 8, 2),
        "spooky" => (["#b6ff6a", "#f4fff0", "#ff8c42", "#e8f0e0", "#ff6a1a", "#ffd0a0", "#7a8a7a", "#1e1a2a", "#9a7aff", "#0a080e"], 8, 3),
        "sakura" => (["#ffd1e8", "#ffffff", "#ff7fb0", "#fff0f6", "#ffb08a", "#ffe6d6", "#b8a0b0", "#5a2a3a", "#a8c8ff", "#1e0e16"], 3, 1),
        other => panic!("IRIS_SCENE: no conozco el escenario «{other}»"),
    };
    let out = format!("pub const COLORS: &[&str] = &{c:?};\npub const AMBIENT: u8 = {ambient};\npub const ORBIT: u8 = {orbit};\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("scene.rs"), out).unwrap();
}
