use super::*;
use super::buddies::original::Original;
use super::color::{apart, hsv, INK_RED};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// El buddy original de un núcleo, para mirar adentro.
fn blob(c: &Core) -> &Original<Blob> {
    c.buddies[0].como_any().downcast_ref().unwrap()
}

const ALL: [State; 21] = [
    State::Booting, State::Sleeping, State::Idle, State::Typing, State::Listening, State::NoVoice,
    State::Transcribing, State::Thinking, State::Planning, State::Searching, State::Reading, State::Editing,
    State::Running, State::Testing, State::Git, State::Web, State::Delegating, State::Speaking, State::Asking,
    State::Compacting, State::Offline,
];

#[test]
fn clasifica_herramientas() {
    assert_eq!(tool_state("Read", "src/ui.rs"), State::Reading);
    assert_eq!(tool_state("Grep", "draw_core"), State::Searching);
    assert_eq!(tool_state("Write", "x"), State::Editing);
    assert_eq!(tool_state("WebSearch", "x"), State::Web);
    assert_eq!(tool_state("Agent", "x"), State::Delegating);
    assert_eq!(tool_state("TodoWrite", ""), State::Planning);
    assert_eq!(tool_state("mcp__claude_ai_Dipro__query", ""), State::Running);
}

#[test]
fn clasifica_comandos_de_bash() {
    for c in ["git push", "cd ~/code/jarvis && git status --short", "GIT_PAGER=cat git log", "git"] {
        assert_eq!(tool_state("Bash", c), State::Git, "{c}");
    }
    for c in ["cargo test", "cd x && cargo test core", "RUST_LOG=1 cargo test", "pnpm vitest run", "npx jest", "python -m pytest -q"] {
        assert_eq!(tool_state("Bash", c), State::Testing, "{c}");
    }
    for c in ["cargo build", "ls -la", "echo git", "grep -rn digit src", "legit status"] {
        assert_eq!(tool_state("Bash", c), State::Running, "{c}");
    }
    // Un `git` seguido de pruebas es una corrida de pruebas.
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
fn en_reposo_baila_con_la_musica() {
    let fuerte = Signals { music: 0.9, ..Default::default() };
    let mut c = run(State::Idle, 4.0, &fuerte);
    assert!(blob(&c).animo.groove > 0.99);
    // El golpe fuerte llega al rojo en la escala, como la barra del teclado.
    let (px, _) = c.pixels(200, 200, 4);
    assert!(px.contains(&(1 + INK_RED * 3 + 2)), "sin rojo en la escala");
    let quieto = run(State::Idle, 4.0, &Signals::default()).pixels(200, 200, 4).0;
    assert!(!quieto.contains(&(1 + INK_RED * 3 + 2)));
    // Una tecla lo saca de reposo y deja de bailar enseguida.
    for _ in 0..10 {
        c.step(0.04, State::Typing, &fuerte);
    }
    assert!(blob(&c).animo.groove < 0.1);
    // Sin música no baila.
    assert_eq!(blob(&run(State::Idle, 4.0, &Signals::default())).animo.groove, 0.0);
}

#[test]
fn todos_los_estados_dibujan_algo_en_varios_tamanos() {
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
fn los_estados_se_separan_del_acento() {
    let azul = [69.0, 123.0, 255.0];
    let pensando = [90.0, 150.0, 255.0];
    let (h, _, _) = hsv(apart(pensando, azul));
    let (ha, _, _) = hsv(azul);
    assert!(((h - ha + 540.0) % 360.0 - 180.0).abs() >= 39.9);
    // Lo que ya está lejos no se toca.
    let verde = [120.0, 235.0, 120.0];
    assert_eq!(apart(verde, azul), verde);
}

#[test]
fn es_determinista() {
    let _turno = crate::theme::tests::TURNO.lock().unwrap_or_else(|e| e.into_inner());
    let sig = Signals::default();
    let shape = |b: Buffer| b.content().iter().map(|c| c.symbol().to_string()).collect::<String>();
    let a = shape(render(&run(State::Thinking, 2.0, &sig), 36, 15));
    let b = shape(render(&run(State::Thinking, 2.0, &sig), 36, 15));
    assert_eq!(a, b);
}

#[test]
fn un_read_corto_no_parpadea() {
    let sig = Signals::default();
    let mut c = run(State::Thinking, 1.0, &sig);
    c.step(0.04, State::Reading, &sig);
    assert_eq!(c.state(), State::Reading);
    // El Read terminó enseguida, pero se sigue viendo un momento.
    c.step(0.04, State::Thinking, &sig);
    assert_eq!(c.state(), State::Reading);
    for _ in 0..20 {
        c.step(0.04, State::Thinking, &sig);
    }
    assert_eq!(c.state(), State::Thinking);
    // Lo urgente entra sin esperar.
    c.step(0.04, State::Reading, &sig);
    c.step(0.04, State::Asking, &sig);
    assert_eq!(c.state(), State::Asking);
}

#[test]
fn eventos_no_rompen_nada() {
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
fn hijos_nacen_trabajan_y_vuelven() {
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
        let before = blob(&c).ondas.listos.len();
        c.step(0.04, State::Idle, &sig);
        bloomed |= blob(&c).ondas.listos.len() > before;
        render(&c, 36, 15);
    }
    // El que salió bien volvió y se fundió (con su destello); el que falló se apagó.
    assert_eq!(c.kid_count(), 1);
    assert!(bloomed);
    // Reiniciar el motor se los lleva.
    c.reboot();
    assert_eq!(c.kid_count(), 0);
}

/// Huella de cómo se ve el núcleo: cada estado, con y sin Baymax, con señales, eventos e
/// hijos, en braille y en puntos de imagen, a lo largo de varios segundos. Sirve para partir el
/// núcleo en piezas sin cambiar nada de lo que se ve:
/// `IRIS_DORADO=/ruta cargo test nucleo::tests::dorado -- --ignored` la escribe ahí, y si el
/// archivo ya existe la compara con él.
#[test]
#[ignore]
fn dorado() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let Some(ruta) = crate::rutas::var("DORADO") else { return };
    let _turno = crate::theme::tests::TURNO.lock().unwrap_or_else(|e| e.into_inner());
    let paleta = std::env::temp_dir().join("iris-dorado-baymax.toml");
    let huella = |c: &Core| {
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
        crate::theme::usar(baymax.then(|| paleta.clone()));
        let señales = [
            Signals::default(),
            Signals { level: 0.6, ctx: 0.85, queue: 3, todos: (6, 4), idle: 40.0, music: 0.7, ..Default::default() },
            Signals { calm: true, ctx: 0.3, kids: vec![(1, State::Searching), (2, State::Editing)], ..Default::default() },
        ];
        for (n, sig) in señales.iter().enumerate() {
            for s in ALL {
                let mut c = Core::new();
                if !sig.kids.is_empty() {
                    c.kid_born(1);
                    c.kid_born(2);
                }
                for paso in 0..90 {
                    c.step(0.04, s, sig);
                    match paso {
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
                    if paso == 40 && !sig.kids.is_empty() {
                        c.kid_pulse(1, false);
                        c.kid_end(2, true);
                    }
                    if paso % 6 == 0 {
                        out.push_str(&format!("{baymax} {n} {s:?} {paso} {:016x}\n", huella(&c)));
                    }
                }
            }
        }
    }
    crate::theme::usar(None);
    match std::fs::read_to_string(&ruta) {
        Ok(antes) => {
            let distintas: Vec<_> = antes.lines().zip(out.lines()).filter(|(a, b)| a != b).map(|(a, _)| a.to_string()).collect();
            assert!(antes.lines().count() == out.lines().count() && distintas.is_empty(), "{} cuadros cambiaron, el primero: {:?}", distintas.len(), distintas.first());
        }
        Err(_) => std::fs::write(&ruta, out).unwrap(),
    }
}

/// `IRIS_SNAPSHOT=1 cargo test nucleo::tests::snapshot -- --nocapture` deja target/nucleo.html con todos
/// los estados, para mirarlos en el navegador.
#[test]
/// `IRIS_SNAPSHOT=1 cargo test nucleo::tests::baymax_ppm` deja en target/baymax/ la cara
/// de Baymax en varios estados, como la ve foot (Sixel), con la paleta Baymax de verdad.
fn baymax_ppm() {
    if !crate::rutas::hay_var("SNAPSHOT") {
        return;
    }
    let home = std::env::var("HOME").unwrap();
    crate::theme::usar(Some(format!("{home}/.config/iris/colores-baymax.toml").into()));
    let dir = std::path::Path::new("target/baymax");
    std::fs::create_dir_all(dir).unwrap();
    let bg = crate::baymax::hacia(crate::theme::fondo_rgb(), 0, 0.0);
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
    if !crate::rutas::hay_var("SNAPSHOT") {
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

    // Animación: cuadros reales del núcleo a 25 fps, para ver lo que una foto no muestra.
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

    // Hijos: nacen tres, trabajan en cosas distintas, mandan partículas y se van.
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
            // Una pulsación por segundo, como un agente que trabaja.
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

    // Un hijo por estado, quieto en su órbita, para comparar las siluetas lado a lado.
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

    // Y fotos en puntos de imagen, como se ve en foot: el mismo guion, cuadros sueltos.
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
