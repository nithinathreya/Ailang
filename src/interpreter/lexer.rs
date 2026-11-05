use super::token::{Span, Token, TokenType};
use anyhow::{anyhow, Result};

pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars = source.char_indices().collect();
        Self {
            source,
            chars,
            cursor: 0,
            line: 1,
            col: 1,
        }
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            let (_, c) = self.chars[self.cursor];
            self.cursor += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        }
    }

    fn current_span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace_and_comments()?;
            if self.is_at_end() {
                break;
            }

            let start_span = self.current_span();
            let c = self.advance().unwrap();

            let token = match c {
                '(' => Token::new(TokenType::LeftParen, start_span, "("),
                ')' => Token::new(TokenType::RightParen, start_span, ")"),
                '{' => Token::new(TokenType::LeftBrace, start_span, "{"),
                '}' => Token::new(TokenType::RightBrace, start_span, "}"),
                '[' => Token::new(TokenType::LeftBracket, start_span, "["),
                ']' => Token::new(TokenType::RightBracket, start_span, "]"),
                ',' => Token::new(TokenType::Comma, start_span, ","),
                ';' => Token::new(TokenType::Semicolon, start_span, ";"),
                ':' => Token::new(TokenType::Colon, start_span, ":"),
                '+' => Token::new(TokenType::Plus, start_span, "+"),
                '-' => Token::new(TokenType::Minus, start_span, "-"),
                '*' => Token::new(TokenType::Star, start_span, "*"),
                '/' => Token::new(TokenType::Slash, start_span, "/"),
                '%' => Token::new(TokenType::Percent, start_span, "%"),

                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::new(TokenType::EqualEqual, start_span, "==")
                    } else {
                        Token::new(TokenType::Equal, start_span, "=")
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::new(TokenType::BangEqual, start_span, "!=")
                    } else {
                        Token::new(TokenType::Bang, start_span, "!")
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::new(TokenType::LessEqual, start_span, "<=")
                    } else {
                        Token::new(TokenType::Less, start_span, "<")
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::new(TokenType::GreaterEqual, start_span, ">=")
                    } else {
                        Token::new(TokenType::Greater, start_span, ">")
                    }
                }
                '&' if self.peek() == Some('&') => {
                    self.advance();
                    Token::new(TokenType::And, start_span, "&&")
                }
                '|' if self.peek() == Some('|') => {
                    self.advance();
                    Token::new(TokenType::Or, start_span, "||")
                }

                '"' | '\'' => self.string_literal(c, start_span)?,

                _ if c.is_ascii_digit() => self.number_literal(c, start_span)?,
                _ if c.is_alphabetic() || c == '_' => self.identifier_or_keyword(c, start_span),

                _ => {
                    return Err(anyhow!(
                        "Lexer error at {}: Unrecognized character '{}'",
                        start_span,
                        c
                    ));
                }
            };

            tokens.push(token);
        }

        tokens.push(Token::new(TokenType::Eof, self.current_span(), ""));
        Ok(tokens)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == Some('/') {
                        // Single-line comment: // ...
                        self.advance();
                        self.advance();
                        while let Some(ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if self.peek_next() == Some('*') {
                        // Multi-line comment: /* ... */
                        let start_span = self.current_span();
                        self.advance();
                        self.advance();
                        let mut closed = false;
                        while !self.is_at_end() {
                            if self.peek() == Some('*') && self.peek_next() == Some('/') {
                                self.advance();
                                self.advance();
                                closed = true;
                                break;
                            }
                            self.advance();
                        }
                        if !closed {
                            return Err(anyhow!("Unterminated multi-line comment starting at {}", start_span));
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn string_literal(&mut self, quote: char, start_span: Span) -> Result<Token> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == quote {
                self.advance();
                return Ok(Token::new(TokenType::String(s.clone()), start_span, format!("\"{}\"", s)));
            }
            if c == '\\' {
                self.advance();
                match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('\'') => s.push('\''),
                    Some('"') => s.push('"'),
                    Some(other) => {
                        s.push('\\');
                        s.push(other);
                    }
                    None => return Err(anyhow!("Unterminated escape sequence in string at {}", start_span)),
                }
            } else {
                s.push(c);
                self.advance();
            }
        }
        Err(anyhow!("Unterminated string literal starting at {}", start_span))
    }

    fn number_literal(&mut self, first_digit: char, start_span: Span) -> Result<Token> {
        let mut num_str = String::new();
        num_str.push(first_digit);

        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else if c == '.' && !is_float && self.peek_next().map(|n| n.is_ascii_digit()).unwrap_or(false) {
                is_float = true;
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            let val = num_str.parse::<f64>().map_err(|e| anyhow!("Invalid float at {}: {}", start_span, e))?;
            Ok(Token::new(TokenType::Float(val), start_span, num_str))
        } else {
            let val = num_str.parse::<i64>().map_err(|e| anyhow!("Invalid integer at {}: {}", start_span, e))?;
            Ok(Token::new(TokenType::Integer(val), start_span, num_str))
        }
    }

    fn identifier_or_keyword(&mut self, first_char: char, start_span: Span) -> Token {
        let mut ident = String::new();
        ident.push(first_char);

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let token_type = match ident.as_str() {
            "let" => TokenType::Let,
            "fn" => TokenType::Fn,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "return" => TokenType::Return,
            "print" => TokenType::Print,
            "true" => TokenType::Boolean(true),
            "false" => TokenType::Boolean(false),
            "nil" | "null" => TokenType::Nil,
            "and" => TokenType::And,
            "or" => TokenType::Or,
            _ => TokenType::Ident(ident.clone()),
        };

        Token::new(token_type, start_span, ident)
    }
}
