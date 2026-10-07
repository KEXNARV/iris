//! BUDDIES: quién vive en el núcleo. Cada buddy compone sus propias piezas, con su propio
//! guion, y decide en qué orden se dibujan; el núcleo solo le pasa lo que pasa y le pide que
//! se pinte.

use super::contexto::{Aviso, Contexto};
use super::estado::State;
use super::rejilla::Grid;

pub(crate) mod baymax;
pub(crate) mod original;

pub(crate) trait Buddy {
    /// Avanza `dt` segundos. El núcleo ya movió el reloj y el estado.
    fn step(&mut self, ctx: &Contexto, dt: f64);

    /// Cambió el estado que se ve (el núcleo ya lo anotó en `ctx`).
    fn cambio(&mut self, de: State, a: State, ctx: &Contexto);

    /// Un evento o algo de los subagentes.
    fn aviso(&mut self, aviso: Aviso, ctx: &Contexto);

    fn paint(&self, g: &mut Grid, ctx: &Contexto);

    /// Los colores de sus tintas, por tono (1 apagado … 3 pleno). Las de base (estado, verde,
    /// rojo, ámbar, acento) van primero y en ese orden.
    fn paleta(&self) -> Vec<[[f64; 3]; 4]>;

    /// Su color ahora, con las transiciones y el rojo de los errores.
    fn color(&self) -> [f64; 3];

    /// Cuánto error lleva encima (0..1): el principal tiñe de rojo la línea de un hijo así.
    fn alarma(&self) -> f64;

    /// Cuántos subagentes tiene a la vista.
    #[cfg(test)]
    fn hijos(&self) -> usize;

    /// Para que las pruebas miren adentro.
    #[cfg(test)]
    fn como_any(&self) -> &dyn std::any::Any;
}
