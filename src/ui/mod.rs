//! Cómo se ve Iris: `draw` elige el estilo de /theme y encima abre lo que vale en todos (^G,
//! ^T, ^O). Cada estilo vive en `estilos/`; lo que comparten, en los módulos de al lado.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::estilo::Estilo;
use crate::{theme, Activity, App, Modal, Role, ToolStatus, VoiceState};

mod vistas;
mod chat;
mod modales;
mod paneles;
mod datos;
mod piezas;
mod texto;
mod estilos;

use vistas::*;
use chat::*;
use modales::*;
use paneles::*;
use datos::*;
use piezas::*;
use texto::*;
use estilos::*;

pub(in crate::ui) const WARM: Color = Color::Rgb(255, 170, 40);

pub(in crate::ui) const RED: Color = Color::Rgb(255, 90, 90);
pub(crate) const GREEN: Color = Color::Rgb(90, 230, 150);

pub(in crate::ui) const SPIN: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub use crate::nucleo::State;

pub fn draw(f: &mut Frame, app: &App) {
    theme::usar(app.buddy.paleta());
    // Los estilos sin paneles necesitan aire; en una terminal chica se ve el clásico.
    let a = f.area();
    let fits = |w: u16, h: u16| a.width >= w && a.height >= h;
    // ^T (la conversación entera) y ^O (la respuesta entera) valen en todos los estilos. Cine
    // las trae dentro de su diseño; en los demás se abren a pantalla completa, como ^G.
    if !(app.estilo == Estilo::Cine && fits(64, 22)) {
        app.reading.set(false);
        if app.transcript {
            f.render_widget(Clear, a);
            transcript_view(f, app, a);
            if app.tools_view.is_some() {
                tools_overlay(f, app);
            }
            return;
        }
        if app.read_override == Some(true) && lectura(f, app) {
            if app.tools_view.is_some() {
                tools_overlay(f, app);
            }
            return;
        }
    }
    match app.estilo {
        Estilo::Propuesta if fits(60, 16) => propuesta(f, app),
        Estilo::Cabina if fits(70, 18) => cabina(f, app),
        Estilo::Cine if fits(64, 22) => cine(f, app),
        Estilo::Fosforo if fits(70, 18) => fosforo(f, app),
        Estilo::Radar if fits(70, 20) => radar(f, app),
        Estilo::Editorial if fits(70, 20) => editorial(f, app),
        Estilo::Zen if fits(40, 12) => zen(f, app),
        Estilo::Bitacora if fits(60, 12) => bitacora(f, app),
        Estilo::Tablero if fits(80, 24) => tablero(f, app),
        _ => clasico(f, app),
    }
    if app.tools_view.is_some() {
        tools_overlay(f, app);
    }
}

/// Borra lo que haya debajo de una ventana.
pub(in crate::ui) fn limpiar(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
}

pub(in crate::ui) fn listening(state: State) -> bool {
    matches!(state, State::Listening | State::NoVoice)
}
