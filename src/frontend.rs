use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Int,
    Void,
    Return,
    Ident(String),
    Number(i64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Eof,
}

pub struct Function {
    pub name: String,
    pub return_value: i64,
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
    let return_value = match p.bump() {
        Token::Number(value) => value,
        other => bail!("expected integer return value, found {other:?}"),
    };
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

    fn expect(&mut self, expected: Token) -> Result<()> {
        let actual = self.bump();
        if actual != expected {
            bail!("expected {expected:?}, found {actual:?}");
        }
        Ok(())
    }
}

fn lex(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
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
        assert_eq!(f.return_value, 42);
    }
}
