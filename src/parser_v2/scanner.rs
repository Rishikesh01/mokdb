use super::tokens::{ParsedLiteral, Token, Types};

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
                '(' => self.add_token(Types::OpenParen, "(".to_string(), None),
                ')' => self.add_token(Types::CloseParen, ")".to_string(), None),
                '*' => self.add_token(Types::AllColumnsOrMultiplication, "*".to_string(), None),
                ',' => self.add_token(Types::Comma, ",".to_string(), None),
                ';' => self.add_token(Types::Semicolon, ";".to_string(), None),
                '>' => self.handle_greater_relational_operator(),
                '<' => self.handle_lesser_relational_operator(),
                '=' => self.add_token(Types::EqualTo, "=".to_string(), None),
                '+' => self.add_token(Types::Addition, "+".to_string(), None),
                '-' => self.add_token(Types::Subtraction, "-".to_string(), None),
                '/' => self.add_token(Types::Division, "/".to_string(), None),
                '%' => self.add_token(Types::Modulus, "%".to_string(), None),
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
                    Types::Literal,
                    string_value.clone(),
                    Some(ParsedLiteral::Text(string_value)),
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
            Types::Literal,
            value.to_string(),
            Some(ParsedLiteral::Floating(value)),
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
            "SELECT" => Types::Select,
            "INSERT" => Types::Insert,
            "DELETE" => Types::Delete,
            "UPDATE" => Types::Update,
            "CREATE" => Types::Create,
            "DROP" => Types::Drop,
            "FROM" => Types::From,
            "WHERE" => Types::Where,
            "INTO" => Types::Into,
            "VALUES" => Types::Values,
            "TRUNCATE" => Types::Truncate,
            "RENAME" => Types::Rename,
            "ALTER" => Types::Alter,
            "ON" => Types::On,
            "ASC" => Types::AscendingOrder,
            "DESC" => Types::DecendingOrder,
            "SET" => Types::Set,
            "COMMIT" => Types::Commit,
            "ROLLBACK" => Types::RollBack,
            "TABLE" => Types::Table,
            "AND" => Types::And,
            "NOT" => Types::Not,
            "NULL" => Types::Null,
            "IS" => Types::Is,
            "ADD" => Types::Add,
            "CONSTRAINT" => Types::Constraint,
            "OR" => Types::Or,
            "SCHEMA" => Types::Schema,
            "DISTINCT" => Types::Distinct,
            "IN" => Types::In,
            "INTEGER" => Types::Insert,
            "TEXT" => Types::Text,
            "Decimal" => Types::Decimal,
            "BOOLEAN" => Types::Boolean,
            "PRIMARY" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let token_part = "KEY";
                match self.source[self.current..self.current + token_part.len()]
                    .to_string()
                    .to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::PrimaryKey
                    }
                    false => Types::Invalid,
                }
            }
            "UNIQUE" => {
                let token_part = "KEY";
                match self.source[self.current..self.current + token_part.len()]
                    .to_string()
                    .to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::PrimaryKey
                    }
                    false => Types::Invalid,
                }
            }
            "FOREGIN" => {
                let token_part = "KEY";
                match self.source[self.current..self.current + token_part.len()]
                    .to_string()
                    .to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::ForeginKey
                    }
                    false => Types::Invalid,
                }
            }

            "LEFT" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '\n' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let token_part = "JOIN";
                match self.source[self.current..self.current + token_part.len()].to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::LeftJoin
                    }
                    false => Types::Invalid,
                }
            }
            "RIGHT" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '\n' {
                        self.advance();
                    } else {
                        break;
                    }
                }

                let token_part = "JOIN";
                match self.source[self.current..self.current + token_part.len()].to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::LeftJoin
                    }
                    false => Types::Invalid,
                }
            }
            "BEGIN" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '\n' {
                        self.advance();
                    } else {
                        break;
                    }
                }

                let token_part = "TRANSACTION";
                match self.source[self.current..self.current + token_part.len()].to_uppercase()
                    == token_part
                {
                    true => Types::BeginTransaction,
                    false => Types::Invalid,
                }
            }
            "FULL" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '\n' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let token_part_1 = "OUTER";
                match self.source[self.current..self.current + token_part_1.len()].to_uppercase()
                    == token_part_1
                {
                    true => {
                        self.current += token_part_1.len();
                        while let Some(c) = self.peek() {
                            if c.is_whitespace() || c == '\n' {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let token_part_2 = "JOIN";
                        match self.source[self.current..self.current + token_part_2.len()]
                            .to_uppercase()
                            == token_part_2
                        {
                            true => {
                                self.current += token_part_2.len();
                                Types::FullOuterJoin
                            }
                            false => Types::Invalid,
                        }
                    }
                    false => Types::Invalid,
                }
            }
            "ORDER" => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '\n' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let token_part = "BY";
                match self.source[self.current..self.current + token_part.len()].to_uppercase()
                    == token_part
                {
                    true => {
                        self.current += token_part.len();
                        Types::OrderBy
                    }
                    false => Types::Invalid,
                }
            }
            _ => Types::Identifier,
        };

        self.add_token(token_type, text.clone(), None);
    }

    fn handle_greater_relational_operator(&mut self) {
        if let Some('=') = self.peek() {
            self.advance();
            self.add_token(Types::GreaterThanOrEqualTo, ">=".to_string(), None);
        } else {
            self.add_token(Types::GreaterThan, ">".to_string(), None);
        }
    }

    fn handle_lesser_relational_operator(&mut self) {
        if let Some('=') = self.peek() {
            self.advance();
            self.add_token(Types::LessThanOrEqualTo, "<=".to_string(), None);
        } else if let Some('>') = self.peek() {
            self.advance();
            self.add_token(Types::NotEqualTo, "<>".to_string(), None);
        } else {
            self.add_token(Types::LessThan, "<".to_string(), None);
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

    fn add_token(&mut self, token_type: Types, lexeme: String, literal: Option<ParsedLiteral>) {
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
