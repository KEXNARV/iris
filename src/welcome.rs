//! The welcome: the first time Iris opens (or with `/welcome`) you pick permissions, style,
//! color, buddy, voice, model and effort, and at the end some cards showing what you can do.
//! Each step is saved when chosen; what's visible (style, color, buddy) changes as you move
//! through the list.

use crate::settings::{self, Permissions};
use crate::app::{App, VoiceMode};
use crate::{buddy, commands, style, theme};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Step {
    Hello,
    Session,
    Permissions,
    UiStyle,
    Accent,
    Buddy,
    Voice,
    Model,
    Effort,
    Tour(usize),
}

/// (title, text) of each tour card.
pub const TOUR: [(&str, &str); 6] = [
    (
        "Háblame",
        "Mantén espacio y habla; al soltarlo te escucho y lo envío. Un toque empieza y otro termina. \
         Si me hablas, te contesto en voz alta (/voice cambia cuándo).",
    ),
    (
        "Comandos",
        "Escribe / y aparece todo lo que sé hacer: /model, /effort, /theme, /buddy, /login… y \
         también los skills que tengas instalados. Tab completa.",
    ),
    (
        "Imágenes y archivos",
        "^V pega una imagen del portapapeles, o arrastra un archivo a la terminal. Cuando te \
         entrego un archivo aparece como etiqueta: un clic lo copia para pegarlo donde quieras.",
    ),
    (
        "Qué estoy haciendo",
        "^G muestra cada herramienta que usé, entera, y Tab cambia a los subagentes, paso a paso. \
         ^T despliega la conversación completa y ^O abre la última respuesta para leerla.",
    ),
    (
        "Sesiones",
        "/clear empieza una conversación nueva y /resume retoma cualquiera de las anteriores. \
         Si un permiso te pregunta algo, «Siempre» no vuelve a preguntar en esta sesión.",
    ),
    (
        "Control",
        "Esc me interrumpe o me calla. ^C dos veces sale. Y /welcome repite todo esto cuando \
         quieras cambiar algo.",
    ),
];

/// The voices: (mode, id, name, what it's for).
pub const VOICES: [(VoiceMode, &str, &str, &str); 3] = [
    (VoiceMode::Auto, "auto", "Si me hablas", "contesto en voz alta cuando me hablaste; si escribes, en silencio"),
    (VoiceMode::Always, "siempre", "Siempre", "todas las respuestas también en voz alta"),
    (VoiceMode::Never, "nunca", "Nunca", "solo texto"),
];

pub struct Welcome {
    pub steps: Vec<Step>,
    pub i: usize,
    pub sel: usize,
    /// What was there before the preview step, to go back to if you leave with Esc.
    style: style::UiStyle,
    buddy: buddy::Buddy,
    /// Something changed that the engine only reads at startup (permissions, model, effort).
    pub relaunch: bool,
}

/// What the main loop has to do after a key.
pub enum Action {
    Nothing,
    /// Log in using the whole terminal; the welcome continues afterwards.
    Login,
    /// Finished: it closes and, if needed, the engine is relaunched.
    Finish { relaunch: bool },
}

impl Welcome {
    pub fn new(app: &App) -> Self {
        let mut steps = vec![Step::Hello];
        if app.no_session {
            steps.push(Step::Session);
        }
        steps.extend([Step::Permissions, Step::UiStyle]);
        if !theme::has_omarchy() {
            steps.push(Step::Accent);
        }
        steps.extend([Step::Buddy, Step::Voice, Step::Model, Step::Effort]);
        steps.extend((0..TOUR.len()).map(Step::Tour));
        Welcome { steps, i: 0, sel: 0, style: app.style, buddy: app.buddy, relaunch: false }
    }

    pub fn step(&self) -> Step {
        self.steps[self.i]
    }

