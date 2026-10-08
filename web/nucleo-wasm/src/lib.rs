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
    /// Cada núcleo de la página es uno: el grande de arriba y los de la rejilla de estados.
    static NUCLEOS: RefCell<Vec<Iris>> = const { RefCell::new(Vec::new()) };
}

fn con<T>(id: u32, f: impl FnOnce(&mut Iris) -> T) -> T {
    NUCLEOS.with(|n| f(&mut n.borrow_mut()[id as usize]))
}

/// Un núcleo nuevo, arrancando. Devuelve su id para las demás llamadas.
#[unsafe(no_mangle)]
pub extern "C" fn iris_nuevo() -> u32 {
    NUCLEOS.with(|n| {
        let mut n = n.borrow_mut();
        n.push(Iris { core: Core::new(), sig: Signals::default(), img: vec![], paleta: vec![] });
        (n.len() - 1) as u32
    })
}

/// Avanza `dt` segundos hacia el estado `estado` (índice de `ESTADOS`), con el nivel de la voz
/// (0..1, al escuchar o hablar) y de la música.
#[unsafe(no_mangle)]
pub extern "C" fn iris_paso(id: u32, dt: f64, estado: u32, voz: f32, musica: f32) {
    con(id, |i| {
        let want = ESTADOS.get(estado as usize).copied().unwrap_or(State::Idle);
        i.sig.level = voz;
        i.sig.music = musica;
        i.core.step(dt, want, &i.sig);
    });
}

/// Un subagente: 0 nace, 1 trabaja (pulso), 2 termina bien, 3 termina con error.
#[unsafe(no_mangle)]
pub extern "C" fn iris_hijo(id: u32, que: u32, hijo: u32) {
    con(id, |i| {
        let hijo = hijo as u64;
        match que {
            0 => {
                i.core.kid_born(hijo);
                i.sig.kids.push((hijo, State::Reading));
            }
            1 => i.core.kid_pulse(hijo, false),
            _ => {
                i.core.kid_end(hijo, que == 2);
                i.sig.kids.retain(|k| k.0 != hijo);
            }
        }
    });
}

/// Algo que pasa de golpe: 0 error, 1 listo, 2 cancelado.
#[unsafe(no_mangle)]
pub extern "C" fn iris_evento(id: u32, que: u32) {
    use nucleo::Event;
    let ev = match que {
        0 => Event::Error,
        1 => Event::Done,
        _ => Event::Cancel,
    };
    con(id, |i| i.core.fire(ev));
}

/// Quién vive en los núcleos: 0 el original, 1 Baymax.
#[unsafe(no_mangle)]
pub extern "C" fn iris_buddy(cual: u32) {
    theme::usar((cual == 1).then(|| "baymax-web".into()));
}

/// Dibuja el núcleo en una imagen de `w`×`h` píxeles con puntos cada `sp`: un índice de color
/// por píxel (0 = nada). Devuelve cuántos colores tiene la paleta.
#[unsafe(no_mangle)]
pub extern "C" fn iris_imagen(id: u32, w: u32, h: u32, sp: u32) -> u32 {
    con(id, |i| {
        let (img, pal) = i.core.pixels(w as usize, h as usize, sp as usize);
        i.img = img;
        i.paleta = pal.iter().flat_map(|c| c.map(|v| v as f32)).collect();
        pal.len() as u32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn iris_imagen_ptr(id: u32) -> *const u8 {
    con(id, |i| i.img.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn iris_paleta_ptr(id: u32) -> *const f32 {
    con(id, |i| i.paleta.as_ptr())
}
