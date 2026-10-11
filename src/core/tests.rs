use super::*;
use super::state::ALL;
use super::recipe::Assembled;
use super::color::{apart, hsv, INK_RED};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// A core's original buddy, to look inside it.
fn blob(c: &Core) -> &Assembled {
    c.buddies[0].as_any().downcast_ref().unwrap()
}


#[test]
fn classifies_tools() {
    assert_eq!(tool_state("Read", "src/ui.rs"), State::Reading);
    assert_eq!(tool_state("Grep", "draw_core"), State::Searching);
    assert_eq!(tool_state("Write", "x"), State::Editing);
    assert_eq!(tool_state("WebSearch", "x"), State::Web);
    assert_eq!(tool_state("Agent", "x"), State::Delegating);
    assert_eq!(tool_state("TodoWrite", ""), State::Planning);
    assert_eq!(tool_state("mcp__claude_ai_Dipro__query", ""), State::Running);
}

#[test]
fn classifies_bash_commands() {
    for c in ["git push", "cd ~/code/jarvis && git status --short", "GIT_PAGER=cat git log", "git"] {
        assert_eq!(tool_state("Bash", c), State::Git, "{c}");
    }
    for c in ["cargo test", "cd x && cargo test core", "RUST_LOG=1 cargo test", "pnpm vitest run", "npx jest", "python -m pytest -q"] {
        assert_eq!(tool_state("Bash", c), State::Testing, "{c}");
    }
    for c in ["cargo build", "ls -la", "echo git", "grep -rn digit src", "legit status"] {
        assert_eq!(tool_state("Bash", c), State::Running, "{c}");
    }
    // A `git` followed by tests is a test run.
    assert_eq!(tool_state("Bash", "git stash && cargo test"), State::Testing);
}

fn run(want: State, secs: f64, sig: &Signals) -> Core {
    let mut c = Core::new();
    let mut t = 0.0;
    while t < secs {
        c.step(0.04, want, sig);
        t += 0.04;
    }
    c
}

fn render(c: &Core, w: u16, h: u16) -> Buffer {
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    c.draw(area, &mut buf);
    buf
}

#[test]
fn dances_to_music_when_idle() {
    let loud = Signals { music: 0.9, ..Default::default() };
    let mut c = run(State::Idle, 4.0, &loud);
    assert!(blob(&c).channel("groove") > 0.99);
    // A strong hit reaches the red on the scale, like the keyboard's bar.
    let (px, _) = c.pixels(200, 200, 4);
    assert!(px.contains(&(1 + INK_RED * 3 + 2)), "sin rojo en la escala");
    let still = run(State::Idle, 4.0, &Signals::default()).pixels(200, 200, 4).0;
    assert!(!still.contains(&(1 + INK_RED * 3 + 2)));
    // A key wakes it from rest and it stops dancing right away.
    for _ in 0..10 {
        c.step(0.04, State::Typing, &loud);
    }
    assert!(blob(&c).channel("groove") < 0.1);
    // No music, no dancing.
    assert_eq!(blob(&run(State::Idle, 4.0, &Signals::default())).channel("groove"), 0.0);
}

#[test]
fn every_state_draws_something_at_several_sizes() {
    let sig = Signals { ctx: 0.5, queue: 2, todos: (5, 2), ..Default::default() };
    for s in ALL {
        let c = run(s, 3.0, &sig);
        assert_eq!(c.state(), s);
        for (w, h) in [(36, 15), (24, 9), (60, 22), (12, 20), (6, 3)] {
            let buf = render(&c, w, h);
            let lit = buf.content().iter().filter(|c| c.symbol() != " ").count();
            assert!(lit > 0, "{s:?} a {w}×{h} no dibujó nada");
        }
    }
}

#[test]
fn states_stand_apart_from_accent() {
    let blue = [69.0, 123.0, 255.0];
    let thinking = [90.0, 150.0, 255.0];
    let (h, _, _) = hsv(apart(thinking, blue));
    let (ha, _, _) = hsv(blue);
    assert!(((h - ha + 540.0) % 360.0 - 180.0).abs() >= 39.9);
    // What's already far away is left alone.
    let green = [120.0, 235.0, 120.0];
    assert_eq!(apart(green, blue), green);
}

