//! How Iris looks: `draw` picks the /theme style and on top opens what applies to all (^G,
//! ^T, ^O). Each style lives in `styles/`; what they share, in the sibling modules.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::style::UiStyle;
use crate::{theme, Activity, App, Modal, Role, ToolStatus, VoiceState};

mod views;
mod chat;
mod modals;
mod panels;
mod data;
mod pieces;
mod text;
mod styles;

use views::*;
use chat::*;
use modals::*;
use panels::*;
use data::*;
use pieces::*;
use text::*;
use styles::*;

pub(in crate::ui) const WARM: Color = Color::Rgb(255, 170, 40);

pub(in crate::ui) const RED: Color = Color::Rgb(255, 90, 90);
pub(crate) const GREEN: Color = Color::Rgb(90, 230, 150);

pub(in crate::ui) const SPIN: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub use crate::core::State;

pub fn draw(f: &mut Frame, app: &App) {
    theme::use_palette(app.buddy.palette());
    // Styles without panels need room; on a small terminal the clásico one is shown.
    let a = f.area();
    let fits = |w: u16, h: u16| a.width >= w && a.height >= h;
    // ^T (the whole conversation) and ^O (the whole reply) work in every style. Cine has
    // them inside its design; in the others they open full screen, like ^G.
    if !(app.style == UiStyle::Cine && fits(64, 22)) {
        app.reading.set(false);
        if app.transcript {
            f.render_widget(Clear, a);
            transcript_view(f, app, a);
            if app.tools_view.is_some() {
                tools_overlay(f, app);
            }
            return;
        }
        if app.read_override == Some(true) && reading(f, app) {
            if app.tools_view.is_some() {
                tools_overlay(f, app);
            }
            return;
        }
    }
    match app.style {
        UiStyle::Propuesta if fits(60, 16) => propuesta(f, app),
        UiStyle::Cabina if fits(70, 18) => cabina(f, app),
        UiStyle::Cine if fits(64, 22) => cine(f, app),
        UiStyle::Fosforo if fits(70, 18) => fosforo(f, app),
        UiStyle::Radar if fits(70, 20) => radar(f, app),
        UiStyle::Editorial if fits(70, 20) => editorial(f, app),
        UiStyle::Zen if fits(40, 12) => zen(f, app),
        UiStyle::Bitacora if fits(60, 12) => bitacora(f, app),
        UiStyle::Tablero if fits(80, 24) => tablero(f, app),
        _ => clasico(f, app),
    }
    if app.tools_view.is_some() {
        tools_overlay(f, app);
    }
}

/// Clears whatever is under a window.
pub(in crate::ui) fn clear_area(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
}

pub(in crate::ui) fn listening(state: State) -> bool {
    matches!(state, State::Listening | State::NoVoice)
}