    /// (title, text, options (name, what it's for)) of the current step.
    pub fn screen(&self, app: &App) -> (String, String, Vec<(String, String)>) {
        let o = |n: &str, d: &str| (n.to_string(), d.to_string());
        match self.step() {
            Step::Hello => (
                "Hola, soy Iris".into(),
                "Te dejo lista en un minuto: cuánto hago sin preguntarte, cómo me veo y cómo te \
                 contesto. Todo se cambia después con /welcome."
                    .into(),
                vec![o("Empezar", ""), o("Saltar", "quedarme con lo de fábrica")],
            ),
            Step::Session => (
                "Tu cuenta de Claude".into(),
                "Trabajo con tu cuenta de Claude y todavía no has iniciado sesión.".into(),
                vec![o("Iniciar sesión ahora", "se abre el navegador"), o("Después", "con /login cuando quieras")],
            ),
            Step::Permissions => (
                "¿Cuánto hago sin preguntarte?".into(),
                "Leer siempre lo hago sin preguntar. Lo que cambia es escribir archivos y correr comandos.".into(),
                settings::PERMISSIONS.iter().map(|p| o(p.2, p.3)).collect(),
            ),
            Step::UiStyle => (
                "¿Cómo me veo?".into(),
                "Muévete por la lista y mira el cambio detrás.".into(),
                style::ALL.iter().map(|e| o(e.2, e.3)).collect(),
            ),
            Step::Accent => (
                "¿De qué color?".into(),
                "Con Omarchy sigo los colores de tu tema; aquí elige uno.".into(),
                theme::ACCENTS.iter().map(|a| o(a.1, "")).collect(),
            ),
            Step::Buddy => (
                "¿Quién vive en el núcleo?".into(),
                "El núcleo es mi cara: late cuando escucho, piensa y hablo.".into(),
                buddy::ALL.iter().map(|b| o(b.2, b.3)).collect(),
            ),
            Step::Voice => {
                let mut text = String::from("Escucharte siempre te escucho con espacio; esto es cuándo te contesto hablando.");
                if app.voice == crate::app::VoiceState::Off {
                    text.push_str(
                        " Ojo: no encuentro los modelos de voz; el instalador los baja \
                         (curl -fsSL https://iris.knarvaez.com/install.sh | sh).",
                    );
                }
                ("¿Cuándo te contesto hablando?".into(), text, VOICES.iter().map(|v| o(v.2, v.3)).collect())
            }
            Step::Model => {
                let mut ops = vec![o("El de tu cuenta", "el que Claude Code use por defecto")];
                ops.extend(commands::MODELS.iter().map(|m| o(m.1, m.2)));
                ("¿Con qué modelo trabajo?".into(), "Se cambia en cualquier momento con /model.".into(), ops)
            }
            Step::Effort => (
                "¿Cuánto pienso antes de contestar?".into(),
                "Más es más lento y más caro, pero mejor en lo difícil. También con /effort.".into(),
                commands::EFFORTS.iter().map(|e| o(e.0, e.1)).collect(),
            ),
            Step::Tour(n) => {
                let last = n + 1 == TOUR.len();
                let button = if last { o("Listo", "a trabajar") } else { o("Siguiente", "") };
                (TOUR[n].0.into(), TOUR[n].1.into(), vec![button])
            }
        }
    }

