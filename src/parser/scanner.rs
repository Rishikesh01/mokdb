use std::{any::Any, char};

use super::{sql_token_types::SQLTokenTypes, token::Token};

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: i64,
    tokens: Vec<Token>,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Self {
            source,
            current: 0,
            start: 0,
            line: 1,
            tokens: Vec::new(),
        }
    }
    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }
        self.tokens.push(Token {
            token_type: SQLTokenTypes::Eof,
            lexeme: "".to_string(),
            literal: None,
        });
        return std::mem::take(&mut self.tokens);
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            '(' => self.add_token(SQLTokenTypes::Leftparen, None),
            ')' => self.add_token(SQLTokenTypes::Rightparen, None),
            '*' => self.add_token(SQLTokenTypes::Star, None),
            ',' => self.add_token(SQLTokenTypes::Comma, None),
            ';' => self.add_token(SQLTokenTypes::Semicolon, None),
            '>' => self.handle_greater_relational_operator(),
            '<' => self.handle_lesser_relational_operator(),
            '=' => self.add_token(SQLTokenTypes::Equal, None),
            '\'' => self.handle_string(),
            '\n' => self.line += 1,
            _ if c.is_numeric() => self.handle_numberic(),
            _ if c.is_alphanumeric() => self.handle_alpha_numeric(),
            _ => {}
        }
    }

    fn handle_greater_relational_operator(&mut self) {
        if self.peek() == '=' {
            self.advance();
            self.add_token(SQLTokenTypes::GreaterThanOrEqualTo, None);
        } else {
            self.add_token(SQLTokenTypes::Greater, None);
        }
    }

    fn handle_lesser_relational_operator(&mut self) {
        if self.peek() == '=' {
            self.advance();
            self.add_token(SQLTokenTypes::LesserThanOrEqualTo, None);
        } else if self.peek() == '>' {
            self.advance();
            self.add_token(SQLTokenTypes::NotEqual, None);
        } else {
            self.add_token(SQLTokenTypes::Lesser, None);
        }
    }

    fn handle_string(&mut self) {
        while self.peek() != '\'' {
            self.advance();
        }
        self.advance();
        let string_value = self.source[self.start..self.current].to_string();
        self.add_token(SQLTokenTypes::String, Some(Box::new(string_value)));
    }

    fn handle_numberic(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let value: f64 = self.source[self.start..self.current].parse().unwrap();
        self.add_token(SQLTokenTypes::Number, Some(Box::new(value)));
    }

    fn handle_alpha_numeric(&mut self) {
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text = &self.source[self.start..self.current].to_uppercase();
        let token_type = match text.as_str() {
            "SELECT" => SQLTokenTypes::Select,
            "INSERT" => SQLTokenTypes::Insert,
            "DELETE" => SQLTokenTypes::Delete,
            "UPDATE" => SQLTokenTypes::Update,
            "CREATE" => SQLTokenTypes::Create,
            "DROP" => SQLTokenTypes::Drop,
            "FROM" => SQLTokenTypes::From,
            "WHERE" => SQLTokenTypes::Where,
            "INTO" => SQLTokenTypes::Into,
            "VALUES" => SQLTokenTypes::Values,
            "TRUNCATE" => SQLTokenTypes::Truncate,
            "RENAME" => SQLTokenTypes::Rename,
            "ALTER" => SQLTokenTypes::Alter,
            "SET" => SQLTokenTypes::Set,
            "COMMIT" => SQLTokenTypes::Commit,
            "ROLLBACK" => SQLTokenTypes::Rollback,
            "SAVEPOINT" => SQLTokenTypes::Savepoint,
            "TABLE" => SQLTokenTypes::Table,
            "PRIMARY" => SQLTokenTypes::Primary,
            "KEY" => SQLTokenTypes::Key,
            "UNIQUE" => SQLTokenTypes::Unique,
            "AND" => SQLTokenTypes::And,
            "NOT" => SQLTokenTypes::Not,
            "NULL" => SQLTokenTypes::Null,
            "IS" => SQLTokenTypes::IS,
            "OR" => SQLTokenTypes::OR,
            _ => SQLTokenTypes::Identifier,
        };

        self.add_token(token_type, None);
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.current..].chars().next().unwrap();
        self.current += 1;
        return c;
    }

    fn is_at_end(&self) -> bool {
        return self.current >= self.source.len();
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source.chars().nth(self.current + 1).unwrap()
        }
    }

    fn peek(&mut self) -> char {
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
