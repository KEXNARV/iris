//! El contrato entre Iris y una pieza (ABI 1). Lo usan los dos lados: la pieza al compilarse a
//! WebAssembly y el host de Iris al cargarla, así no se pueden desincronizar.
//!
//! Exports de la pieza: `iris_abi() -> u32`, `iris_espacio() -> u32`, `iris_entrada() -> u32`
//! (dónde escribe Iris la [`Entrada`]), `iris_nueva()`, `iris_paso()`, `iris_evento(u32)` y
//! `iris_pintar() -> u64` (dirección << 32 | cantidad de [`Orden`]es).
//! Imports: solo `iris.azar() -> f64`.

/// La versión del contrato. Una pieza con otra no se carga.
pub const VERSION: u32 = 1;

/// Cuántas órdenes de dibujo por cuadro como máximo; las demás se ignoran.
pub const MAX_ORDENES: usize = 20_000;

pub const MODO_PLOT: u8 = 0;
pub const MODO_DEBAJO: u8 = 1;

/// Lo que Iris escribe en la memoria de la pieza antes de cada llamada. Todo en `f64`/`u32`,
/// sin punteros: el mismo layout en el host (64 bits) y en wasm32.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Entrada {
    pub t: f64,
    pub dt: f64,
    pub desde: f64,
    pub calma: f64,
    pub voz: f64,
    pub musica: f64,
    pub contexto: f64,
    pub ancla_cx: f64,
    pub ancla_cy: f64,
    pub ancla_radio: f64,
    pub ancla_base: f64,
    /// El tamaño de un punto en unidades del mundo.
    pub du: f64,
    pub estado: u32,
    pub ancla_tinta: u32,
    pub ancla_borde: u32,
    pub _relleno: u32,
    /// Los parámetros propios del espacio (ver [`Espacio`]).
    pub p: [f64; 16],
}

impl Entrada {
    pub const CERO: Entrada = Entrada {
        t: 0.0, dt: 0.0, desde: 0.0, calma: 1.0, voz: 0.0, musica: 0.0, contexto: 0.0,
        ancla_cx: 0.0, ancla_cy: 0.0, ancla_radio: 0.0, ancla_base: 0.0, du: 0.0,
        estado: 0, ancla_tinta: 0, ancla_borde: 0, _relleno: 0, p: [0.0; 16],
    };
}

/// Una orden de dibujo: un punto en coordenadas del mundo.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orden {
    pub x: f64,
    pub y: f64,
    pub tono: u8,
    pub tinta: u8,
    pub modo: u8,
    pub _relleno: [u8; 5],
}

/// A qué parte del núcleo va una pieza. Cada espacio define qué hay en `Entrada::p`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Espacio {
    /// Las motas alrededor del cuerpo. `p`: 0 radio del cuerpo, 1 nivel de voz, 2 tragando
    /// (de escuchar a transcribir), 3 baile, 4 cuánto se ven, 5 arranque, 6 prensas
    /// (compactando), 7 y 8 la mirada, 9 modo liviano (1 = sí).
    Particulas = 1,
}

impl Espacio {
    pub fn de(n: u32) -> Option<Self> {
        match n {
            1 => Some(Espacio::Particulas),
            _ => None,
        }
    }
}

/// Los estados del núcleo, en el orden de Iris.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Arrancando, Durmiendo, Reposo, Escribiendo, Escuchando, SinVoz, Transcribiendo, Pensando,
    Planeando, Buscando, Leyendo, Editando, Corriendo, Probando, Git, Web, Delegando, Hablando,
    Preguntando, Compactando, Desconectado,
}

impl Estado {
    pub const TODOS: [Estado; 21] = [
        Estado::Arrancando, Estado::Durmiendo, Estado::Reposo, Estado::Escribiendo, Estado::Escuchando,
        Estado::SinVoz, Estado::Transcribiendo, Estado::Pensando, Estado::Planeando, Estado::Buscando,
        Estado::Leyendo, Estado::Editando, Estado::Corriendo, Estado::Probando, Estado::Git, Estado::Web,
        Estado::Delegando, Estado::Hablando, Estado::Preguntando, Estado::Compactando, Estado::Desconectado,
    ];

    pub fn de(n: u32) -> Self {
        Estado::TODOS.get(n as usize).copied().unwrap_or(Estado::Reposo)
    }
}

/// Lo que pasa de golpe, en el orden de Iris.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evento {
    Tecla, Error, Listo, Cancela, Descarta, Vuelve, Copia, Pasa,
}

impl Evento {
    pub fn de(n: u32) -> Option<Self> {
        [Evento::Tecla, Evento::Error, Evento::Listo, Evento::Cancela, Evento::Descarta, Evento::Vuelve, Evento::Copia, Evento::Pasa].get(n as usize).copied()
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "iris")]
unsafe extern "C" {
    #[link_name = "azar"]
    pub fn iris_azar() -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_layout_no_cambia() {
        // Si esto cambia, es otra versión del contrato.
        assert_eq!(core::mem::size_of::<Entrada>(), 12 * 8 + 4 * 4 + 16 * 8);
        assert_eq!(core::mem::size_of::<Orden>(), 24);
    }
}
