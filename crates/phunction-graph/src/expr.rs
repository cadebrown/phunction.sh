//! expr — phunction's little signal language.
//!
//! A pocket calculus for patching: `0.3*sin(t*0.5) + bass*0.4` is a whole
//! modulation source. Parsed by a hand-rolled Pratt parser into RPN, then
//! evaluated per frame against host-provided variables. This is the seed of
//! VISION §III's "editable code language for every component": blocks whose
//! behavior is a text field.
//!
//! Design rules:
//! - **Errors are addressed.** Every parse error carries a byte position so
//!   the UI can point at the exact character in theorem voice.
//! - **Variables are declared by the host** at parse time — an unknown name
//!   is a parse error, not a silent zero at play time.
//! - **Evaluation never fails.** Division by zero yields 0, not NaN — a
//!   modulation source must never poison the bus.

/// One RPN instruction.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    /// Push a literal.
    Lit(f32),
    /// Push a host variable by index (into the parse-time name table).
    Var(usize),
    /// Apply a unary function to the top of stack.
    Un(UnOp),
    /// Apply a binary function to the top two (top is rhs).
    Bin(BinOp),
    /// Apply a ternary function to the top three.
    Tern(TernOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnOp {
    Neg,
    Sin,
    Cos,
    Tan,
    Abs,
    Sqrt,
    Floor,
    Fract,
    Exp,
    Log,
    /// Triangle wave with period 1 over the input phase.
    Tri,
    /// Square wave (sign of sin) with period 1.
    Sqr,
    /// Sawtooth (fract) alias for musical intent.
    Saw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TernOp {
    Clamp,
    Lerp,
}

/// A compiled expression, ready to evaluate every frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    ops: Vec<Op>,
}

/// A parse failure, addressed to the offending byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset into the source where the problem starts.
    pub pos: usize,
    /// What went wrong, in words.
    pub msg: String,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "at byte {}: {}", self.pos, self.msg)
    }
}

/// Parse `src` against the host's variable names (e.g. `["t", "bass"]`).
///
/// # Errors
/// Returns a [`ParseError`] pointing at the first offending byte: unknown
/// symbols, wrong arity, unbalanced parens, dangling operators.
pub fn parse(src: &str, vars: &[&str]) -> Result<Program, ParseError> {
    let tokens = lex(src)?;
    let mut p = Parser {
        tokens: &tokens,
        ix: 0,
        vars,
        ops: Vec::new(),
    };
    p.expr(0)?;
    if p.ix != p.tokens.len() {
        return Err(ParseError {
            pos: p.tokens[p.ix].pos,
            msg: format!("unexpected `{}`", p.tokens[p.ix].text),
        });
    }
    Ok(Program { ops: p.ops })
}

impl Program {
    /// Evaluate against variable values parallel to the names passed to
    /// [`parse`]. Missing values read as 0. Never panics, never returns
    /// NaN/∞ — pathological math collapses to 0.
    #[must_use]
    pub fn eval(&self, values: &[f32]) -> f32 {
        let mut stack: Vec<f32> = Vec::with_capacity(8);
        for op in &self.ops {
            match *op {
                Op::Lit(v) => stack.push(v),
                Op::Var(i) => stack.push(values.get(i).copied().unwrap_or(0.0)),
                Op::Un(f) => {
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(apply_un(f, a));
                }
                Op::Bin(f) => {
                    let b = stack.pop().unwrap_or(0.0);
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(apply_bin(f, a, b));
                }
                Op::Tern(f) => {
                    let c = stack.pop().unwrap_or(0.0);
                    let b = stack.pop().unwrap_or(0.0);
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(apply_tern(f, a, b, c));
                }
            }
        }
        let out = stack.pop().unwrap_or(0.0);
        if out.is_finite() {
            out
        } else {
            0.0
        }
    }
}

fn apply_un(f: UnOp, a: f32) -> f32 {
    use core::f32::consts::TAU;
    match f {
        UnOp::Neg => -a,
        UnOp::Sin => (a * TAU).sin(),
        UnOp::Cos => (a * TAU).cos(),
        UnOp::Tan => (a * TAU).tan(),
        UnOp::Abs => a.abs(),
        UnOp::Sqrt => a.max(0.0).sqrt(),
        UnOp::Floor => a.floor(),
        UnOp::Fract => a.fract(),
        UnOp::Exp => a.exp(),
        UnOp::Log => {
            if a > 0.0 {
                a.ln()
            } else {
                0.0
            }
        }
        UnOp::Tri => 1.0 - 4.0 * ((a + 0.25).fract().abs() - 0.5).abs(),
        UnOp::Sqr => {
            if (a * TAU).sin() >= 0.0 {
                1.0
            } else {
                -1.0
            }
        }
        UnOp::Saw => 2.0 * (a - (a + 0.5).floor()),
    }
}

