#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Nil,

    // Identifiers & Keywords
    Ident(String),
    Let,
    Fn,
    If,
    Else,
    While,
    For,
    Return,
    Print,
    True,
    False,

    // Operators
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Equal,        // =
    EqualEqual,   // ==
    Bang,         // !
    BangEqual,    // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    And,          // && or 'and'
    Or,           // || or 'or'

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Semicolon,    // ;
    Colon,        // :

    // Special
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(token_type: TokenType, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            token_type,
            span,
            lexeme: lexeme.into(),
        }
    }
}
