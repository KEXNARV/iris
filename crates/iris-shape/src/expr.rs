//! Formulas: the small language every property of a shape is written in.
//!
//! ```text
//! 0.30 + breath * 0.1 + voice * 0.05
//! if(alarm > 0.5, 3, 1)
//! pulse(ev.error, 0.4) * sin(t * 40) * 0.05
//! ```
//!
//! Numbers, `+ - * / % ^`, comparisons and `&& || !` (true is 1, false 0), parentheses and
//! functions. A name is, in this order: a constant (`pi`, `tau`), something Iris gives every
//! frame (`t`, `dt`, `st`, `state`, `calm`, `voice`, `music`, `context`, `du`), whether the
//! visible state is one (`s.thinking`), seconds since an event (`ev.error`; very large if it
//! never happened), a setting of the piece, or else a channel of the buddy, which the piece then
//! reads (`body.radius`, `gaze.x`…).
//!
//! It's compiled once and evaluated once per property per frame. Same code in Iris, in the
//! browser and on the server, so the three agree on what a formula means.

/// What Iris gives every frame, by position in a [`Frame`]'s `vars`.
pub const VARS: [&str; 9] = ["t", "dt", "st", "state", "calm", "voice", "music", "context", "du"];

/// The states, by index (Iris's order), for `s.<name>`.
pub const STATES: [&str; 21] = [
    "booting", "sleeping", "idle", "typing", "listening", "novoice", "transcribing", "thinking", "planning", "searching", "reading",
    "editing", "running", "testing", "git", "web", "delegating", "speaking", "asking", "compacting", "offline",
];

/// What can happen, for `ev.<name>`: the 8 events, a state change, and the subagents.
pub const EVENTS: [&str; 12] = ["key", "error", "done", "cancel", "nope", "merge", "copy", "pass", "change", "born", "pulse", "finish"];

/// Seconds since something that never happened.
pub const NEVER: f64 = 1.0e6;

#[derive(Clone, Debug)]
enum Node {
    Num(f64),
    Var(usize),
    Is(usize),
    Since(usize),
    Setting(usize),
    Channel(usize),
    Neg(Box<Node>),
    Not(Box<Node>),
    Bin(Op, Box<Node>, Box<Node>),
    Call(Func, Vec<Node>),
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Copy, Debug)]
enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Abs,
    Sign,
    Floor,
    Ceil,
    Round,
    Fract,
    Sqrt,
    Exp,
    Ln,
    Pow,
    Min,
    Max,
    Clamp,
    Mix,
    Smoothstep,
    Step,
    Noise,
    Hash,
    If,
    Pulse,
    Wave,
}

impl Func {
    fn by_name(n: &str) -> Option<(Func, usize, usize)> {
        use Func::*;
        // (function, fewest arguments, most arguments)
        Some(match n {
            "sin" => (Sin, 1, 1),
            "cos" => (Cos, 1, 1),
            "tan" => (Tan, 1, 1),
            "asin" => (Asin, 1, 1),
            "acos" => (Acos, 1, 1),
            "atan" => (Atan, 1, 1),
            "atan2" => (Atan2, 2, 2),
            "abs" => (Abs, 1, 1),
            "sign" => (Sign, 1, 1),
            "floor" => (Floor, 1, 1),
            "ceil" => (Ceil, 1, 1),
            "round" => (Round, 1, 1),
            "fract" => (Fract, 1, 1),
            "sqrt" => (Sqrt, 1, 1),
            "exp" => (Exp, 1, 1),
            "ln" => (Ln, 1, 1),
            "pow" => (Pow, 2, 2),
            "min" => (Min, 2, 8),
            "max" => (Max, 2, 8),
            "clamp" => (Clamp, 3, 3),
            "mix" => (Mix, 3, 3),
            "smoothstep" => (Smoothstep, 3, 3),
            "step" => (Step, 2, 2),
            "noise" => (Noise, 1, 2),
            "hash" => (Hash, 1, 1),
            "if" => (If, 3, 3),
            "pulse" => (Pulse, 2, 2),
            "wave" => (Wave, 1, 2),
            _ => return None,
        })
    }
}

/// The names a formula can use besides Iris's: the piece's settings; and the channels it
/// reads, which grow as formulas name new ones.
pub struct Names<'a> {
    pub settings: &'a [String],
    pub channels: &'a mut Vec<String>,
}

/// A compiled formula.
#[derive(Clone, Debug)]
pub struct Expr {
    root: Node,
    pub src: String,
}

/// What a formula is evaluated against.
pub struct Frame<'a> {
    /// `t, dt, st, state, calm, voice, music, context, du` (see [`VARS`]).
    pub vars: [f64; 9],
    /// Seconds since each of [`EVENTS`].
    pub since: [f64; 12],
    pub settings: &'a [f64],
    pub channels: &'a [f64],
}