    /// Where the list starts when reaching a step: on what's currently chosen.
    fn initial_sel(&self, app: &App) -> usize {
        match self.step() {
            Step::Permissions => settings::PERMISSIONS.iter().position(|p| p.0 == Permissions::load()).unwrap_or(0),
            Step::UiStyle => style::ALL.iter().position(|e| e.0 == app.style).unwrap_or(0),
            Step::Accent => {
                let a = theme::accent_u32();
                theme::ACCENTS.iter().position(|x| x.2 == a).unwrap_or(0)
            }
            Step::Buddy => buddy::ALL.iter().position(|b| b.0 == app.buddy).unwrap_or(0),
            Step::Voice => VOICES.iter().position(|v| v.0 == app.voice_mode).unwrap_or(0),
            Step::Model => {
                let m = settings::read("modelo");
                m.and_then(|m| commands::MODELS.iter().position(|x| x.0 == m)).map_or(0, |i| i + 1)
            }
            Step::Effort => {
                let e = settings::read("esfuerzo").unwrap_or_else(|| "auto".into());
                commands::EFFORTS.iter().position(|x| x.0 == e).unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn go_to(&mut self, app: &mut App, i: usize) {
        self.i = i;
        self.style = app.style;
        self.buddy = app.buddy;
        self.sel = self.initial_sel(app);
    }

    /// What's visible changes while choosing.
    fn preview(&self, app: &mut App) {
        match self.step() {
            Step::UiStyle => app.style = style::ALL[self.sel].0,
            Step::Buddy => app.buddy = buddy::ALL[self.sel].0,
            Step::Accent => theme::preview_accent(theme::ACCENTS[self.sel].2),
            _ => {}
        }
    }

    /// Undoes the current step's preview (when going back or leaving without choosing).
    fn undo(&self, app: &mut App) {
        match self.step() {
            Step::UiStyle => app.style = self.style,
            Step::Buddy => app.buddy = self.buddy,
            Step::Accent => theme::reload(),
            _ => {}
        }
    }

    pub fn move_sel(&mut self, app: &mut App, down: bool) -> Action {
        let n = self.screen(app).2.len().max(1);
        self.sel = if down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
        self.preview(app);
        Action::Nothing
    }

    pub fn choose_number(&mut self, app: &mut App, k: usize) -> Action {
        if k < self.screen(app).2.len() {
            self.sel = k;
            self.preview(app);
        }
        Action::Nothing
    }

    pub fn back(&mut self, app: &mut App) -> Action {
        if self.i > 0 {
            self.undo(app);
            let i = self.i - 1;
            self.go_to(app, i);
        }
        Action::Nothing
    }

    /// Esc: leaves with what's been chosen so far.
    pub fn skip(&mut self, app: &mut App) -> Action {
        self.undo(app);
        settings::welcome_done();
        Action::Finish { relaunch: self.relaunch }
    }

    /// Enter: saves the choice and moves to the next one.
    pub fn choose(&mut self, app: &mut App) -> Action {
        let sel = self.sel;
        let mut action = Action::Nothing;
        match self.step() {
            Step::Hello if sel == 1 => return self.skip(app),
            Step::Session if sel == 0 => action = Action::Login,
            Step::Permissions => {
                let p = settings::PERMISSIONS[sel].0;
                if p != Permissions::load() {
                    self.relaunch = true;
                }
                let _ = settings::save("permisos", p.id());
            }
            Step::UiStyle => crate::actions::set_style(app, app.style),
            Step::Accent => {
                let _ = settings::save("acento", &format!("#{:06x}", theme::ACCENTS[sel].2));
            }
            Step::Buddy => crate::actions::set_buddy(app, app.buddy),
            Step::Voice => {
                app.voice_mode = VOICES[sel].0;
                let _ = settings::save("voz", VOICES[sel].1);
            }
            Step::Model => {
                let new_value = if sel == 0 { String::new() } else { commands::MODELS[sel - 1].0.to_string() };
                if settings::read("modelo").unwrap_or_default() != new_value {
                    self.relaunch = true;
                }
                let _ = settings::save("modelo", &new_value);
            }
            Step::Effort => {
                let new_value = commands::EFFORTS[sel].0;
                if settings::read("esfuerzo").as_deref().unwrap_or("auto") != new_value {
                    self.relaunch = true;
                }
                let _ = settings::save("esfuerzo", new_value);
            }
            _ => {}
        }
        if self.i + 1 >= self.steps.len() {
            settings::welcome_done();
            return Action::Finish { relaunch: self.relaunch };
        }
        let i = self.i + 1;
        self.go_to(app, i);
        action
    }
}

/// The voice saved in the welcome (`IRIS_HABLA` overrides it).
pub fn saved_voice() -> Option<VoiceMode> {
    let v = settings::read("voz")?;
    VOICES.iter().find(|x| x.1 == v).map(|x| x.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_cards() {
        // They must fit in the panel without scrolling in a normal terminal.
        for (t, x) in TOUR {
            assert!(t.chars().count() < 30);
            assert!(x.chars().count() < 260, "{t}: {}", x.chars().count());
        }
    }
}
