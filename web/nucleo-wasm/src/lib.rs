//! El núcleo de Iris en la página. No es una copia: incluye los mismos archivos que compila la
//! terminal (`src/nucleo`, `theme`, `baymax`, `rutas`, `sixel`) y les da una salida para el
//! navegador. La página llama a `iris_paso` en cada cuadro y pinta lo que deja `iris_imagen`.

#![allow(dead_code, unused_imports)]

#[path = "../../../src/baymax.rs"]
mod baymax;
#[path = "../../../src/nucleo/mod.rs"]
mod nucleo;
#[path = "../../../src/rutas.rs"]
mod rutas;
#[path = "../../../src/sixel.rs"]
mod sixel;
#[path = "../../../src/theme.rs"]
mod theme;

use std::cell::RefCell;

use nucleo::{Core, Signals, State};

/// Los estados en el orden en que los nombra la página (`ESTADOS` en index.html).
const ESTADOS: [State; 12] = [
    State::Idle,
    State::Listening,
    State::Transcribing,
    State::Thinking,
    State::Planning,
    State::Reading,
    State::Searching,
    State::Editing,
    State::Running,
    State::Delegating,
    State::Speaking,
    State::Sleeping,
];

struct Iris {
    core: Core,
    sig: Signals,
    img: Vec<u8>,
    paleta: Vec<f32>,
}

thread_local! {
    static IRIS: RefCell<Option<Iris>> = const { RefCell::new(None) };
}

fn con<T>(f: impl FnOnce(&mut Iris) -> T) -> T {
    IRIS.with(|c| {
        let mut c = c.borrow_mut();
        let iris = c.get_or_insert_with(|| Iris { core: Core::new(), sig: Signals::default(), img: vec![], paleta: vec![] });
        f(iris)
    })
}

/// Avanza `dt` segundos hacia el estado `estado` (índice de `ESTADOS`), con el nivel de la voz
/// (0..1, al escuchar o hablar) y de la música.
#[unsafe(no_mangle)]
pub extern "C" fn iris_paso(dt: f64, estado: u32, voz: f32, musica: f32) {
    con(|i| {
        let want = ESTADOS.get(estado as usize).copied().unwrap_or(State::Idle);
        i.sig.level = voz;
        i.sig.music = musica;
        i.core.step(dt, want, &i.sig);
    });
}

/// Un subagente: 0 nace, 1 trabaja (pulso), 2 termina bien, 3 termina con error.
#[unsafe(no_mangle)]
pub extern "C" fn iris_hijo(que: u32, id: u32) {
    con(|i| {
        let id = id as u64;
        match que {
            0 => {
                i.core.kid_born(id);
                i.sig.kids.push((id, State::Reading));
            }
            1 => i.core.kid_pulse(id, false),
            _ => {
                i.core.kid_end(id, que == 2);
                i.sig.kids.retain(|k| k.0 != id);
            }
        }
    });
}

/// Quién vive en el núcleo: 0 el original, 1 Baymax.
#[unsafe(no_mangle)]
pub extern "C" fn iris_buddy(cual: u32) {
    theme::usar((cual == 1).then(|| "baymax-web".into()));
}

/// Dibuja el núcleo en una imagen de `w`×`h` píxeles con puntos cada `sp`: un índice de color
/// por píxel (0 = nada). Devuelve cuántos colores tiene la paleta.
#[unsafe(no_mangle)]
pub extern "C" fn iris_imagen(w: u32, h: u32, sp: u32) -> u32 {
    con(|i| {
        let (img, pal) = i.core.pixels(w as usize, h as usize, sp as usize);
        i.img = img;
        i.paleta = pal.iter().flat_map(|c| c.map(|v| v as f32)).collect();
        pal.len() as u32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn iris_imagen_ptr() -> *const u8 {
    con(|i| i.img.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn iris_paleta_ptr() -> *const f32 {
    con(|i| i.paleta.as_ptr())
}
