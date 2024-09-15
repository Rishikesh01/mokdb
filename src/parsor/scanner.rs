use std::any::Any;

use super::{sql_token_types::SQLTokenTypes, token::Token};

#[derive(Debug)]
pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: i64,
    tokens: Vec<Token>,
}

impl Scanner {
    fn new(self, source: String) -> Self {
        Self {
            source: source,
            current: 0,
            start: 0,
            line: 1,
            tokens: Vec::new(),
        }
    }
    fn scan_tokens(&mut self) -> &Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current
        }
        self.tokens.push(Token {
            token_type: SQLTokenTypes::EOF,
            lexeme: "".to_string(),
            literal: None,
        });

        return &self.tokens;
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            '(' => self.add_token(SQLTokenTypes::LEFTPAREN, None),
            ')' => self.add_token(SQLTokenTypes::RIGHTPAREN, None),
            '*' => self.add_token(SQLTokenTypes::STAR, None),
            ',' => self.add_token(SQLTokenTypes::COMMA, None),
            ';' => self.add_token(SQLTokenTypes::SEMICOLON, None),
            '>' => self.add_token(SQLTokenTypes::GREATER, None),
            '<' => self.add_token(SQLTokenTypes::LESSER, None),
            '=' => self.add_token(SQLTokenTypes::EQUAL, None),
            '\n' => self.line += 1,

            _ => {}
        }
    }

    fn advance(&mut self) -> char {
        let c = self.source.chars().nth(self.current).unwrap();
        self.current += 1;
        return c;
    }

    fn is_at_end(&self) -> bool {
        return self.current >= self.source.len();
    }

    fn peek(self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        return self.source.chars().nth(self.current).unwrap();
    }

    fn add_token(&mut self, sql_token_type: SQLTokenTypes, literal: Option<Box<dyn Any>>) {
        self.tokens.push(Token {
            token_type: sql_token_type,
            lexeme: self.source[self.start..self.current].to_string(),
            literal,
        })
    }
}

