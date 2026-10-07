//! Texto y hora: cortar en líneas, recortar, y la hora y la fecha locales sin depender de nada.

/// La hora local de hace `ago` segundos, `hh:mm:ss`.
pub(in crate::ui) fn clock_ago(ago: u64) -> String {
    let (h, m, s) = local_hms();
    let now = h * 3600 + m * 60 + s;
    let then = (now + 86_400 - ago % 86_400) % 86_400;
    format!("{:02}:{:02}:{:02}", then / 3600, then / 60 % 60, then % 60)
}

/// Como `wrap`, marcando las piezas que continúan la línea anterior del texto original.
pub(in crate::ui) fn wrap_cont(s: &str, width: usize) -> Vec<(String, bool)> {
    s.lines()
        .flat_map(|l| wrap(l, width).into_iter().enumerate().map(|(i, p)| (p, i > 0)))
        .collect()
}

pub(in crate::ui) fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            out.push(String::new());
        } else {
            out.extend(textwrap::wrap(line, width.max(1)).into_iter().map(|c| c.into_owned()));
        }
    }
    out
}

pub(in crate::ui) fn truncate(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut w = 0;
    let mut out = String::new();
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw > max {
            if max > 0 {
                out.pop();
                out.push('…');
            }
            break;
        }
        w += cw;
        out.push(c);
    }
    out
}

pub(in crate::ui) fn local_hms() -> (u64, u64, u64) {
    // Hora local sin dependencias: se toma el desfase de `date +%z` una vez.
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i64> = OnceLock::new();
    let off = *OFFSET.get_or_init(|| {
        std::process::Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                let s = s.trim();
                let sign = if s.starts_with('-') { -1 } else { 1 };
                let n: i64 = s.get(1..)?.parse().ok()?;
                Some(sign * ((n / 100) * 3600 + (n % 100) * 60))
            })
            .unwrap_or(0)
    });
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + off;
    let s = now.rem_euclid(86_400) as u64;
    (s / 3600, s / 60 % 60, s % 60)
}

pub(in crate::ui) fn now_hhmmss() -> String {
    let (h, m, s) = local_hms();
    format!("{h:02}:{m:02}:{s:02}")
}

pub(in crate::ui) fn greeting() -> &'static str {
    match local_hms().0 {
        5..=11 => "Buenos días",
        12..=18 => "Buenas tardes",
        _ => "Buenas noches",
    }
}

/// Fecha local: (año, mes, día, día de la semana con 0 = domingo).
pub(in crate::ui) fn local_date() -> (i64, u32, u32, u32) {
    let utc = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // El desfase sale de comparar la hora local con la UTC del día.
    let (h, m, s) = local_hms();
    let local_sod = (h * 3600 + m * 60 + s) as i64;
    let mut off = local_sod - utc.rem_euclid(86_400);
    if off > 43_200 {
        off -= 86_400;
    } else if off < -43_200 {
        off += 86_400;
    }
    let days = (utc + off).div_euclid(86_400);
    let wd = (days + 4).rem_euclid(7) as u32;
    // Días desde 1970 → fecha civil (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + (mo <= 2) as i64;
    (y, mo, d, wd)
}