#[test]
fn is_deterministic() {
    let _turn = crate::theme::tests::TURN.lock().unwrap_or_else(|e| e.into_inner());
    let sig = Signals::default();
    let shape = |b: Buffer| b.content().iter().map(|c| c.symbol().to_string()).collect::<String>();
    let a = shape(render(&run(State::Thinking, 2.0, &sig), 36, 15));
    let b = shape(render(&run(State::Thinking, 2.0, &sig), 36, 15));
    assert_eq!(a, b);
}

#[test]
fn short_read_does_not_flicker() {
    let sig = Signals::default();
    let mut c = run(State::Thinking, 1.0, &sig);
    c.step(0.04, State::Reading, &sig);
    assert_eq!(c.state(), State::Reading);
    // The Read ended right away, but it stays visible for a moment.
    c.step(0.04, State::Thinking, &sig);
    assert_eq!(c.state(), State::Reading);
    for _ in 0..20 {
        c.step(0.04, State::Thinking, &sig);
    }
    assert_eq!(c.state(), State::Thinking);
    // Urgent states come in without waiting.
    c.step(0.04, State::Reading, &sig);
    c.step(0.04, State::Asking, &sig);
    assert_eq!(c.state(), State::Asking);
}

#[test]
fn events_break_nothing() {
    let sig = Signals::default();
    let mut c = run(State::Idle, 1.0, &sig);
    for ev in [Event::Key, Event::Error, Event::Done, Event::Cancel, Event::Nope, Event::Merge, Event::Copy, Event::Pass] {
        c.fire(ev);
        for _ in 0..10 {
            c.step(0.04, State::Idle, &sig);
            render(&c, 36, 15);
        }
    }
}

#[test]
fn kids_are_born_work_and_return() {
    let mut sig = Signals::default();
    let mut c = run(State::Idle, 1.0, &sig);
    for id in 0..3 {
        c.kid_born(id);
    }
    sig.kids = vec![(0, State::Reading), (1, State::Editing), (2, State::Running)];
    for _ in 0..30 {
        c.step(0.04, State::Delegating, &sig);
    }
    c.kid_pulse(0, false);
    c.kid_pulse(2, true);
    for (w, h) in [(36, 15), (24, 9), (60, 22), (6, 3)] {
        render(&c, w, h);
    }
    c.kid_end(0, true);
    c.kid_end(1, false);
    sig.kids.retain(|k| k.0 == 2);
    let mut bloomed = false;
    for _ in 0..50 {
        let before = blob(&c).rings();
        c.step(0.04, State::Idle, &sig);
        bloomed |= blob(&c).rings() > before;
        render(&c, 36, 15);
    }
    // The one that succeeded came back and merged (with its flash); the one that failed faded out.
    assert_eq!(c.kid_count(), 1);
    assert!(bloomed);
    // Restarting the engine takes them away.
    c.reboot();
    assert_eq!(c.kid_count(), 0);
}

/// Fingerprint of how the core looks: every state, with and without Baymax, with signals,
/// events and kids, in braille and in image dots, over several seconds. Used to split the core
/// into pieces without changing anything visible:
/// `IRIS_DORADO=/path cargo test core::tests::golden -- --ignored` writes it there, and if the
/// file already exists it compares against it.
#[test]
#[ignore]
fn golden() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let Some(path) = crate::paths::var("DORADO") else { return };
    let _turn = crate::theme::tests::TURN.lock().unwrap_or_else(|e| e.into_inner());
    let palette = std::env::temp_dir().join("iris-dorado-baymax.toml");
    let fingerprint = |c: &Core| {
        let mut h = DefaultHasher::new();
        for (w, hh) in [(36, 15), (60, 22), (12, 6)] {
            for cell in render(c, w, hh).content() {
                cell.symbol().hash(&mut h);
            }
        }
        c.pixels(240, 200, 4).0.hash(&mut h);
        c.pixels(120, 90, 3).0.hash(&mut h);
        h.finish()
    };
    let mut out = String::new();
    for baymax in [false, true] {
        crate::theme::use_palette(baymax.then(|| palette.clone()));
        let signals = [
            Signals::default(),
            Signals { level: 0.6, ctx: 0.85, queue: 3, todos: (6, 4), idle: 40.0, music: 0.7, ..Default::default() },
            Signals { calm: true, ctx: 0.3, kids: vec![(1, State::Searching), (2, State::Editing)], ..Default::default() },
        ];
        for (n, sig) in signals.iter().enumerate() {
            for s in ALL {
                let mut c = Core::new();
                if !sig.kids.is_empty() {
                    c.kid_born(1);
                    c.kid_born(2);
                }
                for step in 0..90 {
                    c.step(0.04, s, sig);
                    match step {
                        20 => c.fire(Event::Key),
                        35 => c.fire(Event::Error),
                        50 => c.fire(Event::Done),
                        60 => c.fire(Event::Merge),
                        70 => c.fire(Event::Copy),
                        75 => c.fire(Event::Pass),
                        80 => c.fire(Event::Nope),
                        85 => c.fire(Event::Cancel),
                        _ => {}
                    }
                    if step == 40 && !sig.kids.is_empty() {
                        c.kid_pulse(1, false);
                        c.kid_end(2, true);
                    }
                    if step % 6 == 0 {
                        out.push_str(&format!("{baymax} {n} {s:?} {step} {:016x}\n", fingerprint(&c)));
                    }
                }
            }
        }
    }
    crate::theme::use_palette(None);
    match std::fs::read_to_string(&path) {
        Ok(before) => {
            let differing: Vec<_> = before.lines().zip(out.lines()).filter(|(a, b)| a != b).map(|(a, _)| a.to_string()).collect();
            assert!(before.lines().count() == out.lines().count() && differing.is_empty(), "{} cuadros cambiaron, el primero: {:?}", differing.len(), differing.first());
        }
        Err(_) => std::fs::write(&path, out).unwrap(),
    }
}

