use std::str::Chars;
use std::iter::Peekable;

/// Token kinds for the iot-flow DSL.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenKind {
    // Single-character symbols
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semicolon,

    // Operators
    Assign,          // =
    Plus,            // +
    Minus,           // -
    Star,            // *
    Slash,           // /
    Percent,         // %

    // Literals
    Identifier(String),
    Number(f64),
    StringLiteral(String),

    // Keywords
    If,
    Else,
    While,
    For,
    In,
    Function,

    Eof,
}

/// A token with its kind and the position where it was found.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Token { kind, line, column }
    }
}

/// Errors that can occur during tokenization.
#[derive(Debug, PartialEq)]
pub enum LexError {
    UnexpectedChar(char, usize, usize),
    UnterminatedString(usize, usize),
    InvalidNumber(String, usize, usize),
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedChar(ch, line, col) => {
                write!(f, "unexpected character '{}' at {}:{}", ch, line, col)
            }
            LexError::UnterminatedString(line, col) => {
                write!(f, "unterminated string starting at {}:{}", line, col)
            }
            LexError::InvalidNumber(s, line, col) => {
                write!(f, "invalid number '{}' at {}:{}", s, line, col)
            }
        }
    }
}

impl std::error::Error for LexError {}

/// The tokenizer (lexer) walks over the input and produces a stream of tokens.
pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,
    current_char: Option<char>,
    line: usize,
    column: usize,
    /// Position in the original string where the current token starts
    start_line: usize,
    start_column: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from a source string slice.
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            source: input.chars().peekable(),
            current_char: None,
            line: 1,
            column: 0,
            start_line: 1,
            start_column: 1,
        };
        lexer.advance(); // initialize current_char
        lexer
    }

    /// Advance to the next character.
    fn advance(&mut self) {
        if let Some(ch) = self.current_char {
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        self.current_char = self.source.next();
    }

    /// Peek at the next character without consuming it.
    fn peek(&mut self) -> Option<char> {
        self.source.peek().copied()
    }

    /// Consume whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_char {
                Some(ch) if ch.is_ascii_whitespace() => self.advance(),
                Some('/') if self.peek() == Some('/') => {
                    // Single-line comment
                    while let Some(c) = self.current_char {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('#') => {
                    // Hash-style comment
                    while let Some(c) = self.current_char {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    /// Consume an identifier or keyword.
    fn consume_identifier(&mut self) -> TokenKind {
        let mut ident = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "function" => TokenKind::Function,
            _ => TokenKind::Identifier(ident),
        }
    }

    /// Consume a numeric literal (integer or float).
    fn consume_number(&mut self) -> Result<TokenKind, LexError> {
        let mut number_str = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() || ch == '.' {
                number_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match number_str.parse::<f64>() {
            Ok(n) => Ok(TokenKind::Number(n)),
            Err(_) => Err(LexError::InvalidNumber(
                number_str,
                self.line,
                self.column,
            )),
        }
    }

    /// Consume a string literal delimited by double quotes.
    fn consume_string(&mut self) -> Result<TokenKind, LexError> {
        // current_char is the opening quote
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip opening quote

        let mut s = String::new();
        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance(); // consume closing quote
                return Ok(TokenKind::StringLiteral(s));
            } else if ch == '\\' {
                // escape sequence
                self.advance();
                match self.current_char {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(other) => s.push(other),
                    None => return Err(LexError::UnterminatedString(start_line, start_col)),
                }
                self.advance();
            } else {
                s.push(ch);
                self.advance();
            }
        }

        Err(LexError::UnterminatedString(start_line, start_col))
    }

    /// Get the next token from the input.
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        let tok_start_line = self.line;
        let tok_start_col = self.column;

        let kind = match self.current_char {
            Some('(') => { self.advance(); TokenKind::LParen },
            Some(')') => { self.advance(); TokenKind::RParen },
            Some('{') => { self.advance(); TokenKind::LBrace },
            Some('}') => { self.advance(); TokenKind::RBrace },
            Some(',') => { self.advance(); TokenKind::Comma },
            Some(':') => { self.advance(); TokenKind::Colon },
            Some(';') => { self.advance(); TokenKind::Semicolon },

            Some('=') => { self.advance(); TokenKind::Assign },
            Some('+') => { self.advance(); TokenKind::Plus },
            Some('-') => { self.advance(); TokenKind::Minus },
            Some('*') => { self.advance(); TokenKind::Star },
            Some('/') => { self.advance(); TokenKind::Slash },
            Some('%') => { self.advance(); TokenKind::Percent },

            Some('"') => return Ok(Token::new(self.consume_string()?, tok_start_line, tok_start_col)),
            Some(ch) if ch.is_ascii_digit() => {
                let num = self.consume_number()?;
                TokenKind::Number(num)
            }
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                TokenKind::Identifier(self.consume_identifier())
            }
            None => TokenKind::Eof,
            Some(other) => return Err(LexError::UnexpectedChar(other, tok_start_line, tok_start_col)),
        };

        Ok(Token::new(kind, tok_start_line, tok_start_col))
    }

    /// Convenience method to tokenize the entire input into a Vec<Token>.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            if token.kind == TokenKind::Eof {
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("(foo) {bar, baz}");
        let tokens: Vec<_> = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::new(TokenKind::LParen, 1, 1),
                Token::new(
                    TokenKind::Identifier("foo".to_string()),
                    1,
                    2
                ),
                Token::new(TokenKind::RParen, 1, 5),
                Token::new(TokenKind::LBrace, 1, 7),
                Token::new(
                    TokenKind::Identifier("bar".to_string()),
                    1,
                    8
                ),
                Token::new(TokenKind::Comma, 1, 11),
                Token::new(
                    TokenKind::Identifier("baz".to_string()),
                    1,
                    13
                ),
            ]
        );
    }

    #[test]
    fn test_numbers_and_strings() {
        let mut lexer = Lexer::new(r#"123 45.67 "hello\nworld""#);
        let tokens: Vec<_> = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::new(TokenKind::Number(123.0), 1, 1),
                Token::new(TokenKind::Number(45.67), 1, 5),
                Token::new(
                    TokenKind::StringLiteral("hello\nworld".to_string()),
                    1,
                    11
                ),
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("if else while for in function");
        let tokens: Vec<_> = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::new(TokenKind::If, 1, 1),
                Token::new(TokenKind::Else, 1, 5),
                Token::new(TokenKind::While, 1, 10),
                Token::new(TokenKind::For, 1, 16),
                Token::new(TokenKind::In, 1, 20),
                Token::new(TokenKind::Function, 1, 23),
            ]
        );
    }

    #[test]
    fn test_comments_and_whitespace() {
        let mut lexer = Lexer::new(r#"
            // this is a comment
            # another comment
            x = 42;
        "#);
        let tokens: Vec<_> = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::new(
                    TokenKind::Identifier("x".to_string()),
                    5,
                    13
                ),
                Token::new(TokenKind::Assign, 5, 15),
                Token::new(TokenKind::Number(42.0), 5, 17),
                Token::new(TokenKind::Semicolon, 5, 19),
            ]
        );
    }

    #[test]
    fn test_unexpected_char_error() {
        let mut lexer = Lexer::new("@");
        let err = lexer.next_token().unwrap_err();
        assert_eq!(
            err,
            LexError::UnexpectedChar('@', 1, 1)
        );
    }
}