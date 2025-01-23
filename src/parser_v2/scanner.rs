use super::tokens::{SQLTokenType, Token};
use std::any::Any;

struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
            column: 0,
            tokens: Vec::new(),
        }
    }

    fn scan_token(&mut self) {
        self.start = self.current;
        let c = self.advance();

        if let Some(c) = c {
            match c {
                '(' => self.add_token(SQLTokenType::Leftparen, "(".to_string(), None),
                ')' => self.add_token(SQLTokenType::Rightparen, ")".to_string(), None),
                '*' => self.add_token(SQLTokenType::Star, "*".to_string(), None),
                ',' => self.add_token(SQLTokenType::Comma, ",".to_string(), None),
                ';' => self.add_token(SQLTokenType::Semicolon, ";".to_string(), None),
                '>' => self.handle_greater_relational_operator(),
                '<' => self.handle_lesser_relational_operator(),
                '=' => self.add_token(SQLTokenType::Equal, "=".to_string(), None),
                '\'' => self.handle_string(),
                '\n' => {
                    self.line += 1;
                    self.column = 0;
                }
                _ if c.is_numeric() => self.handle_numeric(),
                _ if c.is_alphanumeric() => self.handle_alpha_numeric(),
                _ => {}
            }
        }
    }

    fn handle_string(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\'' {
                self.advance();
                let string_value: String = self.source[self.start..self.current - 1].to_string();
                self.add_token(
                    SQLTokenType::String,
                    string_value.clone(),
                    Some(Box::new(string_value)),
                );
                return;
            } else if c == '\\' {
                self.advance();
                if let Some(escaped_char) = self.peek() {
                    match escaped_char {
                        '\'' | '\\' | 'n' | 't' => {
                            self.advance();
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
            } else {
                self.advance();
            }
        }
    }

    fn handle_numeric(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        if let Some('.') = self.peek() {
            self.advance(); // Consume the dot
            let mut has_digits_after_dot = false;

            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    has_digits_after_dot = true;
                    self.advance();
                } else {
                    has_digits_after_dot = false;
                    break;
                }
            }

            if !has_digits_after_dot {
                println!(
                    "Error: Invalid floating point number '{}'.",
                    &self.source[self.start..self.current]
                );
                return;
            }
        }

        // Successfully parse the number (either integer or floating point)
        let value: f64 = self.source[self.start..self.current].parse().unwrap();
        self.add_token(
            SQLTokenType::Number,
            value.to_string(),
            Some(Box::new(value)),
        );
    }

    fn handle_alpha_numeric(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = self.source[self.start..self.current]
            .to_string()
            .to_uppercase();
        let token_type = match text.as_str() {
            "SELECT" => SQLTokenType::Select,
            "INSERT" => SQLTokenType::Insert,
            "DELETE" => SQLTokenType::Delete,
            "UPDATE" => SQLTokenType::Update,
            "CREATE" => SQLTokenType::Create,
            "DROP" => SQLTokenType::Drop,
            "FROM" => SQLTokenType::From,
            "WHERE" => SQLTokenType::Where,
            "INTO" => SQLTokenType::Into,
            "VALUES" => SQLTokenType::Values,
            "TRUNCATE" => SQLTokenType::Truncate,
            "RENAME" => SQLTokenType::Rename,
            "ALTER" => SQLTokenType::Alter,
            "SET" => SQLTokenType::Set,
            "COMMIT" => SQLTokenType::Commit,
            "ROLLBACK" => SQLTokenType::Rollback,
            "SAVEPOINT" => SQLTokenType::Savepoint,
            "TABLE" => SQLTokenType::Table,
            "PRIMARY" => SQLTokenType::Primary,
            "KEY" => SQLTokenType::Key,
            "UNIQUE" => SQLTokenType::Unique,
            "AND" => SQLTokenType::And,
            "NOT" => SQLTokenType::Not,
            "NULL" => SQLTokenType::Null,
            "IS" => SQLTokenType::IS,
            "OR" => SQLTokenType::OR,
            _ => SQLTokenType::Identifier,
        };

        self.add_token(token_type, text.clone(), Some(Box::new(text)));
    }

    fn handle_greater_relational_operator(&mut self) {
        if let Some('=') = self.peek() {
            self.advance();
            self.add_token(SQLTokenType::GreaterThanOrEqualTo, ">=".to_string(), None);
        } else {
            self.add_token(SQLTokenType::Greater, ">".to_string(), None);
        }
    }

    fn handle_lesser_relational_operator(&mut self) {
        if let Some('=') = self.peek() {
            self.advance();
            self.add_token(SQLTokenType::LesserThanOrEqualTo, "<=".to_string(), None);
        } else if let Some('>') = self.peek() {
            self.advance();
            self.add_token(SQLTokenType::NotEqual, "<>".to_string(), None);
        } else {
            self.add_token(SQLTokenType::Lesser, "<".to_string(), None);
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.source[self.current..].chars().next();
        if let Some(ch) = c {
            self.current += ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        c
    }

    fn peek(&self) -> Option<char> {
        self.source[self.current..].chars().next()
    }

    fn add_token(
        &mut self,
        token_type: SQLTokenType,
        lexeme: String,
        literal: Option<Box<dyn Any>>,
    ) {
        let value = Token::new(token_type, lexeme, literal, self.line, self.column);
        self.tokens.push(value);
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}

pub trait SQLInput {
    fn tokenize(self) -> Vec<Token>;
}

impl SQLInput for Scanner {
    fn tokenize(mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.scan_token();
        }
        self.tokens
    }
}
