//! Chat links: a click on a URL opens it in the browser. With the mouse in Iris's hands the
//! terminal no longer opens anything by itself, so we do it.
//!
//! There's no need to know where each link ended up on screen: the piece of text under the
//! click (plus those of the neighboring rows, because a URL longer than the line gets split
//! across several) is looked up among the conversation's URLs, which are whole in the messages' text.

use std::process::{Command, Stdio};

use crate::select::View;

/// The http(s) URLs in a text, whole and without repeats. Also those of markdown links
/// (`[text](url)`) and those in `<url>`.
pub fn urls(text: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut rest = text;
    while let Some(i) = rest.find("http") {
        let tail = &rest[i..];
        if !(tail.starts_with("https://") || tail.starts_with("http://")) {
            rest = &tail[4..];
            continue;
        }
        let end = tail.find(|c: char| c.is_whitespace() || "<>\"'`)]".contains(c)).unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(|c: char| ".,;:!?*_".contains(c));
        if url.len() > "https://".len() && !out.iter().any(|u| u == url) {
            out.push(url.to_string());
        }
        rest = &tail[end.max(1)..];
    }
    out
}

/// The URL under (row, column) of the view, if any, looked up among `urls`.
pub fn under(view: &View, (r, c): (usize, usize), urls: &[String]) -> Option<String> {
    let row = view.rows.get(r)?;
    let (tok, start, ends) = word_at(&row.text, c.max(row.skip))?;
    // The pieces that can make up the URL: the row's and, if the URL was split, those of the
    // rows before and after (with no space in between, as the wrap left them). It keeps going
    // while the piece fills the whole row: that way it crosses a URL split into three or more.
    let mut chunk = tok.clone();
    let (mut i, mut touches) = (r, start);
    while touches && view.rows[i].cont && i > 0 {
        let p = &view.rows[i - 1];
        let Some(prev) = last(&p.text, p.skip) else { break };
        touches = standalone(&p.text, p.skip);
        chunk = prev + &chunk;
        i -= 1;
    }
    let (mut j, mut touches) = (r, ends);
    while touches {
        let Some(n) = view.rows.get(j + 1).filter(|n| n.cont) else { break };
        let Some(next) = first(&n.text, n.skip) else { break };
        touches = standalone(&n.text, n.skip);
        chunk.push_str(&next);
        j += 1;
    }
    const BORDER: &str = "([<«\"'`)]>».,;:!?*_";
    // What's stuck in front of the URL isn't part of it: the `text](` of a raw markdown
    // link (the Cine subtitle shows it like that), a `(` or a `<`.
    let from_url = |s: &str| s.find("http").map_or(s.to_string(), |i| s[i..].to_string());
    let clean = from_url(&chunk);
    let clean = clean.trim_matches(|c: char| BORDER.contains(c));
    let tok = from_url(&tok);
    let tok = tok.trim_matches(|c: char| BORDER.contains(c));
    // A standalone word that also appears inside a URL («tienda») isn't the link: what was
    // clicked has to be part of a written URL, with its scheme.
    if tok.is_empty() || !clean.contains("://") {
        return None;
    }
    // The clicked piece has to be in the URL; among those that have it, the one that matches
    // the rebuilt piece (what was before the URL or after it, like `)`, has already been cleaned).
    urls.iter()
        .filter(|u| u.contains(tok))
        .find(|u| u.contains(clean) || clean.contains(u.as_str()))
        .cloned()
}

/// If the row has a single word (no spaces after the decoration): a piece of a word longer
/// than the line, cut with no space.
fn standalone(text: &str, skip: usize) -> bool {
    let t: String = text.chars().skip(skip).collect();
    !t.trim().is_empty() && !t.trim().contains(char::is_whitespace)
}

/// The word (no spaces) at column `col`, and whether it touches the start and the end of the
/// row's text.
fn word_at(text: &str, col: usize) -> Option<(String, bool, bool)> {
    use unicode_width::UnicodeWidthChar;
    let chars: Vec<char> = text.chars().collect();
    let mut x = 0;
    let mut at = None;
    for (i, ch) in chars.iter().enumerate() {
        let w = ch.width().unwrap_or(0);
        if col < x + w.max(1) {
            at = Some(i);
            break;
        }
        x += w;
    }
    let i = at?;
    if chars[i].is_whitespace() {
        return None;
    }
    let mut a = i;
    while a > 0 && !chars[a - 1].is_whitespace() {
        a -= 1;
    }
    let mut b = i;
    while b + 1 < chars.len() && !chars[b + 1].is_whitespace() {
        b += 1;
    }
    let before = chars[..a].iter().all(|c| c.is_whitespace());
    let after = chars[b + 1..].iter().all(|c| c.is_whitespace());
    Some((chars[a..=b].iter().collect(), before, after))
}

