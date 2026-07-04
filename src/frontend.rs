use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Int, Void, Return, Ident(String), Number(i64), Plus, Minus, Star, Slash, Percent,
    Bang, EqEq, NotEq, Lt, Le, Gt, Ge, LParen, RParen, LBrace, RBrace, Semi, Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Integer(i64), Neg(Box<Expr>), Not(Box<Expr>), Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>), Mul(Box<Expr>, Box<Expr>), Div(Box<Expr>, Box<Expr>),
    Rem(Box<Expr>, Box<Expr>), Eq(Box<Expr>, Box<Expr>), Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>), Le(Box<Expr>, Box<Expr>), Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
}

pub struct Function { pub name: String, pub return_value: Expr }

pub fn parse(source: &str) -> Result<Function> {
    let mut p = Parser { tokens: lex(source)?, pos: 0 };
    p.expect(Token::Int)?;
    let name = match p.bump() { Token::Ident(name) => name, other => bail!("expected function name, found {other:?}") };
    p.expect(Token::LParen)?; p.expect(Token::Void)?; p.expect(Token::RParen)?;
    p.expect(Token::LBrace)?; p.expect(Token::Return)?;
    let return_value = p.parse_expr()?;
    p.expect(Token::Semi)?; p.expect(Token::RBrace)?; p.expect(Token::Eof)?;
    Ok(Function { name, return_value })
}

struct Parser { tokens: Vec<Token>, pos: usize }
impl Parser {
    fn bump(&mut self) -> Token { let t = self.tokens[self.pos].clone(); self.pos += 1; t }
    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn expect(&mut self, expected: Token) -> Result<()> { let actual = self.bump(); if actual != expected { bail!("expected {expected:?}, found {actual:?}"); } Ok(()) }
    fn parse_expr(&mut self) -> Result<Expr> { self.parse_equality() }
    fn parse_equality(&mut self) -> Result<Expr> { let mut e = self.parse_relational()?; loop { e = match self.peek() { Token::EqEq => { self.bump(); Expr::Eq(Box::new(e), Box::new(self.parse_relational()?)) }, Token::NotEq => { self.bump(); Expr::Ne(Box::new(e), Box::new(self.parse_relational()?)) }, _ => break }; } Ok(e) }
    fn parse_relational(&mut self) -> Result<Expr> { let mut e = self.parse_additive()?; loop { e = match self.peek() { Token::Lt => { self.bump(); Expr::Lt(Box::new(e), Box::new(self.parse_additive()?)) }, Token::Le => { self.bump(); Expr::Le(Box::new(e), Box::new(self.parse_additive()?)) }, Token::Gt => { self.bump(); Expr::Gt(Box::new(e), Box::new(self.parse_additive()?)) }, Token::Ge => { self.bump(); Expr::Ge(Box::new(e), Box::new(self.parse_additive()?)) }, _ => break }; } Ok(e) }
    fn parse_additive(&mut self) -> Result<Expr> { let mut e = self.parse_multiplicative()?; loop { e = match self.peek() { Token::Plus => { self.bump(); Expr::Add(Box::new(e), Box::new(self.parse_multiplicative()?)) }, Token::Minus => { self.bump(); Expr::Sub(Box::new(e), Box::new(self.parse_multiplicative()?)) }, _ => break }; } Ok(e) }
    fn parse_multiplicative(&mut self) -> Result<Expr> { let mut e = self.parse_unary()?; loop { e = match self.peek() { Token::Star => { self.bump(); Expr::Mul(Box::new(e), Box::new(self.parse_unary()?)) }, Token::Slash => { self.bump(); Expr::Div(Box::new(e), Box::new(self.parse_unary()?)) }, Token::Percent => { self.bump(); Expr::Rem(Box::new(e), Box::new(self.parse_unary()?)) }, _ => break }; } Ok(e) }
    fn parse_unary(&mut self) -> Result<Expr> { match self.peek() { Token::Minus => { self.bump(); Ok(Expr::Neg(Box::new(self.parse_unary()?))) }, Token::Bang => { self.bump(); Ok(Expr::Not(Box::new(self.parse_unary()?))) }, _ => self.parse_primary() } }
    fn parse_primary(&mut self) -> Result<Expr> { match self.bump() { Token::Number(v) => Ok(Expr::Integer(v)), Token::LParen => { let e = self.parse_expr()?; self.expect(Token::RParen)?; Ok(e) }, other => bail!("expected expression, found {other:?}") } }
}

fn lex(source: &str) -> Result<Vec<Token>> {
    let b = source.as_bytes(); let mut out = Vec::new(); let mut i = 0;
    while i < b.len() { match b[i] {
        b' ' | b'\t' | b'\r' | b'\n' => i += 1,
        b'=' if b.get(i + 1) == Some(&b'=') => { out.push(Token::EqEq); i += 2; },
        b'!' if b.get(i + 1) == Some(&b'=') => { out.push(Token::NotEq); i += 2; },
        b'!' => { out.push(Token::Bang); i += 1; },
        b'<' if b.get(i + 1) == Some(&b'=') => { out.push(Token::Le); i += 2; },
        b'>' if b.get(i + 1) == Some(&b'=') => { out.push(Token::Ge); i += 2; },
        b'<' => { out.push(Token::Lt); i += 1; }, b'>' => { out.push(Token::Gt); i += 1; },
        b'+' => { out.push(Token::Plus); i += 1; }, b'-' => { out.push(Token::Minus); i += 1; },
        b'*' => { out.push(Token::Star); i += 1; }, b'/' => { out.push(Token::Slash); i += 1; },
        b'%' => { out.push(Token::Percent); i += 1; }, b'(' => { out.push(Token::LParen); i += 1; },
        b')' => { out.push(Token::RParen); i += 1; }, b'{' => { out.push(Token::LBrace); i += 1; },
        b'}' => { out.push(Token::RBrace); i += 1; }, b';' => { out.push(Token::Semi); i += 1; },
        b'0'..=b'9' => { let s = i; while i < b.len() && b[i].is_ascii_digit() { i += 1; } out.push(Token::Number(source[s..i].parse().context("invalid integer literal")?)); },
        b'a'..=b'z' | b'A'..=b'Z' | b'_' => { let s = i; while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') { i += 1; } out.push(match &source[s..i] { "int" => Token::Int, "void" => Token::Void, "return" => Token::Return, x => Token::Ident(x.to_owned()) }); },
        other => bail!("unexpected byte {:?} at offset {i}", other as char),
    }} out.push(Token::Eof); Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_first_program() { let f = parse("int main(void) { return 42; }").unwrap(); assert_eq!(f.name, "main"); assert_eq!(f.return_value, Expr::Integer(42)); }
    #[test] fn parses_all_comparisons() { for op in ["==", "!=", "<", "<=", ">", ">="] { parse(&format!("int main(void) {{ return 2 {op} 3; }}")).unwrap(); } }
    #[test] fn parses_logical_not() { let f = parse("int main(void) { return !!42; }").unwrap(); assert!(matches!(f.return_value, Expr::Not(_))); }
    #[test] fn comparison_precedence_is_c_like() { let f = parse("int main(void) { return 1 + 2 < 4 == 1; }").unwrap(); assert!(matches!(f.return_value, Expr::Eq(_, _))); }
}