/// `IRIS_SNAPSHOT=1 cargo test core::tests::snapshot -- --nocapture` leaves target/nucleo.html with all
/// the states, to look at them in the browser.
#[test]
/// `IRIS_SNAPSHOT=1 cargo test core::tests::baymax_ppm` leaves Baymax's face in several states
/// in target/baymax/, as foot sees it (Sixel), with the real Baymax palette.
fn baymax_ppm() {
    if !crate::paths::has_var("SNAPSHOT") {
        return;
    }
    crate::theme::use_palette(Some(crate::paths::config().unwrap().join("colores-baymax.toml")));
    let dir = std::path::Path::new("target/baymax");
    std::fs::create_dir_all(dir).unwrap();
    let bg = crate::baymax::toward(crate::theme::background_rgb(), 0, 0.0);
    let sig = Signals::default();
    for (s, secs) in [(State::Idle, 3.0), (State::Thinking, 2.6), (State::Speaking, 2.6), (State::Sleeping, 3.0), (State::Searching, 2.6), (State::Offline, 2.6)] {
        let c = run(s, secs, &sig);
        let (w, h) = (480, 400);
        let (img, colors) = c.pixels(w, h, 4);
        let mut out = format!("P6 {w} {h} 255\n").into_bytes();
        for i in img {
            let c = if i == 0 { bg } else { colors[i as usize - 1] };
            out.extend(c.map(|v| v.round() as u8));
        }
        std::fs::write(dir.join(format!("{:?}.ppm", s)), out).unwrap();
    }
}

