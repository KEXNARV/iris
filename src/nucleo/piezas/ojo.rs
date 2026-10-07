//! OJO: la pupila del cuerpo. Se dilata según el estado, se contrae de golpe con un error, se
//! entrecierra al bostezar y al parpadear queda solo una raya. Mira hacia donde diga la mirada.
//! Es una capa de adentro del cuerpo, la de más arriba.

use super::Relleno;
use crate::nucleo::rejilla::Grid;

pub(crate) struct Ojo {
    radio: f64,
    /// Nunca baja de estos puntos de radio (en el panel grande de Cine se perdía).
    min_puntos: f64,
    /// Cuánto se contrae con un error y al bostezar; cuánto crece bajo la lupa.
    error: f64,
    bostezo: f64,
    lupa: f64,
}

/// Lo que el ojo necesita del buddy en este cuadro.
pub(crate) struct Mando {
    /// Centro de la pupila (el del cuerpo más la mirada).
    pub x: f64,
    pub y: f64,
    /// Cuánto se ve (0 sin pupila) y cuánto se abre (>1 dilatada).
    pub pupil: f64,
    pub dilate: f64,
    pub red: f64,
    pub yawn: f64,
    /// Bajo la lupa se agranda.
    pub lens: f64,
    pub parpadeo: bool,
    pub tinta: u8,
    /// Lo que mide un punto.
    pub du: f64,
}

/// La pupila en este cuadro.
pub(crate) struct Pupila {
    x: f64,
    y: f64,
    r: f64,
    parpadeo: bool,
    tinta: u8,
}

impl Ojo {
    pub fn new() -> Self {
        Ojo { radio: 0.085, min_puntos: 2.5, error: 0.45, bostezo: 0.8, lupa: 1.5 }
    }

    pub fn pupila(&self, m: Mando) -> Pupila {
        let pr = if m.pupil > 0.01 {
            (self.radio * m.dilate * (1.0 - m.red * self.error)).max(self.min_puntos * m.du) * m.pupil * (1.0 - m.yawn * self.bostezo)
        } else {
            0.0
        };
        let r = pr * if m.lens > 0.5 { self.lupa } else { 1.0 };
        Pupila { x: m.x, y: m.y, r, parpadeo: m.parpadeo, tinta: m.tinta }
    }
}

impl Relleno for Pupila {
    fn punto(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        let (ex, ey) = (x - self.x, y - self.y);
        let er = ex.hypot(ey);
        if self.r > 0.01 && er < self.r {
            // Al parpadear queda solo una raya.
            if !self.parpadeo || ey.abs() < 0.02 {
                g.dot(i, j, 3, self.tinta);
            }
            return true;
        }
        false
    }
}