fn apply_bin(f: BinOp, a: f32, b: f32) -> f32 {
    match f {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => {
            if b.abs() > 1e-9 {
                a / b
            } else {
                0.0
            }
        }
        BinOp::Pow => a.powf(b),
        BinOp::Min => a.min(b),
        BinOp::Max => a.max(b),
    }
}

fn apply_tern(f: TernOp, a: f32, b: f32, c: f32) -> f32 {
    match f {
        TernOp::Clamp => a.clamp(b.min(c), c.max(b)),
        TernOp::Lerp => a + (b - a) * c,
    }
}

// ── lexer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct Token {
    pos: usize,
    text: String,
    kind: Tk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tk {
    Num,
    Ident,
    Punct,
}

fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < bytes.len() && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let text = &src[start..i];
            if text.parse::<f32>().is_err() {
                return Err(ParseError {
                    pos: start,
                    msg: format!("malformed number `{text}`"),
                });
            }
            out.push(Token {
                pos: start,
                text: text.to_string(),
                kind: Tk::Num,
            });
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len()
                && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(Token {
                pos: start,
                text: src[start..i].to_string(),
                kind: Tk::Ident,
            });
        } else if "+-*/^(),".contains(c) {
            out.push(Token {
                pos: i,
                text: c.to_string(),
                kind: Tk::Punct,
            });
            i += 1;
        } else {
            return Err(ParseError {
                pos: i,
                msg: format!("stray `{c}`"),
            });
        }
    }
    Ok(out)
}

