//! La bienvenida: la primera vez que se abre Iris (o con `/welcome`) se eligen permisos,
//! estilo, color, buddy, voz, modelo y esfuerzo, y al final unas tarjetas con lo que se puede
//! hacer. Cada paso se guarda al elegirlo; lo que se ve (estilo, color, buddy) cambia
//! mientras uno se mueve por la lista.

use crate::ajustes::{self, Permisos};
use crate::app::{App, VozModo};
use crate::{buddy, commands, estilo, theme};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Paso {
    Hola,
    Sesion,
    Permisos,
    Estilo,
    Acento,
    Buddy,
    Voz,
    Modelo,
    Esfuerzo,
    Tour(usize),
}

/// (título, texto) de cada tarjeta del recorrido.
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

/// Las voces: (modo, id, nombre, para qué).
pub const VOCES: [(VozModo, &str, &str, &str); 3] = [
    (VozModo::Auto, "auto", "Si me hablas", "contesto en voz alta cuando me hablaste; si escribes, en silencio"),
    (VozModo::Siempre, "siempre", "Siempre", "todas las respuestas también en voz alta"),
    (VozModo::Nunca, "nunca", "Nunca", "solo texto"),
];

pub struct Bienvenida {
    pub pasos: Vec<Paso>,
    pub i: usize,
    pub sel: usize,
    /// Lo que había antes del paso con vista previa, para volver si se sale con Esc.
    estilo: estilo::Estilo,
    buddy: buddy::Buddy,
    /// Cambió algo que el motor solo lee al arrancar (permisos, modelo, esfuerzo).
    pub relanzar: bool,
}

/// Lo que tiene que hacer el bucle principal después de una tecla.
pub enum Accion {
    Nada,
    /// Iniciar sesión con la terminal entera; la bienvenida sigue después.
    Login,
    /// Terminó: se cierra y, si hace falta, se relanza el motor.
    Fin { relanzar: bool },
}

impl Bienvenida {
    pub fn new(app: &App) -> Self {
        let mut pasos = vec![Paso::Hola];
        if app.sin_sesion {
            pasos.push(Paso::Sesion);
        }
        pasos.extend([Paso::Permisos, Paso::Estilo]);
        if !theme::hay_omarchy() {
            pasos.push(Paso::Acento);
        }
        pasos.extend([Paso::Buddy, Paso::Voz, Paso::Modelo, Paso::Esfuerzo]);
        pasos.extend((0..TOUR.len()).map(Paso::Tour));
        Bienvenida { pasos, i: 0, sel: 0, estilo: app.estilo, buddy: app.buddy, relanzar: false }
    }

    pub fn paso(&self) -> Paso {
        self.pasos[self.i]
    }