fn last(text: &str, skip: usize) -> Option<String> {
    let t: String = text.chars().skip(skip).collect();
    t.trim_end().rsplit(char::is_whitespace).next().filter(|s| !s.is_empty()).map(str::to_string)
}

fn first(text: &str, skip: usize) -> Option<String> {
    let t: String = text.chars().skip(skip).collect();
    t.split_whitespace().next().map(str::to_string)
}

/// Opens the URL in the default browser. Only http(s): no `file://` or other schemes.
pub fn open(url: &str) -> bool {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return false;
    }
    #[cfg(windows)]
    let mut cmd = {
        // `start` goes through cmd, which eats the URL's `&`; this one doesn't.
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok()
}

/// The domain, for the notice: «abriendo claude.ai».
pub fn domain(url: &str) -> &str {
    let sin = url.split("://").nth(1).unwrap_or(url);
    sin.split(['/', '?', '#']).next().unwrap_or(sin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::Row;
    use ratatui::layout::Rect;

    fn view(rows: &[(&str, usize, bool)]) -> View {
        View {
            area: Rect::new(0, 0, 40, rows.len() as u16),
            rows: rows.iter().map(|(t, s, c)| Row { text: t.to_string(), skip: *s, cont: *c }).collect(),
            first: 0,
        }
    }

    #[test]
    fn extracts_markdown_and_bare_urls() {
        let u = urls("Mira [el doc](https://claude.ai/code/artifact/abc-123). Y <https://a.b/c?x=1&y=2>, o http://x.y/z.");
        assert_eq!(u, vec!["https://claude.ai/code/artifact/abc-123", "https://a.b/c?x=1&y=2", "http://x.y/z"]);
    }

    #[test]
    fn click_on_a_whole_url() {
        let u = urls("está en https://iris.knarvaez.com/tienda hoy");
        let v = view(&[("  está en https://iris.knarvaez.com/tienda hoy", 2, false)]);
        assert_eq!(under(&v, (0, 15), &u).as_deref(), Some("https://iris.knarvaez.com/tienda"));
        assert_eq!(under(&v, (0, 4), &u), None);
    }

    #[test]
    fn click_on_any_piece_of_a_wrapped_url() {
        let url = "https://claude.ai/code/artifact/dfa3e55a-dd1c-4525-8cbf-84ebaa24e0cb";
        let u = urls(&format!("El documento está aquí: [Plan]({url})"));
        let v = view(&[
            ("  El documento está aquí: Plan", 2, false),
            ("  (https://claude.ai/code/artifact/dfa3e5", 2, true),
            ("  5a-dd1c-4525-8cbf-84ebaa24e0cb)", 2, true),
        ]);
        assert_eq!(under(&v, (1, 10), &u).as_deref(), Some(url));
        assert_eq!(under(&v, (2, 6), &u).as_deref(), Some(url));
    }

    #[test]
    fn raw_markdown_like_the_cine_subtitle() {
        let url = "https://claude.ai/code/artifact/dfa3e55a-dd1c-4525-8cbf-84ebaa24e0cb";
        let u = urls(&format!("El documento está aquí: [Plan técnico]({url})"));
        let v = view(&[
            ("      El documento está aquí: [Plan técnico](https://claude.ai/code/", 6, false),
            ("             artifact/dfa3e55a-dd1c-4525-8cbf-84ebaa24e0cb)", 13, true),
        ]);
        assert_eq!(under(&v, (0, 50), &u).as_deref(), Some(url));
        assert_eq!(under(&v, (0, 40), &u).as_deref(), Some(url));
        assert_eq!(under(&v, (1, 20), &u).as_deref(), Some(url));
        assert_eq!(under(&v, (0, 9), &u), None);
    }

    #[test]
    fn url_split_across_three_rows() {
        let url = "https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbbbbbb/cccc";
        let u = urls(url);
        let v = view(&[("  https://example.com/aaaaaaaaaaaaaaaaaa", 2, false), ("  aa/bbbbbbbbbbbbbb", 2, true), ("  bbbbbb/cccc y sigue", 2, true)]);
        assert_eq!(under(&v, (2, 4), &u).as_deref(), Some(url));
        assert_eq!(under(&v, (1, 4), &u).as_deref(), Some(url));
    }

    #[test]
    fn word_also_in_a_url_opens_nothing() {
        let u = urls("ver https://iris.knarvaez.com/tienda");
        let v = view(&[("  la tienda abre pronto, ver", 2, false), ("  https://iris.knarvaez.com/tienda", 2, true)]);
        assert_eq!(under(&v, (0, 5), &u), None);
        // The word after a split URL doesn't either.
        let v = view(&[("  ver https://iris.knarvaez.com/tienda", 2, false), ("  sigue aquí", 2, true)]);
        assert_eq!(under(&v, (1, 3), &u), None);
    }

    #[test]
    fn domain_for_the_notice() {
        assert_eq!(domain("https://claude.ai/code/x?y=1"), "claude.ai");
    }
}