// ── parser (precedence climbing) ─────────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [Token],
    ix: usize,
    vars: &'a [&'a str],
    ops: Vec<Op>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.ix)
    }

    fn end_pos(&self) -> usize {
        self.tokens.last().map_or(0, |t| t.pos + t.text.len())
    }

    fn expr(&mut self, min_bp: u8) -> Result<(), ParseError> {
        self.unary()?;
        while let Some(t) = self.peek() {
            let (bp, right_assoc, op) = match t.text.as_str() {
                "+" => (1, false, BinOp::Add),
                "-" => (1, false, BinOp::Sub),
                "*" => (2, false, BinOp::Mul),
                "/" => (2, false, BinOp::Div),
                "^" => (3, true, BinOp::Pow),
                _ => break,
            };
            if bp < min_bp {
                break;
            }
            self.ix += 1;
            self.expr(if right_assoc { bp } else { bp + 1 })?;
            self.ops.push(Op::Bin(op));
        }
        Ok(())
    }

    fn unary(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Some(t) if t.text == "-" => {
                self.ix += 1;
                self.unary()?;
                self.ops.push(Op::Un(UnOp::Neg));
                Ok(())
            }
            Some(t) if t.text == "+" => {
                self.ix += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn primary(&mut self) -> Result<(), ParseError> {
        let Some(t) = self.peek().cloned() else {
            return Err(ParseError {
                pos: self.end_pos(),
                msg: "expression ends too soon".into(),
            });
        };
        match t.kind {
            Tk::Num => {
                self.ix += 1;
                // lex() already validated the literal parses
                self.ops.push(Op::Lit(t.text.parse().unwrap_or(0.0)));
                Ok(())
            }
            Tk::Ident => {
                self.ix += 1;
                let calls = self.peek().is_some_and(|n| n.text == "(");
                if calls {
                    self.call(&t)
                } else if let Some(i) = self.vars.iter().position(|v| *v == t.text) {
                    self.ops.push(Op::Var(i));
                    Ok(())
                } else {
                    Err(ParseError {
                        pos: t.pos,
                        msg: format!(
                            "unknown symbol `{}` (have: {})",
                            t.text,
                            self.vars.join(", ")
                        ),
                    })
                }
            }
            Tk::Punct if t.text == "(" => {
                self.ix += 1;
                self.expr(0)?;
                self.expect(")")?;
                Ok(())
            }
            Tk::Punct => Err(ParseError {
                pos: t.pos,
                msg: format!("expected a value, found `{}`", t.text),
            }),
        }
    }

    fn call(&mut self, name: &Token) -> Result<(), ParseError> {
        let (arity, op): (usize, Op) = match name.text.as_str() {
            "sin" => (1, Op::Un(UnOp::Sin)),
            "cos" => (1, Op::Un(UnOp::Cos)),
            "tan" => (1, Op::Un(UnOp::Tan)),
            "abs" => (1, Op::Un(UnOp::Abs)),
            "sqrt" => (1, Op::Un(UnOp::Sqrt)),
            "floor" => (1, Op::Un(UnOp::Floor)),
            "fract" => (1, Op::Un(UnOp::Fract)),
            "exp" => (1, Op::Un(UnOp::Exp)),
            "log" => (1, Op::Un(UnOp::Log)),
            "tri" => (1, Op::Un(UnOp::Tri)),
            "sqr" => (1, Op::Un(UnOp::Sqr)),
            "saw" => (1, Op::Un(UnOp::Saw)),
            "min" => (2, Op::Bin(BinOp::Min)),
            "max" => (2, Op::Bin(BinOp::Max)),
            "pow" => (2, Op::Bin(BinOp::Pow)),
            "clamp" => (3, Op::Tern(TernOp::Clamp)),
            "lerp" => (3, Op::Tern(TernOp::Lerp)),
            _ => {
                return Err(ParseError {
                    pos: name.pos,
                    msg: format!("unknown function `{}`", name.text),
                })
            }
        };
        self.expect("(")?;
        for i in 0..arity {
            if i > 0 {
                self.expect(",")?;
            }
            self.expr(0)?;
        }
        self.expect(")")?;
        self.ops.push(op);
        Ok(())
    }

    fn expect(&mut self, text: &str) -> Result<(), ParseError> {
        match self.peek() {
            Some(t) if t.text == text => {
                self.ix += 1;
                Ok(())
            }
            Some(t) => Err(ParseError {
                pos: t.pos,
                msg: format!("expected `{text}`, found `{}`", t.text),
            }),
            None => Err(ParseError {
                pos: self.end_pos(),
                msg: format!("expected `{text}`, found the end"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARS: &[&str] = &["t", "beat", "bass", "mid", "air", "rms"];

    fn eval(src: &str, values: &[f32]) -> f32 {
        parse(src, VARS).expect(src).eval(values)
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3", &[]), 7.0);
        assert_eq!(eval("(1 + 2) * 3", &[]), 9.0);
        assert_eq!(eval("2 ^ 3 ^ 2", &[]), 512.0, "pow is right-associative");
        assert_eq!(eval("-2 ^ 2", &[]), 4.0, "unary binds the literal first");
        assert_eq!(eval("10 / 4", &[]), 2.5);
    }

    #[test]
    fn variables_resolve_by_position() {
        let v = [1.5, 0.0, 0.25, 0.0, 0.0, 0.9];
        assert_eq!(eval("t", &v), 1.5);
        assert_eq!(eval("bass + rms", &v), 1.15);
    }

    #[test]
    fn functions_and_arity() {
        assert!((eval("sin(0.25)", &[]) - 1.0).abs() < 1e-6);
        assert_eq!(eval("min(3, 2)", &[]), 2.0);
        assert_eq!(eval("clamp(5, 0, 1)", &[]), 1.0);
        assert_eq!(eval("lerp(0, 10, 0.3)", &[]), 3.0);
        assert_eq!(eval("tri(0.25)", &[]), 1.0);
        assert_eq!(eval("tri(0.75)", &[]), -1.0);
        assert!(eval("tri(0.5)", &[]).abs() < 1e-6);
    }

    #[test]
    fn eval_never_poisons_the_bus() {
        assert_eq!(eval("1 / 0", &[]), 0.0);
        assert_eq!(eval("log(-3)", &[]), 0.0);
        assert_eq!(eval("sqrt(-1)", &[]), 0.0);
        let big = eval("exp(exp(9))", &[]);
        assert!(big.is_finite());
    }

    #[test]
    fn errors_are_addressed() {
        let e = parse("bass + wobble", VARS).unwrap_err();
        assert_eq!(e.pos, 7);
        assert!(e.msg.contains("wobble"));

        let e = parse("sin(t", VARS).unwrap_err();
        assert!(e.msg.contains("expected `)`"));

        let e = parse("min(1)", VARS).unwrap_err();
        assert!(e.msg.contains("expected `,`"));

        let e = parse("2 +", VARS).unwrap_err();
        assert!(e.msg.contains("ends too soon"));

        let e = parse("1 ? 2", VARS).unwrap_err();
        assert_eq!(e.pos, 2);
    }

    #[test]
    fn the_shipping_default_parses() {
        let p = parse("0.3*sin(t*0.1) + bass*0.5", VARS).unwrap();
        let v = p.eval(&[2.5, 0.0, 0.8, 0.0, 0.0, 0.0]);
        assert!(v.is_finite());
        assert!(v.abs() <= 0.8);
    }
}