#[test]
fn snapshot() {
    if !crate::paths::has_var("SNAPSHOT") {
        return;
    }
    crate::theme::poll();
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><body style='background:#05090d;color:#ccc;font:13px monospace;\
         display:flex;flex-wrap:wrap;gap:14px;padding:14px'>",
    );
    let sig = Signals { ctx: 0.35, todos: (6, 3), ..Default::default() };
    for (s, w, h) in ALL.iter().map(|s| (*s, 36u16, 15u16)).chain([(State::Thinking, 24, 9), (State::Idle, 60, 22)]) {
        let c = run(s, 2.6, &sig);
        let buf = render(&c, w, h);
        html.push_str("<div><pre style='line-height:1.12;font-family:\"JetBrainsMono Nerd Font\",monospace;font-size:15px;margin:0'>");
        for y in 0..h {
            for x in 0..w {
                let cell = &buf[(x, y)];
                let Color::Rgb(r, g, b) = cell.fg else {
                    html.push(' ');
                    continue;
                };
                html.push_str(&format!("<span style='color:rgb({r},{g},{b})'>{}</span>", cell.symbol()));
            }
            html.push('\n');
        }
        let col = s.rgb();
        html.push_str(&format!(
            "</pre><div style='text-align:center;color:rgb({},{},{})'>{} {w}×{h}</div></div>",
            col[0], col[1], col[2], s.label()
        ));
    }
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/target/nucleo.html"), html).unwrap();

    // Animation: real core frames at 25 fps, to see what a still image doesn't show.
    let mut frames = Vec::new();
    let mut c = Core::new();
    let script = [(State::Idle, 5.0), (State::Thinking, 4.0), (State::Listening, 3.0), (State::Speaking, 4.0)];
    let sig = Signals { ctx: 0.3, level: 0.03, idle: 30.0, ..Default::default() };
    for (s, secs) in script {
        let mut t = 0.0;
        while t < secs {
            c.step(0.04, s, &sig);
            t += 0.04;
            let buf = render(&c, 36, 15);
            let mut f = String::new();
            for y in 0..15 {
                for x in 0..36 {
                    let cell = &buf[(x, y)];
                    match cell.fg {
                        Color::Rgb(r, g, b) => f.push_str(&format!("<span style=color:rgb({r},{g},{b})>{}</span>", cell.symbol())),
                        _ => f.push(' '),
                    }
                }
                f.push('\n');
            }
            frames.push(format!("[{:?},{:?}]", f, s.label()));
        }
    }
    let anim = format!(
        "<!doctype html><meta charset=utf-8><body style='background:#05090d;color:#ccc;font:14px monospace;padding:20px'>\
         <pre id=f style='font-family:\"JetBrainsMono Nerd Font\",monospace;font-size:22px;line-height:1.12;margin:0'></pre>\
         <div id=l style='font-size:16px;margin-top:8px'></div><script>const F=[{}];let i=0;\
         setInterval(()=>{{const[x,l]=F[i++%F.length];f.innerHTML=x;document.getElementById('l').textContent=l;}},40)</script>",
        frames.join(",")
    );
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/target/nucleo-anim.html"), anim).unwrap();

    // Kids: three are born, work on different things, send particles and leave.
    let mut c = run(State::Idle, 2.6, &Signals::default());
    let mut sig = Signals { ctx: 0.3, ..Default::default() };
    let mut frames = Vec::new();
    let mut t = 0.0;
    let mut next = 0;
    let script: [(f64, &dyn Fn(&mut Core, &mut Signals)); 9] = [
        (0.3, &|c, s| { c.kid_born(0); s.kids.push((0, State::Thinking)); }),
        (1.0, &|c, s| { c.kid_born(1); s.kids.push((1, State::Thinking)); }),
        (1.6, &|c, s| { c.kid_born(2); s.kids.push((2, State::Searching)); }),
        (2.6, &|c, s| { c.kid_pulse(0, false); s.kids[0].1 = State::Reading; }),
        (3.4, &|c, s| { c.kid_pulse(1, false); s.kids[1].1 = State::Editing; }),
        (4.6, &|c, s| { c.kid_pulse(2, false); s.kids[2].1 = State::Running; }),
        (6.5, &|c, _| c.kid_pulse(2, true)),
        (8.0, &|c, s| { c.kid_end(0, true); s.kids.retain(|k| k.0 != 0); }),
        (9.5, &|c, s| { c.kid_end(1, false); s.kids.retain(|k| k.0 != 1); }),
    ];
    while t < 12.0 {
        while next < script.len() && t >= script[next].0 {
            (script[next].1)(&mut c, &mut sig);
            next += 1;
        }
        if t > 6.0 && t < 6.04 {
            // One pulse per second, like a working agent.
            c.kid_pulse(2, false);
        }
        c.step(0.04, State::Idle, &sig);
        t += 0.04;
        let (w, h) = (48u16, 20u16);
        let buf = render(&c, w, h);
        let mut f = String::new();
        for y in 0..h {
            for x in 0..w {
                let cell = &buf[(x, y)];
                match cell.fg {
                    Color::Rgb(r, g, b) => f.push_str(&format!("<span style=color:rgb({r},{g},{b})>{}</span>", cell.symbol())),
                    _ => f.push(' '),
                }
            }
            f.push('\n');
        }
        let label = sig.kids.iter().map(|k| k.1.label()).collect::<Vec<_>>().join(" · ");
        frames.push(format!("[{:?},{:?}]", f, label));
    }
    let anim = format!(
        "<!doctype html><meta charset=utf-8><body style='background:#05090d;color:#ccc;font:14px monospace;padding:20px'>\
         <pre id=f style='font-family:\"JetBrainsMono Nerd Font\",monospace;font-size:22px;line-height:1.12;margin:0'></pre>\
         <div id=l style='font-size:16px;margin-top:8px'></div><script>const F=[{}];let i=0;\
         setInterval(()=>{{const[x,l]=F[i++%F.length];f.innerHTML=x;document.getElementById('l').textContent=l;}},40)</script>",
        frames.join(",")
    );
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/target/nucleo-hijos.html"), anim).unwrap();

    // One kid per state, still in its orbit, to compare the silhouettes side by side.
    let states = [State::Thinking, State::Reading, State::Searching, State::Editing, State::Running, State::Web];
    for (n, st) in states.iter().enumerate() {
        let mut c = run(State::Idle, 2.6, &Signals::default());
        let sig = Signals { kids: vec![(0, *st)], ..Default::default() };
        c.kid_born(0);
        for _ in 0..60 {
            c.step(0.04, State::Idle, &sig);
        }
        let (w, h) = (560, 560);
        let (img, colors) = c.pixels(w, h, 4);
        let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
        for &i in &img {
            let px = if i == 0 { [5.0, 9.0, 13.0] } else { colors[i as usize - 1] };
            ppm.extend(px.map(|v| v.round() as u8));
        }
        std::fs::write(format!("{}/target/estado-{n}.ppm", env!("CARGO_MANIFEST_DIR")), ppm).unwrap();
    }

    // And stills in image dots, as seen in foot: the same script, individual frames.
    let mut c = run(State::Idle, 2.6, &Signals::default());
    let mut sig = Signals { ctx: 0.3, ..Default::default() };
    let (mut t, mut next) = (0.0, 0);
    let shots = [1.3, 2.75, 5.0, 6.62, 8.5, 9.9];
    let mut shot = 0;
    while shot < shots.len() {
        while next < script.len() && t >= script[next].0 {
            (script[next].1)(&mut c, &mut sig);
            next += 1;
        }
        if t > 6.0 && t < 6.04 {
            c.kid_pulse(2, false);
        }
        c.step(0.04, State::Idle, &sig);
        t += 0.04;
        if t >= shots[shot] {
            let (w, h) = (480, 300);
            let (img, colors) = c.pixels(w, h, 4);
            let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
            for &i in &img {
                let px = if i == 0 { [5.0, 9.0, 13.0] } else { colors[i as usize - 1] };
                ppm.extend(px.map(|v| v.round() as u8));
            }
            std::fs::write(format!("{}/target/hijo-{shot}.ppm", env!("CARGO_MANIFEST_DIR")), ppm).unwrap();
            shot += 1;
        }
    }
}

