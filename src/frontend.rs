use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Int,
    Void,
    Return,
    Ident(String),
    Number(i64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Integer(i64),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Rem(Box<Expr>, Box<Expr>),
}

pub struct Function {
    pub name: String,
    pub return_value: Expr,
}

pub fn parse(source: &str) -> Result<Function> {
    let tokens = lex(source)?;
    let mut p = Parser { tokens, pos: 0 };
    p.expect(Token::Int)?;
    let name = match p.bump() {
        Token::Ident(name) => name,
        other => bail!("expected function name, found {other:?}"),
    };
    p.expect(Token::LParen)?;
    p.expect(Token::Void)?;
    p.expect(Token::RParen)?;
    p.expect(Token::LBrace)?;
    p.expect(Token::Return)?;
    let return_value = p.parse_expr()?;
    p.expect(Token::Semi)?;
    p.expect(Token::RBrace)?;
    p.expect(Token::Eof)?;
    Ok(Function { name, return_value })
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        token
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let actual = self.bump();
        if actual != expected {
            bail!("expected {expected:?}, found {actual:?}");
        }
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            expr = match self.peek() {
                Token::Plus => {
                    self.bump();
                    Expr::Add(Box::new(expr), Box::new(self.parse_multiplicative()?))
                }
                Token::Minus => {
                    self.bump();
                    Expr::Sub(Box::new(expr), Box::new(self.parse_multiplicative()?))
                }
                _ => break,
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut expr = self.parse_unary()?;
        loop {
            expr = match self.peek() {
                Token::Star => {
                    self.bump();
                    Expr::Mul(Box::new(expr), Box::new(self.parse_unary()?))
                }
                Token::Slash => {
                    self.bump();
                    Expr::Div(Box::new(expr), Box::new(self.parse_unary()?))
                }
                Token::Percent => {
                    self.bump();
                    Expr::Rem(Box::new(expr), Box::new(self.parse_unary()?))
                }
                _ => break,
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.peek() == &Token::Minus {
            self.bump();
            return Ok(Expr::Neg(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.bump() {
            Token::Number(value) => Ok(Expr::Integer(value)),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => bail!("expected expression, found {other:?}"),
        }
    }
}

fn lex(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'+' => {
                out.push(Token::Plus);
                i += 1;
            }
            b'-' => {
                out.push(Token::Minus);
                i += 1;
            }
            b'*' => {
                out.push(Token::Star);
                i += 1;
            }
            b'/' => {
                out.push(Token::Slash);
                i += 1;
            }
            b'%' => {
                out.push(Token::Percent);
                i += 1;
            }
            b'(' => {
                out.push(Token::LParen);
                i += 1;
            }
            b')' => {
                out.push(Token::RParen);
                i += 1;
            }
            b'{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            b'}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            b';' => {
                out.push(Token::Semi);
                i += 1;
            }
            b'0'..=b'9' => {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let value = source[start..i]
                    .parse()
                    .context("invalid integer literal")?;
                out.push(Token::Number(value));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                out.push(match &source[start..i] {
                    "int" => Token::Int,
                    "void" => Token::Void,
                    "return" => Token::Return,
                    ident => Token::Ident(ident.to_owned()),
                });
            }
            other => bail!("unexpected byte {:?} at offset {i}", other as char),
        }
    }
    out.push(Token::Eof);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_program() {
        let f = parse("int main(void) { return 42; }").unwrap();
        assert_eq!(f.name, "main");
        assert_eq!(f.return_value, Expr::Integer(42));
    }

    #[test]
    fn respects_operator_precedence() {
        let f = parse("int main(void) { return 2 + 3 * 4; }").unwrap();
        assert_eq!(
            f.return_value,
            Expr::Add(
                Box::new(Expr::Integer(2)),
                Box::new(Expr::Mul(
                    Box::new(Expr::Integer(3)),
                    Box::new(Expr::Integer(4))
                ))
            )
        );
    }

    #[test]
    fn parses_parentheses_and_unary_minus() {
        let f = parse("int main(void) { return -(2 + 3); }").unwrap();
        assert_eq!(
            f.return_value,
            Expr::Neg(Box::new(Expr::Add(
                Box::new(Expr::Integer(2)),
                Box::new(Expr::Integer(3))
            )))
        );
    }

    #[test]
    fn parses_remainder_with_multiplicative_precedence() {
        let f = parse("int main(void) { return 40 + 8 % 3; }").unwrap();
        assert_eq!(
            f.return_value,
            Expr::Add(
                Box::new(Expr::Integer(40)),
                Box::new(Expr::Rem(
                    Box::new(Expr::Integer(8)),
                    Box::new(Expr::Integer(3))
                ))
            )
        );
    }
}
