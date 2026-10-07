//! Mandar lo escrito a Claude, y pegar texto o imágenes en la orden.

use crate::*;

pub(crate) fn submit(app: &mut App, claude: &mut Option<Claude>, text: String) {
    if text.is_empty() && app.images.is_empty() {
        return;
    }
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    let queued = app.busy;
    app.read_override = None;
    // Contesta hablando si le hablaste (o si la voz está en «siempre»).
    app.habla.stop();
    app.speak_turn = match app.voz_modo {
        VozModo::Siempre => true,
        VozModo::Nunca => false,
        VozModo::Auto => std::mem::take(&mut app.spoken_next),
    };
    app.spoken_next = false;
    app.lector = habla::Lector::new();
    if text.trim() == "/compact" || text.starts_with("/compact ") {
        app.compacting = true;
    }
    let images = std::mem::take(&mut app.images);
    let shown = match images.len() {
        0 => text.clone(),
        n => {
            let list: Vec<String> = images.iter().enumerate().map(|(i, im)| im.label(i + 1)).collect();
            let head = format!("▣ {n} imagen{} ({})", if n == 1 { "" } else { "es" }, list.join(", "));
            if text.is_empty() { head } else { format!("{head}\n{text}") }
        }
    };
    app.push(Role::User, shown);
    app.messages.last_mut().unwrap().images = images.iter().filter_map(|i| miniatura::from_bytes(&i.raw)).collect();
    match c.send(&text, &images) {
        Ok(uuid) => {
            // Con el motor libre lo toma al instante; solo a mitad de turno queda esperando.
            if queued {
                app.messages.last_mut().unwrap().waiting = Some(uuid);
            } else {
                app.turn_from = app.activity.len();
            }
            app.busy = true;
            app.thinking = true;
            app.interrupted = false;
        }
        Err(e) => app.push(Role::Error, format!("no pude enviar: {e}")),
    }
}

/// Ctrl+V: una imagen del portapapeles se adjunta; texto, se pega en la entrada.
pub(crate) fn paste(app: &mut App) {
    match clip::paste() {
        Ok(clip::Clip::Image(img)) => attach(app, img),
        Ok(clip::Clip::Text(t)) => pasted(app, &t),
        Ok(clip::Clip::Nothing) => app.flash = Some(("el portapapeles está vacío".into(), Instant::now())),
        Err(e) => app.push(Role::Error, format!("no pude pegar: {e}")),
    }
}

/// Texto pegado (o archivos arrastrados a la terminal, que llegan como rutas).
pub(crate) fn pasted(app: &mut App, text: &str) {
    if let Some(paths) = clip::paths(text) {
        for p in paths {
            match clip::read(&p) {
                Ok(img) => attach(app, img),
                Err(e) => app.push(Role::Error, format!("no pude adjuntar {p}: {e}")),
            }
        }
        return;
    }
    entrada::insert(&mut app.input, &mut app.cur, &text.replace("\r\n", "\n").replace('\r', "\n"));
    app.menu = 0;
    app.last_key = Instant::now();
}

pub(crate) fn attach(app: &mut App, img: clip::Image) {
    let label = img.label(app.images.len() + 1);
    app.images.push(img);
    app.flash = Some((format!("{label} adjunta · se va con el próximo mensaje"), Instant::now()));
}
