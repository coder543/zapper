use std::iter::Peekable;
use tokenizer::{Operator, Token, Tokenizer};

pub type Ident<'a> = &'a str;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'a> {
    Raw(&'a str),
    Filter(Ident<'a>, Box<Expr<'a>>, Vec<Literal<'a>>),
    StringLiteral(&'a str),
    Identifier(Ident<'a>),
    Numeric(Numeric<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Numeric<'a> {
    Raw(f64),
    Identifier(&'a str),
    Negate(Box<Numeric<'a>>),
    Parentheses(Box<Numeric<'a>>),
    Binary(Operator, Box<Numeric<'a>>, Box<Numeric<'a>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal<'a> {
    Number(f64),
    StringLiteral(&'a str),
}

type PeekTokenizer<'a> = Peekable<Tokenizer<'a>>;

pub fn parse<'a>(tokenizer: Tokenizer<'a>) -> Result<Vec<Expr<'a>>, String> {
    let mut tokenizer = tokenizer.peekable();
    let mut nodes = Vec::new();
    loop {
        match Expr::parse_outer(&mut tokenizer) {
            Ok(node) => nodes.push(node),
            Err(ref err) if err.is_empty() => return Ok(nodes),
            Err(err) => return Err(err),
        }
    }
}

macro_rules! next {
    ($tokenizer: ident) => {
        $tokenizer
            .next()
            .ok_or_else(|| String::from("Unexpected end of input!"))??
    };
    ($tokenizer: ident, $err: tt) => {
        $tokenizer.next().ok_or_else(|| String::from($err))??
    };
}

macro_rules! peek {
    ($tokenizer: ident) => {
        match $tokenizer.peek() {
            Some(&Err(_)) | None => None,
            Some(&Ok(ref val)) => Some(val),
        }
    };
    ($tokenizer: ident, $err: tt) => {
        match $tokenizer.peek().ok_or_else(|| String::from($err))? {
            &Err(_) => Err(String::from($err))?,
            &Ok(ref val) => val,
        }
    };
}

macro_rules! next_and_peek {
    ($tokenizer: ident, $err: tt) => {
        (next!($tokenizer, $err), peek!($tokenizer))
    };
}

impl<'a> Expr<'a> {
    fn parse_outer(tokenizer: &mut PeekTokenizer<'a>) -> Result<Expr<'a>, String> {
        match next!(tokenizer, "") {
            Token::Raw(string) => Ok(Expr::Raw(string)),
            Token::OpeningBrace => {
                let expr = Expr::parse(tokenizer);
                if let Err(err) = expr {
                    return Err(format!("Error: {}", err));
                }
                match next!(tokenizer, UNEXPECTED_EOB) {
                    Token::ClosingBrace => expr,
                    tok => Err(format!(
                        "Unexpectedly found {:#?} after {:#?}. Remaining tokens: {:#?}",
                        tok,
                        expr,
                        tokenizer.collect::<Vec<_>>(),
                    ))?,
                }
            }
            _ => Ok(Expr::Raw("Unexpected token!")),
        }
    }

    fn parse(tokenizer: &mut PeekTokenizer<'a>) -> Result<Expr<'a>, String> {
        let expr = match next_and_peek!(tokenizer, UNEXPECTED_EOB) {
            (Token::Op(op), _) => Expr::Numeric(Numeric::unary_operator(op, tokenizer)?),
            (token, Some(&Token::Op(op))) if op != Operator::Pipe => {
                Expr::Numeric(Numeric::binary_operator(token, tokenizer)?)
            }

            (Token::ClosingBrace, _) => Err("Empty block is invalid!")?,
            (token, _) => Expr::from_token(token)?,
        };

        if let Some(&Token::Op(Operator::Pipe)) = peek!(tokenizer) {
            next!(tokenizer);
            Expr::filter(expr, tokenizer)
        } else {
            Ok(expr)
        }
    }

    fn filter(expr: Expr<'a>, tokenizer: &mut PeekTokenizer<'a>) -> Result<Expr<'a>, String> {
        let ident = match Expr::parse(tokenizer)? {
            Expr::Identifier(ident) => ident,
            token => Err(format!(
                "Illegal token {:?} found while expecting the name of a filter",
                token
            ))?,
        };
        let args = Expr::get_args(tokenizer)?;
        let expr = Expr::Filter(ident, Box::new(expr), args);

        if let Some(&Token::Op(Operator::Pipe)) = peek!(tokenizer) {
            next!(tokenizer);
            Expr::filter(expr, tokenizer)
        } else {
            Ok(expr)
        }
    }

    fn get_args(tokenizer: &mut PeekTokenizer<'a>) -> Result<Vec<Literal<'a>>, String> {
        let mut args = Vec::new();
        loop {
            match peek!(tokenizer, UNEXPECTED_EOB) {
                &Token::ClosingBrace | &Token::Op(Operator::Pipe) => return Ok(args),
                _ => args.push(Literal::parse(tokenizer)?),
            }
        }
    }

    fn from_token(token: Token<'a>) -> Result<Expr<'a>, String> {
        match token {
            Token::Identifier(ident) => Ok(Expr::Identifier(ident)),
            Token::Number(num) => Ok(Expr::Numeric(Numeric::Raw(num))),
            Token::StringLiteral(string) => Ok(Expr::StringLiteral(string)),
            token => Err(format!("Invalid token: {:?}", token)),
        }
    }
}

impl<'a> Numeric<'a> {
    fn parse(tokenizer: &mut PeekTokenizer<'a>) -> Result<Numeric<'a>, String> {
        match next_and_peek!(tokenizer, "Expected numeric value, found end of input!") {
            (Token::Op(op), _) => Numeric::unary_operator(op, tokenizer),
            (token, Some(&Token::Op(op)))
                if op != Operator::ClosingParen && op != Operator::Pipe =>
            {
                Numeric::binary_operator(token, tokenizer)
            }
            (Token::Number(num), _) => Ok(Numeric::Raw(num)),
            (Token::Identifier(ident), _) => Ok(Numeric::Identifier(ident)),
            (Token::StringLiteral(string), _) => Err(format!(
                "Found string {:?} when looking for a numeric literal!",
                string
            )),
            (token, _) => Err(format!(
                "Illegal token {:?} found while trying to parse numeric literal",
                token
            )),
        }
    }

    fn parenthetical(tokenizer: &mut PeekTokenizer<'a>) -> Result<Numeric<'a>, String> {
        let expr = Numeric::parse(tokenizer)?;
        match next!(tokenizer) {
            Token::Op(Operator::ClosingParen) => Ok(Numeric::Parentheses(Box::new(expr))),
            Token::Op(Operator::Pipe) => {
                Err("You must close all parentheses before using a filter!")?
            }
            tok => Err(format!(
                "A closing parenthesis is missing! Found {:?} instead.",
                tok
            ))?,
        }
    }

    fn unary_operator(
        op: Operator,
        tokenizer: &mut PeekTokenizer<'a>,
    ) -> Result<Numeric<'a>, String> {
        match op {
            Operator::Dash => Ok(Numeric::Negate(Box::new(Numeric::parse(tokenizer)?))),
            Operator::OpeningParen => Numeric::parenthetical(tokenizer),
            _ => Err(format!("invalid unary operator: {:?}", op)),
        }
    }

    fn binary_operator(
        tok: Token<'a>,
        tokenizer: &mut PeekTokenizer<'a>,
    ) -> Result<Numeric<'a>, String> {
        let op = match next!(tokenizer) {
            Token::Op(op) => op,
            other => {
                return Err(format!(
                    "internal parser error! found {:?} when looking for operator.",
                    other
                ))
            }
        };

        let next = Numeric::parse(tokenizer)?;
        let op_val = match next {
            Numeric::Binary(op, _, _) => op.value(),
            _ => 50,
        };

        match op {
            Operator::Plus | Operator::Dash | Operator::Asterisk | Operator::Slash => {
                Ok(if op_val < op.value() {
                    match next {
                        Numeric::Binary(op2, expr1, expr2) => Numeric::Binary(
                            op2,
                            Box::new(Numeric::Binary(
                                op,
                                Box::new(Numeric::from_token(tok)?),
                                expr1,
                            )),
                            expr2,
                        ),
                        op => return Err(format!("Illegal operator {:?} found", op)),
                    }
                } else {
                    Numeric::Binary(op, Box::new(Numeric::from_token(tok)?), Box::new(next))
                })
            }
            op => Err(format!("Illegal operator {:?} found", op)),
        }
    }

    fn from_token(token: Token<'a>) -> Result<Numeric<'a>, String> {
        match token {
            Token::Number(num) => Ok(Numeric::Raw(num)),
            Token::Identifier(ident) => Ok(Numeric::Identifier(ident)),
            Token::StringLiteral(string) => Err(format!(
                "Found string {:?} when looking for a numeric literal!",
                string
            )),
            token => Err(format!("Illegal token {:?} found", token)),
        }
    }
}

impl<'a> Literal<'a> {
    fn parse(tokenizer: &mut PeekTokenizer<'a>) -> Result<Literal<'a>, String> {
        match next!(tokenizer) {
            Token::StringLiteral(string) => Ok(Literal::StringLiteral(string)),
            Token::Number(num) => Ok(Literal::Number(num)),
            token => Err(format!("Illegal token {:?} found", token)),
        }
    }
}

const UNEXPECTED_EOB: &str = "Unexpected end of input in unclosed substitution block!";

#[cfg(test)]
mod tests {
    use super::*;
    use tokenizer::Tokenizer;

    #[test]
    fn arithmetic() {
        let source = r#"This is a test {{ 3 / 4 - (2 + 4) }} and even more!"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a test "),
                Expr::Numeric(Numeric::Binary(
                    Operator::Dash,
                    Box::new(Numeric::Binary(
                        Operator::Slash,
                        Box::new(Numeric::Raw(3.0)),
                        Box::new(Numeric::Raw(4.0))
                    )),
                    Box::new(Numeric::Parentheses(Box::new(Numeric::Binary(
                        Operator::Plus,
                        Box::new(Numeric::Raw(2.0)),
                        Box::new(Numeric::Raw(4.0))
                    ))))
                )),
                Expr::Raw(" and even more!")
            ]
        );
    }

    #[test]
    fn arithmetic_with_identifiers() {
        let source = r#"This is a test {{ x / height - (y + n) }} and even more!"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a test "),
                Expr::Numeric(Numeric::Binary(
                    Operator::Dash,
                    Box::new(Numeric::Binary(
                        Operator::Slash,
                        Box::new(Numeric::Identifier("x")),
                        Box::new(Numeric::Identifier("height"))
                    )),
                    Box::new(Numeric::Parentheses(Box::new(Numeric::Binary(
                        Operator::Plus,
                        Box::new(Numeric::Identifier("y")),
                        Box::new(Numeric::Identifier("n"))
                    ))))
                )),
                Expr::Raw(" and even more!")
            ]
        );
    }

    #[test]
    fn multiple_substitutions() {
        let source = r#"This is a {{multi}} substitution {{ template }}"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a "),
                Expr::Identifier("multi"),
                Expr::Raw(" substitution "),
                Expr::Identifier("template"),
            ]
        );
    }

    #[test]
    fn only_substitution() {
        let source = r#"{{template}}"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(exprs, [Expr::Identifier("template")]);
    }

    #[test]
    fn filter() {
        let source = r#"This is a test {{ height / 3 | round 2 }} and even more!"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a test "),
                Expr::Filter(
                    "round",
                    Box::new(Expr::Numeric(Numeric::Binary(
                        Operator::Slash,
                        Box::new(Numeric::Identifier("height")),
                        Box::new(Numeric::Raw(3.0))
                    ))),
                    vec![Literal::Number(2.0)]
                ),
                Expr::Raw(" and even more!")
            ]
        );
    }

    #[test]
    fn simple_filter() {
        let source = r#"This is a test {{ height | hex }} and even more!"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a test "),
                Expr::Filter("hex", Box::new(Expr::Identifier("height")), vec![]),
                Expr::Raw(" and even more!")
            ]
        );
    }

    #[test]
    fn multi_filter() {
        let source = r#"This is a test {{ height / 3 | round 2 | hex }} and even more!"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("This is a test "),
                Expr::Filter(
                    "hex",
                    Box::new(Expr::Filter(
                        "round",
                        Box::new(Expr::Numeric(Numeric::Binary(
                            Operator::Slash,
                            Box::new(Numeric::Identifier("height")),
                            Box::new(Numeric::Raw(3.0))
                        ))),
                        vec![Literal::Number(2.0)]
                    )),
                    vec![]
                ),
                Expr::Raw(" and even more!")
            ]
        );
    }

    #[test]
    fn parse_tokenizer_test_source() {
        let source = r#"this is a very {{ adjective | to_upper }} system of extra {{super}}ness.
            {{2+2/1}}
            {{ some_var | concat "various tests " }}
            {{ -3.4 * -count }}"#;
        let tokenizer = Tokenizer::new(source);
        let exprs = parse(tokenizer).unwrap();
        assert_eq!(
            exprs,
            [
                Expr::Raw("this is a very "),
                Expr::Filter("to_upper", Box::new(Expr::Identifier("adjective")), vec![]),
                Expr::Raw(" system of extra "),
                Expr::Identifier("super"),
                Expr::Raw("ness.\n            "),
                Expr::Numeric(Numeric::Binary(
                    Operator::Plus,
                    Box::new(Numeric::Raw(2.0)),
                    Box::new(Numeric::Binary(
                        Operator::Slash,
                        Box::new(Numeric::Raw(2.0)),
                        Box::new(Numeric::Raw(1.0)),
                    )),
                )),
                Expr::Raw("\n            "),
                Expr::Filter(
                    "concat",
                    Box::new(Expr::Identifier("some_var")),
                    vec![Literal::StringLiteral("various tests ")],
                ),
                Expr::Raw("\n            "),
                Expr::Numeric(Numeric::Negate(Box::new(Numeric::Binary(
                    Operator::Asterisk,
                    Box::new(Numeric::Raw(3.4)),
                    Box::new(Numeric::Negate(Box::new(Numeric::Identifier("count")))),
                )))),
            ]
        );
    }
}