    /// (título, texto, opciones (nombre, para qué)) del paso en curso.
    pub fn pantalla(&self, app: &App) -> (String, String, Vec<(String, String)>) {
        let o = |n: &str, d: &str| (n.to_string(), d.to_string());
        match self.paso() {
            Paso::Hola => (
                "Hola, soy Iris".into(),
                "Te dejo lista en un minuto: cuánto hago sin preguntarte, cómo me veo y cómo te \
                 contesto. Todo se cambia después con /welcome."
                    .into(),
                vec![o("Empezar", ""), o("Saltar", "quedarme con lo de fábrica")],
            ),
            Paso::Sesion => (
                "Tu cuenta de Claude".into(),
                "Trabajo con tu cuenta de Claude y todavía no has iniciado sesión.".into(),
                vec![o("Iniciar sesión ahora", "se abre el navegador"), o("Después", "con /login cuando quieras")],
            ),
            Paso::Permisos => (
                "¿Cuánto hago sin preguntarte?".into(),
                "Leer siempre lo hago sin preguntar. Lo que cambia es escribir archivos y correr comandos.".into(),
                ajustes::PERMISOS.iter().map(|p| o(p.2, p.3)).collect(),
            ),
            Paso::Estilo => (
                "¿Cómo me veo?".into(),
                "Muévete por la lista y mira el cambio detrás.".into(),
                estilo::TODOS.iter().map(|e| o(e.2, e.3)).collect(),
            ),
            Paso::Acento => (
                "¿De qué color?".into(),
                "Con Omarchy sigo los colores de tu tema; aquí elige uno.".into(),
                theme::ACENTOS.iter().map(|a| o(a.1, "")).collect(),
            ),
            Paso::Buddy => (
                "¿Quién vive en el núcleo?".into(),
                "El núcleo es mi cara: late cuando escucho, piensa y hablo.".into(),
                buddy::TODOS.iter().map(|b| o(b.2, b.3)).collect(),
            ),
            Paso::Voz => {
                let mut texto = String::from("Escucharte siempre te escucho con espacio; esto es cuándo te contesto hablando.");
                if app.voice == crate::app::VoiceState::Off {
                    texto.push_str(
                        " Ojo: no encuentro los modelos de voz; el instalador los baja \
                         (curl -fsSL https://iris.knarvaez.com/install.sh | sh).",
                    );
                }
                ("¿Cuándo te contesto hablando?".into(), texto, VOCES.iter().map(|v| o(v.2, v.3)).collect())
            }
            Paso::Modelo => {
                let mut ops = vec![o("El de tu cuenta", "el que Claude Code use por defecto")];
                ops.extend(commands::MODELS.iter().map(|m| o(m.1, m.2)));
                ("¿Con qué modelo trabajo?".into(), "Se cambia en cualquier momento con /model.".into(), ops)
            }
            Paso::Esfuerzo => (
                "¿Cuánto pienso antes de contestar?".into(),
                "Más es más lento y más caro, pero mejor en lo difícil. También con /effort.".into(),
                commands::EFFORTS.iter().map(|e| o(e.0, e.1)).collect(),
            ),
            Paso::Tour(n) => {
                let ultima = n + 1 == TOUR.len();
                let boton = if ultima { o("Listo", "a trabajar") } else { o("Siguiente", "") };
                (TOUR[n].0.into(), TOUR[n].1.into(), vec![boton])
            }
        }
    }

