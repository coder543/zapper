#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
    OpeningBrace,
    ClosingBrace,
    Op(Operator),
    Number(f64),
    StringLiteral(&'a str),
    Identifier(&'a str),
    Raw(&'a str),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Operator {
    Plus,
    Dash,
    Slash,
    Asterisk,
    OpeningParen,
    ClosingParen,
    Pipe,
}

impl Operator {
    pub fn value(&self) -> u32 {
        use self::Operator::*;
        match *self {
            Pipe => 0,
            Plus => 1,
            Dash => 2,
            Asterisk => 3,
            Slash => 4,
            OpeningParen => 5,
            ClosingParen => 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tokenizer<'a> {
    source: &'a str,
    in_template: bool,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            source,
            in_template: false,
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>, String>;

    fn next(&mut self) -> Option<Result<Token<'a>, String>> {
        use self::Token::*;
        use self::Operator::*;

        if self.source.len() == 0 {
            return None;
        }

        if !self.in_template {
            let index = self.source.find("{{").unwrap_or_else(|| self.source.len());
            if index == 0 {
                // skip the opening curly braces
                self.source = &self.source[2..];
                self.in_template = true;
                return Some(Ok(OpeningBrace));
            } else {
                // return a chunk of raw text
                let (next, source) = self.source.split_at(index);
                self.source = source;
                return Some(Ok(Raw(next)));
            }
        } else {
            let word = self.source.split_whitespace().next()?;

            // whitespace-aware starting position
            let word_start = self.source.find(&word[0..1]).unwrap();
            self.source = &self.source[word_start..];

            let word_len = word.len();

            let mut end = self.source[..word_len].find("}}").unwrap_or(word_len);

            if end == 0 {
                self.source = &self.source[2..];
                self.in_template = false;
                return Some(Ok(ClosingBrace));
            }

            if let Some(operator) =
                self.source[..end].find(&['|', '*', '+', '-', '/', '(', ')', '"'] as &[char])
            {
                if operator == 0 {
                    let op = &self.source[0..1];
                    self.source = &self.source[1..];
                    return Some(match op {
                        "+" => Ok(Op(Plus)),
                        "-" => Ok(Op(Dash)),
                        "/" => Ok(Op(Slash)),
                        "*" => Ok(Op(Asterisk)),
                        "|" => Ok(Op(Pipe)),
                        "(" => Ok(Op(OpeningParen)),
                        ")" => Ok(Op(ClosingParen)),
                        "\"" => {
                            if let Some(end) = self.source.find('"') {
                                let quote = &self.source[..end];
                                self.source = &self.source[end + 1..];
                                Ok(StringLiteral(quote))
                            } else {
                                Err("No closing quotation mark".to_string())
                            }
                        }
                        op => Err(format!("invalid operator {}", op)),
                    });
                }
                end = operator;
            }

            let word = &word[..end];
            self.source = &self.source[end..];

            if let Ok(num) = word[..end].parse() {
                return Some(Ok(Number(num)));
            }

            if word.contains('"') {
                self.source = "";
                return Some(Err("Badly placed quotation mark".to_string()));
            }

            Some(Ok(Identifier(word)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tokenizer;
    use super::Token::*;
    use super::Operator::*;

    #[test]
    fn tokens() {
        let source = r#"this is a very {{ adjective | to_upper }} system of extra {{super}}ness.
            {{2+2/1}}
            {{ some_var | concat "various tests " }}
            {{ -3.4 * -count }}"#;

        assert_eq!(
            Tokenizer::new(source)
                .collect::<Result<Vec<_>, String>>()
                .unwrap(),
            vec![
                Raw("this is a very "),
                OpeningBrace,
                Identifier("adjective"),
                Op(Pipe),
                Identifier("to_upper"),
                ClosingBrace,
                Raw(" system of extra "),
                OpeningBrace,
                Identifier("super"),
                ClosingBrace,
                Raw("ness.\n            "),
                OpeningBrace,
                Number(2.0),
                Op(Plus),
                Number(2.0),
                Op(Slash),
                Number(1.0),
                ClosingBrace,
                Raw("\n            "),
                OpeningBrace,
                Identifier("some_var"),
                Op(Pipe),
                Identifier("concat"),
                StringLiteral("various tests "),
                ClosingBrace,
                Raw("\n            "),
                OpeningBrace,
                Op(Dash),
                Number(3.4),
                Op(Asterisk),
                Op(Dash),
                Identifier("count"),
                ClosingBrace,
            ]
        );
    }
}
