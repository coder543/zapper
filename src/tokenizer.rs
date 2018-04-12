#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    OpeningBrace,
    ClosingBrace,
    OpeningParen,
    ClosingParen,
    Plus,
    Dash,
    Slash,
    Asterisk,
    Pipe,
    Number(f64),
    StringLiteral(&'a str),
    Identifier(&'a str),
    Raw(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct Tokenizer<'a> {
    source: &'a str,
    in_template: bool,
}

impl<'b> Tokenizer<'b> {
    pub fn new<'a>(source: &'a str) -> Tokenizer<'a> {
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
                        "+" => Ok(Plus),
                        "-" => Ok(Dash),
                        "/" => Ok(Slash),
                        "*" => Ok(Asterisk),
                        "|" => Ok(Pipe),
                        "(" => Ok(OpeningParen),
                        ")" => Ok(ClosingParen),
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

    #[test]
    fn tokens() {
        let source = r#"this is a very {{ adjective | to_upper }} system of extra {{super}}ness.
            {{2+2/1}}
            {{ concat "various tests " some_var }}
            {{ -3.4 * -count }}"#;

        assert_eq!(
            Tokenizer::new(source)
                .collect::<Result<Vec<_>, String>>()
                .unwrap(),
            [
                Raw("this is a very "),
                OpeningBrace,
                Identifier("adjective"),
                Pipe,
                Identifier("to_upper"),
                ClosingBrace,
                Raw(" system of extra "),
                OpeningBrace,
                Identifier("super"),
                ClosingBrace,
                Raw("ness.\n            "),
                OpeningBrace,
                Number(2.0),
                Plus,
                Number(2.0),
                Slash,
                Number(1.0),
                ClosingBrace,
                Raw("\n            "),
                OpeningBrace,
                Identifier("concat"),
                StringLiteral("various tests "),
                Identifier("some_var"),
                ClosingBrace,
                Raw("\n            "),
                OpeningBrace,
                Dash,
                Number(3.4),
                Asterisk,
                Dash,
                Identifier("count"),
                ClosingBrace
            ]
        );
    }
}
