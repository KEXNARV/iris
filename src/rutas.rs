//! Dónde guarda Iris lo suyo, igual en Linux y en Windows, y las variables de entorno.
//!
//! Linux: `~/.config/iris` (lo elegido), `~/.local/share/iris` (modelos y voz) y
//! `~/.local/state/iris` (historial). Windows: `%APPDATA%\iris` y `%LOCALAPPDATA%\iris`.
//! Antes se llamaba JARVIS: al arrancar se mudan las carpetas viejas y las variables
//! `JARVIS_*` siguen valiendo si no hay una `IRIS_*`.

use std::path::{Path, PathBuf};

const NOMBRE: &str = "iris";
const VIEJO: &str = "jarvis";

/// Lo que se elige y se guarda (estilo, buddy, colores, piezas instaladas).
pub fn config() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(NOMBRE))
}

/// Lo pesado que se baja: los modelos de voz y el entorno de la voz que habla.
pub fn datos() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join(NOMBRE))
}

/// Lo que se va acumulando (el historial de órdenes). Windows no tiene carpeta de estado.
pub fn estado() -> Option<PathBuf> {
    Some(dirs::state_dir().or_else(dirs::data_local_dir)?.join(NOMBRE))
}

/// `IRIS_<nombre>`, o `JARVIS_<nombre>` si solo está la vieja.
pub fn var(nombre: &str) -> Option<String> {
    std::env::var(format!("IRIS_{nombre}")).or_else(|_| std::env::var(format!("JARVIS_{nombre}"))).ok()
}

pub fn hay_var(nombre: &str) -> bool {
    std::env::var_os(format!("IRIS_{nombre}")).or_else(|| std::env::var_os(format!("JARVIS_{nombre}"))).is_some()
}

/// Muda `…/jarvis` a `…/iris` en las tres carpetas, una vez. En la vieja queda un enlace a la
/// nueva: un Iris viejo que siga abierto (o un atajo que la nombre) no se queda sin sus cosas.
pub fn migrar() {
    let bases = [dirs::config_dir(), dirs::data_local_dir(), dirs::state_dir()];
    for base in bases.into_iter().flatten() {
        mudar(&base.join(VIEJO), &base.join(NOMBRE));
    }
}

fn mudar(viejo: &Path, nuevo: &Path) {
    let es_carpeta = std::fs::symlink_metadata(viejo).is_ok_and(|m| m.is_dir());
    if !es_carpeta || nuevo.exists() {
        return;
    }
    if std::fs::rename(viejo, nuevo).is_ok() {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(nuevo, viejo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muda_una_vez_y_deja_enlace() {
        let base = std::env::temp_dir().join(format!("iris-rutas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (viejo, nuevo) = (base.join(VIEJO), base.join(NOMBRE));
        std::fs::create_dir_all(&viejo).unwrap();
        std::fs::write(viejo.join("estilo"), "cine\n").unwrap();
        mudar(&viejo, &nuevo);
        assert_eq!(std::fs::read_to_string(nuevo.join("estilo")).unwrap(), "cine\n");
        #[cfg(unix)]
        assert_eq!(std::fs::read_to_string(viejo.join("estilo")).unwrap(), "cine\n", "lo viejo sigue llegando");
        // Ya mudada, otra vez no hace nada.
        mudar(&viejo, &nuevo);
        assert!(nuevo.join("estilo").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn la_variable_nueva_manda_sobre_la_vieja() {
        // Nombres que nadie más usa: las pruebas corren en paralelo.
        unsafe {
            std::env::set_var("JARVIS_PRUEBA_RUTAS", "vieja");
        }
        assert_eq!(var("PRUEBA_RUTAS").as_deref(), Some("vieja"));
        unsafe {
            std::env::set_var("IRIS_PRUEBA_RUTAS", "nueva");
        }
        assert_eq!(var("PRUEBA_RUTAS").as_deref(), Some("nueva"));
    }
}