impl Expr {
    pub fn constant(v: f64) -> Expr {
        Expr { root: Node::Num(v), src: format!("{v}") }
    }

    pub fn compile(src: &str, names: &mut Names) -> Result<Expr, String> {
        let toks = lex(src)?;
        let mut p = Parser { toks, i: 0, names };
        let root = p.or()?;
        if p.i < p.toks.len() {
            return Err(format!("sobra «{}»", p.toks[p.i].text()));
        }
        Ok(Expr { root, src: src.to_string() })
    }

    pub fn eval(&self, f: &Frame) -> f64 {
        let v = eval(&self.root, f);
        if v.is_finite() { v } else { 0.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Sym(&'static str),
}

impl Tok {
    fn text(&self) -> String {
        match self {
            Tok::Num(n) => n.to_string(),
            Tok::Name(n) => n.clone(),
            Tok::Sym(s) => s.to_string(),
        }
    }
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    const SYMS: [&str; 19] = ["&&", "||", "<=", ">=", "==", "!=", "+", "-", "*", "/", "%", "^", "<", ">", "!", "(", ")", ",", "="];
    let b = src.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') && b.get(i + 1).is_some_and(|d| d.is_ascii_digit() || *d == b'-' || *d == b'+') {
                i += 2;
                while i < b.len() && (b[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            let s = &src[start..i];
            out.push(Tok::Num(s.parse().map_err(|_| format!("«{s}» no es un número"))?));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                i += 1;
            }
            out.push(Tok::Name(src[start..i].trim_end_matches('.').to_string()));
        } else if let Some(s) = SYMS.iter().find(|s| src[i..].starts_with(**s)) {
            if *s == "=" {
                return Err("«=» solo: para comparar se escribe «==»".into());
            }
            out.push(Tok::Sym(s));
            i += s.len();
        } else {
            return Err(format!("no entiendo «{c}»"));
        }
    }
    if out.is_empty() {
        return Err("la fórmula está vacía".into());
    }
    Ok(out)
}

struct Parser<'a, 'b> {
    toks: Vec<Tok>,
    i: usize,
    names: &'a mut Names<'b>,
}

impl Parser<'_, '_> {
    fn peek(&self, s: &str) -> bool {
        matches!(self.toks.get(self.i), Some(Tok::Sym(x)) if *x == s)
    }

    fn eat(&mut self, s: &str) -> bool {
        if self.peek(s) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn bin(&mut self, ops: &[(&str, Op)], next: fn(&mut Self) -> Result<Node, String>) -> Result<Node, String> {
        let mut l = next(self)?;
        'outer: loop {
            for (s, op) in ops {
                if self.eat(s) {
                    let r = next(self)?;
                    l = Node::Bin(*op, Box::new(l), Box::new(r));
                    continue 'outer;
                }
            }
            return Ok(l);
        }
    }

    fn or(&mut self) -> Result<Node, String> {
        self.bin(&[("||", Op::Or)], Self::and)
    }

    fn and(&mut self) -> Result<Node, String> {
        self.bin(&[("&&", Op::And)], Self::cmp)
    }

    fn cmp(&mut self) -> Result<Node, String> {
        self.bin(&[("<=", Op::Le), (">=", Op::Ge), ("==", Op::Eq), ("!=", Op::Ne), ("<", Op::Lt), (">", Op::Gt)], Self::sum)
    }

    fn sum(&mut self) -> Result<Node, String> {
        self.bin(&[("+", Op::Add), ("-", Op::Sub)], Self::product)
    }

    fn product(&mut self) -> Result<Node, String> {
        self.bin(&[("*", Op::Mul), ("/", Op::Div), ("%", Op::Rem)], Self::unary)
    }

    fn unary(&mut self) -> Result<Node, String> {
        if self.eat("-") {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        if self.eat("!") {
            return Ok(Node::Not(Box::new(self.unary()?)));
        }
        if self.eat("+") {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Result<Node, String> {
        let base = self.atom()?;
        if self.eat("^") {
            // Right-associative, and binds tighter than a minus on its right: 2^-1.
            let exp = self.unary()?;
            return Ok(Node::Bin(Op::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<Node, String> {
        let tok = self.toks.get(self.i).cloned().ok_or("la fórmula termina antes de tiempo")?;
        self.i += 1;
        match tok {
            Tok::Num(n) => Ok(Node::Num(n)),
            Tok::Sym("(") => {
                let e = self.or()?;
                if !self.eat(")") {
                    return Err("falta cerrar un «)»".into());
                }
                Ok(e)
            }
            Tok::Name(n) if self.peek("(") => {
                self.i += 1;
                let (f, lo, hi) = Func::by_name(&n).ok_or(format!("no hay ninguna función «{n}»"))?;
                let mut args = vec![];
                if !self.eat(")") {
                    loop {
                        args.push(self.or()?);
                        if self.eat(")") {
                            break;
                        }
                        if !self.eat(",") {
                            return Err(format!("en «{n}(…)» falta una «,» o un «)»"));
                        }
                    }
                }
                if args.len() < lo || args.len() > hi {
                    let want = if lo == hi { format!("{lo}") } else { format!("de {lo} a {hi}") };
                    return Err(format!("«{n}» lleva {want} argumentos y tiene {}", args.len()));
                }
                Ok(Node::Call(f, args))
            }
            Tok::Name(n) => self.name(&n),
            Tok::Sym(s) => Err(format!("no esperaba «{s}»")),
        }
    }

    fn name(&mut self, n: &str) -> Result<Node, String> {
        match n {
            "pi" => return Ok(Node::Num(std::f64::consts::PI)),
            "tau" => return Ok(Node::Num(std::f64::consts::TAU)),
            "true" => return Ok(Node::Num(1.0)),
            "false" => return Ok(Node::Num(0.0)),
            _ => {}
        }
        if let Some(i) = VARS.iter().position(|v| *v == n) {
            return Ok(Node::Var(i));
        }
        if let Some(s) = n.strip_prefix("s.") {
            return STATES.iter().position(|x| *x == s).map(Node::Is).ok_or(format!("no hay ningún estado «{s}»"));
        }
        if let Some(e) = n.strip_prefix("ev.") {
            return EVENTS.iter().position(|x| *x == e).map(Node::Since).ok_or(format!("no hay ningún evento «{e}»"));
        }
        if let Some(i) = self.names.settings.iter().position(|s| s == n) {
            return Ok(Node::Setting(i));
        }
        let ch = &mut *self.names.channels;
        let i = match ch.iter().position(|c| c == n) {
            Some(i) => i,
            None => {
                ch.push(n.to_string());
                ch.len() - 1
            }
        };
        Ok(Node::Channel(i))
    }
}

fn truth(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// A hash of a number to 0..1, the same everywhere.
pub fn hash(x: f64) -> f64 {
    let mut h = x.to_bits().wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth value noise, -1..1.
pub fn noise(x: f64, y: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let at = |a: f64, b: f64| hash(a * 157.0 + b * 311.0 + 0.5) * 2.0 - 1.0;
    let s = |u: f64| u * u * (3.0 - 2.0 * u);
    let (u, v) = (s(xf), s(yf));
    let top = at(xi, yi) + (at(xi + 1.0, yi) - at(xi, yi)) * u;
    let bot = at(xi, yi + 1.0) + (at(xi + 1.0, yi + 1.0) - at(xi, yi + 1.0)) * u;
    top + (bot - top) * v
}

fn eval(n: &Node, f: &Frame) -> f64 {
    match n {
        Node::Num(v) => *v,
        Node::Var(i) => f.vars[*i],
        Node::Is(i) => truth(f.vars[3] as usize == *i),
        Node::Since(i) => f.since[*i],
        Node::Setting(i) => f.settings.get(*i).copied().unwrap_or(0.0),
        Node::Channel(i) => f.channels.get(*i).copied().unwrap_or(0.0),
        Node::Neg(a) => -eval(a, f),
        Node::Not(a) => truth(eval(a, f) <= 0.5),
        Node::Bin(op, a, b) => {
            let (x, y) = (eval(a, f), eval(b, f));
            match op {
                Op::Add => x + y,
                Op::Sub => x - y,
                Op::Mul => x * y,
                Op::Div => x / y,
                Op::Rem => x.rem_euclid(y),
                Op::Pow => x.powf(y),
                Op::Lt => truth(x < y),
                Op::Le => truth(x <= y),
                Op::Gt => truth(x > y),
                Op::Ge => truth(x >= y),
                Op::Eq => truth(x == y),
                Op::Ne => truth(x != y),
                Op::And => truth(x > 0.5 && y > 0.5),
                Op::Or => truth(x > 0.5 || y > 0.5),
            }
        }
        Node::Call(func, args) => {
            let a = |i: usize| eval(&args[i], f);
            match func {
                Func::Sin => a(0).sin(),
                Func::Cos => a(0).cos(),
                Func::Tan => a(0).tan(),
                Func::Asin => a(0).clamp(-1.0, 1.0).asin(),
                Func::Acos => a(0).clamp(-1.0, 1.0).acos(),
                Func::Atan => a(0).atan(),
                Func::Atan2 => a(0).atan2(a(1)),
                Func::Abs => a(0).abs(),
                Func::Sign => {
                    let x = a(0);
                    if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }
                }
                Func::Floor => a(0).floor(),
                Func::Ceil => a(0).ceil(),
                Func::Round => a(0).round(),
                Func::Fract => a(0).rem_euclid(1.0),
                Func::Sqrt => a(0).max(0.0).sqrt(),
                Func::Exp => a(0).exp(),
                Func::Ln => a(0).ln(),
                Func::Pow => a(0).powf(a(1)),
                Func::Min => args.iter().map(|x| eval(x, f)).fold(f64::INFINITY, f64::min),
                Func::Max => args.iter().map(|x| eval(x, f)).fold(f64::NEG_INFINITY, f64::max),
                Func::Clamp => a(0).max(a(1)).min(a(2)),
                Func::Mix => a(0) + (a(1) - a(0)) * a(2),
                Func::Smoothstep => {
                    let (e0, e1, x) = (a(0), a(1), a(2));
                    let u = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                    u * u * (3.0 - 2.0 * u)
                }
                Func::Step => truth(a(1) >= a(0)),
                Func::Noise => noise(a(0), if args.len() > 1 { a(1) } else { 0.0 }),
                Func::Hash => hash(a(0)),
                Func::If => {
                    if a(0) > 0.5 { a(1) } else { a(2) }
                }
                // 1 right when `x` is 0, down to 0 at `dur`: for what an event leaves behind.
                Func::Pulse => {
                    let (x, d) = (a(0), a(1));
                    if x >= 0.0 && x < d && d > 0.0 { 1.0 - x / d } else { 0.0 }
                }
                // A sine of `freq` turns per second (1 by default), 0..1.
                Func::Wave => {
                    let freq = if args.len() > 1 { a(1) } else { 1.0 };
                    0.5 + 0.5 * (a(0) * freq * std::f64::consts::TAU).sin()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, channels: &[(&str, f64)]) -> Result<f64, String> {
        let settings = vec!["size".to_string()];
        let mut names = vec![];
        let e = Expr::compile(src, &mut Names { settings: &settings, channels: &mut names })?;
        let vals: Vec<f64> = names.iter().map(|n| channels.iter().find(|c| c.0 == n).map_or(0.0, |c| c.1)).collect();
        let mut vars = [0.0; 9];
        vars[0] = 2.0;
        vars[3] = 7.0;
        let mut since = [NEVER; 12];
        since[1] = 0.1;
        Ok(e.eval(&Frame { vars, since, settings: &[1.5], channels: &vals }))
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(run("1 + 2 * 3", &[]), Ok(7.0));
        assert_eq!(run("(1 + 2) * 3", &[]), Ok(9.0));
        assert_eq!(run("2 ^ 3 ^ 2", &[]), Ok(512.0));
        assert_eq!(run("-2 ^ 2", &[]), Ok(-4.0));
        assert_eq!(run("2 ^ -1", &[]), Ok(0.5));
        assert_eq!(run("7 % 3", &[]), Ok(1.0));
        assert_eq!(run("1 < 2 && 3 > 2", &[]), Ok(1.0));
        assert_eq!(run("!0", &[]), Ok(1.0));
        assert_eq!(run("1e-3 * 1000", &[]), Ok(1.0));
    }

    #[test]
    fn names_resolve_in_order() {
        assert_eq!(run("t", &[]), Ok(2.0));
        assert_eq!(run("s.thinking", &[]), Ok(1.0));
        assert_eq!(run("s.idle", &[]), Ok(0.0));
        assert_eq!(run("size * 2", &[]), Ok(3.0));
        assert_eq!(run("body.radius + gaze.x", &[("body.radius", 0.4), ("gaze.x", 0.1)]), Ok(0.5));
        assert!(run("pulse(ev.error, 0.4)", &[]).unwrap() > 0.7);
        assert_eq!(run("pulse(ev.done, 0.4)", &[]), Ok(0.0));
    }

    #[test]
    fn functions() {
        assert_eq!(run("max(1, 5, 3)", &[]), Ok(5.0));
        assert_eq!(run("clamp(5, 0, 1)", &[]), Ok(1.0));
        assert_eq!(run("mix(0, 10, 0.25)", &[]), Ok(2.5));
        assert_eq!(run("if(1 > 2, 10, 20)", &[]), Ok(20.0));
        assert_eq!(run("wave(0.25)", &[]), Ok(1.0));
        assert!((-1.0..=1.0).contains(&run("noise(3.7, 1.2)", &[]).unwrap()));
    }

    #[test]
    fn errors_say_what_is_wrong() {
        assert!(run("1 +", &[]).unwrap_err().contains("termina"));
        assert!(run("foo(1)", &[]).unwrap_err().contains("función"));
        assert!(run("sin(1, 2)", &[]).unwrap_err().contains("argumentos"));
        assert!(run("(1", &[]).unwrap_err().contains("cerrar"));
        assert!(run("s.dancing", &[]).unwrap_err().contains("estado"));
        assert!(run("a = 1", &[]).unwrap_err().contains("=="));
        assert!(run("1 / 0", &[]) == Ok(0.0), "non-finite results are 0");
    }
}