    /// Dónde empieza la lista al llegar a un paso: en lo que está elegido ahora.
    fn sel_inicial(&self, app: &App) -> usize {
        match self.paso() {
            Paso::Permisos => ajustes::PERMISOS.iter().position(|p| p.0 == Permisos::cargar()).unwrap_or(0),
            Paso::Estilo => estilo::TODOS.iter().position(|e| e.0 == app.estilo).unwrap_or(0),
            Paso::Acento => {
                let a = theme::acento_u32();
                theme::ACENTOS.iter().position(|x| x.2 == a).unwrap_or(0)
            }
            Paso::Buddy => buddy::TODOS.iter().position(|b| b.0 == app.buddy).unwrap_or(0),
            Paso::Voz => VOCES.iter().position(|v| v.0 == app.voz_modo).unwrap_or(0),
            Paso::Modelo => {
                let m = ajustes::leer("modelo");
                m.and_then(|m| commands::MODELS.iter().position(|x| x.0 == m)).map_or(0, |i| i + 1)
            }
            Paso::Esfuerzo => {
                let e = ajustes::leer("esfuerzo").unwrap_or_else(|| "auto".into());
                commands::EFFORTS.iter().position(|x| x.0 == e).unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn ir(&mut self, app: &mut App, i: usize) {
        self.i = i;
        self.estilo = app.estilo;
        self.buddy = app.buddy;
        self.sel = self.sel_inicial(app);
    }

    /// Lo que se ve cambia mientras se elige.
    fn vista_previa(&self, app: &mut App) {
        match self.paso() {
            Paso::Estilo => app.estilo = estilo::TODOS[self.sel].0,
            Paso::Buddy => app.buddy = buddy::TODOS[self.sel].0,
            Paso::Acento => theme::probar_acento(theme::ACENTOS[self.sel].2),
            _ => {}
        }
    }

    /// Deshace la vista previa del paso en curso (al volver atrás o salir sin elegir).
    fn deshacer(&self, app: &mut App) {
        match self.paso() {
            Paso::Estilo => app.estilo = self.estilo,
            Paso::Buddy => app.buddy = self.buddy,
            Paso::Acento => theme::recargar(),
            _ => {}
        }
    }

    pub fn mover(&mut self, app: &mut App, abajo: bool) -> Accion {
        let n = self.pantalla(app).2.len().max(1);
        self.sel = if abajo { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
        self.vista_previa(app);
        Accion::Nada
    }

    pub fn elegir_numero(&mut self, app: &mut App, k: usize) -> Accion {
        if k < self.pantalla(app).2.len() {
            self.sel = k;
            self.vista_previa(app);
        }
        Accion::Nada
    }

    pub fn atras(&mut self, app: &mut App) -> Accion {
        if self.i > 0 {
            self.deshacer(app);
            let i = self.i - 1;
            self.ir(app, i);
        }
        Accion::Nada
    }

    /// Esc: se sale con lo elegido hasta ahora.
    pub fn saltar(&mut self, app: &mut App) -> Accion {
        self.deshacer(app);
        ajustes::bienvenida_hecha();
        Accion::Fin { relanzar: self.relanzar }
    }

    /// Enter: guarda lo elegido y pasa al siguiente.
    pub fn elegir(&mut self, app: &mut App) -> Accion {
        let sel = self.sel;
        let mut accion = Accion::Nada;
        match self.paso() {
            Paso::Hola if sel == 1 => return self.saltar(app),
            Paso::Sesion if sel == 0 => accion = Accion::Login,
            Paso::Permisos => {
                let p = ajustes::PERMISOS[sel].0;
                if p != Permisos::cargar() {
                    self.relanzar = true;
                }
                let _ = ajustes::guardar("permisos", p.id());
            }
            Paso::Estilo => crate::ordenes::set_estilo(app, app.estilo),
            Paso::Acento => {
                let _ = ajustes::guardar("acento", &format!("#{:06x}", theme::ACENTOS[sel].2));
            }
            Paso::Buddy => crate::ordenes::set_buddy(app, app.buddy),
            Paso::Voz => {
                app.voz_modo = VOCES[sel].0;
                let _ = ajustes::guardar("voz", VOCES[sel].1);
            }
            Paso::Modelo => {
                let nuevo = if sel == 0 { String::new() } else { commands::MODELS[sel - 1].0.to_string() };
                if ajustes::leer("modelo").unwrap_or_default() != nuevo {
                    self.relanzar = true;
                }
                let _ = ajustes::guardar("modelo", &nuevo);
            }
            Paso::Esfuerzo => {
                let nuevo = commands::EFFORTS[sel].0;
                if ajustes::leer("esfuerzo").as_deref().unwrap_or("auto") != nuevo {
                    self.relanzar = true;
                }
                let _ = ajustes::guardar("esfuerzo", nuevo);
            }
            _ => {}
        }
        if self.i + 1 >= self.pasos.len() {
            ajustes::bienvenida_hecha();
            return Accion::Fin { relanzar: self.relanzar };
        }
        let i = self.i + 1;
        self.ir(app, i);
        accion
    }
}

/// La voz guardada en la bienvenida (`IRIS_HABLA` manda sobre ella).
pub fn voz_guardada() -> Option<VozModo> {
    let v = ajustes::leer("voz")?;
    VOCES.iter().find(|x| x.1 == v).map(|x| x.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarjetas_cortas() {
        // Que quepan en el panel sin desplazarse en una terminal normal.
        for (t, x) in TOUR {
            assert!(t.chars().count() < 30);
            assert!(x.chars().count() < 260, "{t}: {}", x.chars().count());
        }
    }
}
