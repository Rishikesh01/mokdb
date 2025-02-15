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

    fn scan_token(&mut self) -> Result<(), String> {
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
                '\'' => self.handle_string()?,
                '\n' => {
                    self.line += 1;
                    self.column = 0;
                }
                _ if c.is_whitespace() => {}
                _ if c.is_numeric() => self.handle_numeric()?,
                _ if c.is_alphanumeric() => self.handle_alpha_numeric()?,
                _ => {
                    return Err(format!(
                        "Unexpected character: '{}'\n line number: {}, column: {}",
                        c, self.line, self.column
                    ))
                }
            }
        }
        Ok(())
    }

    fn handle_string(&mut self) -> Result<(), String> {
        while let Some(c) = self.peek() {
            if c == '\'' {
                self.advance();
                let string_value: String = self.source[self.start..self.current - 1].to_string();
                self.add_token(
                    Types::Literal,
                    string_value.clone(),
                    Some(ParsedLiteral::Text(string_value)),
                );
                return Ok(());
            } else if c == '\\' {
                self.advance();
                if let Some(escaped_char) = self.peek() {
                    match escaped_char {
                        '\'' | '\\' | 'n' | 't' => {
                            self.advance();
                        }
                        _ => {
                            return Err(format!(
                                "Invalid escape sequence: \\{}\n at line number: {}, column: {}",
                                escaped_char, self.line, self.column
                            ))
                        }
                    }
                }
            } else {
                self.advance();
            }
        }

        Err(format!(
            "Unterminating string at line number: {}, column: {}",
            self.line, self.column
        ))
    }

    fn handle_numeric(&mut self) -> Result<(), String> {
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
                return Err(format!(
                    "error: Invalid floating point number '{}'\n at line number: {}, column: {}",
                    &self.source[self.start..self.current],
                    self.line,
                    self.column
                ));
            }
        }

        // Successfully parse the number (either integer or floating point)
        let value: f64 = self.source[self.start..self.current].parse().map_err(|_| {
            format!(
                "Could not parse number '{}'\n at line number: {}, column {}",
                &self.source[self.start..self.current],
                self.line,
                self.column,
            )
        })?;
        self.add_token(
            Types::Literal,
            value.to_string(),
            Some(ParsedLiteral::Decimal(value)),
        );
        Ok(())
    }

    fn handle_alpha_numeric(&mut self) -> Result<(), String> {
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
                    false => {
                        return Err(format!(
                            "Error on line {}: Expected 'KEY' after 'PRIMARY' but found '{}'.",
                            self.line,
                            &self.source[self.current..]
                        ));
                    }
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
                    false => {
                        return Err(format!(
                            "Error on line {}: Expected 'KEY' after 'UNIQUE' but found '{}'.",
                            self.line,
                            &self.source[self.current..]
                        ));
                    }
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
                    false => {
                        return Err(format!(
                            "Error on line {}: Expected 'KEY' after 'FOREIGN' but found '{}'.",
                            self.line,
                            &self.source[self.current..]
                        ));
                    }
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
                    false => {
                        return Err(format!(
                        "expected 'JOIN' after 'LEFT' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                    }
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
                    false => {
                        return Err(format!(
                        "expected 'JOIN' after 'RIGHT' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                    }
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
                    false => {
                        return Err(format!(
                        "expected 'TRANSACTION' after 'BEGIN' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                    }
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
                            false => {
                                return Err(format!(
                        "expected 'JOIN' after 'OUTER' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                            }
                        }
                    }
                    false => {
                        return Err(format!(
                        "expected 'OUTER' after 'FULL' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                    }
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
                    false => {
                        return Err(format!(
                        "expected 'BY' after 'ORDER' but found '{}'\n line number: {}, column:{}",
                        self.line,
                        self.column,
                        &self.source[self.current..]
                    ))
                    }
                }
            }
            _ => Types::Identifier,
        };

        self.add_token(token_type, text.clone(), None);
        Ok(())
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
    fn tokenize(self) -> Result<Vec<Token>, String>;
}

impl SQLInput for Scanner {
    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        while !self.is_at_end() {
            self.scan_token()?;
        }
        Ok(self.tokens)
    }
}