/// Store 2b: the built-in eye compiled to WebAssembly (`crates/pieces/eye`), in the place of
/// `builtin:eye` in the original recipe, draws the same dots in every state, with signals,
/// events and kids.
#[cfg(feature = "pieces")]
#[test]
fn wasm_eye_draws_the_same_as_native() {
    use super::recipe::{original, Assembled, Recipe};
    use std::hash::{DefaultHasher, Hash, Hasher};
    let eye = concat!(env!("CARGO_MANIFEST_DIR"), "/crates/pieces/eye/eye.wasm");
    let src = include_str!("recipe/original.toml").replace("use = \"builtin:eye\"", &format!("use = \"file:{eye}\""));
    assert_ne!(src, include_str!("recipe/original.toml"));
    let wasm = Recipe::parse(&src).unwrap();
    let core = |r: &Recipe| Core::with(vec![Box::new(Assembled::new(r).unwrap())], 0.0, State::Booting);
    let fingerprint = |c: &Core| {
        let mut h = DefaultHasher::new();
        for (w, hh) in [(36, 15), (60, 22)] {
            for cell in render(c, w, hh).content() {
                cell.symbol().hash(&mut h);
            }
        }
        c.pixels(240, 200, 4).0.hash(&mut h);
        h.finish()
    };
    let signals = [Signals::default(), Signals { level: 0.6, ctx: 0.85, music: 0.7, ..Default::default() }];
    let mut frames = 0;
    for sig in &signals {
        for s in ALL {
            let (mut a, mut b) = (core(&original()), core(&wasm));
            for step in 0..40 {
                for c in [&mut a, &mut b] {
                    c.step(0.04, s, sig);
                    if step == 15 {
                        c.fire(Event::Error);
                    }
                    if step == 20 {
                        c.kid_born(1);
                    }
                }
                if step % 5 == 0 {
                    assert_eq!(fingerprint(&a), fingerprint(&b), "{s:?} en el paso {step}");
                    frames += 1;
                }
            }
        }
    }
    assert!(frames > 300);
}
