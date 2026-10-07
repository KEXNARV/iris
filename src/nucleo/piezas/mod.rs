//! PIEZAS: lo que un buddy puede componer. Cada una tiene su estado, su configuración (como
//! campos de la instancia: dos halos pueden ser dos instancias distintas), su animación, sus
//! reacciones y su dibujo. No saben qué buddy las usa: lo que necesitan de él (intensidades,
//! dónde está el cuerpo) se lo pasa el buddy en cada cuadro.

use super::azar::Azar;
use super::contexto::Contexto;
use super::estado::Event;
use super::rejilla::Grid;

pub(crate) mod arcos;
pub(crate) mod brote;
pub(crate) mod cuerpo;
pub(crate) mod escala;
pub(crate) mod gestos;
pub(crate) mod git;
pub(crate) mod hijos;
pub(crate) mod ideas;
pub(crate) mod lectura;
pub(crate) mod liquido;
pub(crate) mod lupa;
pub(crate) mod marcas;
pub(crate) mod mirada;
pub(crate) mod ojo;
pub(crate) mod ondas;
pub(crate) mod paquetes;
pub(crate) mod particulas;
pub(crate) mod prensas;
pub(crate) mod pregunta;
pub(crate) mod puntadas;
pub(crate) mod radar;
pub(crate) mod resorte;
pub(crate) mod voz;
pub(crate) mod zetas;

/// Una pieza. Todo es opcional: hay piezas que solo se dibujan, y otras que solo se mueven
/// (la mirada, el resorte).
pub(crate) trait Pieza {
    /// Lo que le pasa el buddy para avanzar un cuadro (`()` si no lo necesita).
    type Paso<'a>;
    /// Lo que le pasa el buddy para dibujarse (`()` si no lo necesita).
    type Pinta<'a>;

    fn step(&mut self, _ctx: &Contexto, _paso: Self::Paso<'_>, _dt: f64) {}

    /// Un evento de Iris. El azar es el del buddy: si la pieza lo usa, el orden importa.
    fn evento(&mut self, _ev: Event, _ctx: &Contexto, _azar: &mut Azar) {}

    fn paint(&self, _g: &mut Grid, _ctx: &Contexto, _m: Self::Pinta<'_>) {}
}

/// Una capa de adentro de un cuerpo: el cuerpo recorre sus puntos interiores y le pregunta a
/// cada capa, en orden, si el punto es suyo. La primera que dice que sí lo pinta (o lo deja
/// vacío) y las de abajo no lo ven.
pub(crate) trait Relleno {
    fn punto(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool;
}

/// Sale pasándose un poco y vuelve: para lo que nace.
pub(crate) fn ease_out_back(u: f64) -> f64 {
    1.0 + 2.7 * (u - 1.0).powi(3) + 1.7 * (u - 1.0).powi(2)
}
