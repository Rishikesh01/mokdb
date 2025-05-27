use super::tokens::{LiteralValue, Syntax::*, Token};

pub struct Lexer {
    input: String,
    line_no: usize,
    column_no: usize,
    current_pos: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input,
            line_no: 1,
            column_no: 0,
            current_pos: 0,
        }
    }

    pub fn scan(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.current_pos >= self.input.len() {
            return None;
        }

        let mut chars = self.input[self.current_pos..].chars();
        let first = chars.next().unwrap();

        if let Some(second) = chars.next() {
            match (first, second) {
                ('!', '=') => {
                    self.advance();
                    self.advance();
                    return Some(Token::new(
                        Neq,
                        "!=".to_string(),
                        None,
                        self.line_no,
                        self.column_no,
                    ));
                }
                ('<', '=') => {
                    self.advance();
                    self.advance();
                    return Some(Token::new(
                        Lte,
                        "<=".to_string(),
                        None,
                        self.line_no,
                        self.column_no,
                    ));
                }
                ('>', '=') => {
                    self.advance();
                    self.advance();
                    return Some(Token::new(
                        Gte,
                        ">=".to_string(),
                        None,
                        self.line_no,
                        self.column_no,
                    ));
                }
                _ => {}
            }
        }

        if let Some(tok) = self.match_token(first) {
            return Some(tok);
        }

        if first.is_ascii_alphabetic() || first == '_' || first == '"' {
            self.advance();
            return Some(self.scan_identifier_or_keyword(first));
        }

        if first.is_ascii_digit() {
            self.advance();
            return Some(self.scan_number(first));
        }

        if first == '\'' {
            self.advance();
            return Some(self.scan_string());
        }

        self.advance();
        Some(Token::new(
            Identifier,
            first.to_string(),
            None,
            self.line_no,
            self.column_no,
        ))
    }

    fn match_token(&mut self, c: char) -> Option<Token> {
        let kind = match c {
            '=' => Eq,
            '>' => Gt,
            '<' => Lt,
            '+' => Plus,
            '-' => Minus,
            '*' => Star,
            '/' => Slash,
            '%' => Percent,
            ',' => Comma,
            ';' => Semicolon,
            '(' => OpenParen,
            ')' => CloseParen,
            _ => return None,
        };
        self.advance();
        Some(Token::new(
            kind,
            c.to_string(),
            None,
            self.line_no,
            self.column_no,
        ))
    }

    fn scan_identifier_or_keyword(&mut self, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);

        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '"' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let upper_ident = ident.to_ascii_uppercase();

        // Map keywords to Token kind
        let kind = match upper_ident.as_str() {
            "TRUE" | "FALSE" => Literal, // Boolean literals
            "ASC" => Asc,
            "DESC" => Desc,
            "SELECT" => Select,
            "INSERT" => Insert,
            "UPDATE" => Update,
            "DELETE" => Delete,
            "DROP" => Drop,
            "WITH" => With,
            "CREATE" => Create,
            "FROM" => From,
            "WHERE" => Where,
            "GROUP" => Group,
            "ORDER" => Order,
            "BY" => By,
            "LIMIT" => Limit,
            "OFFSET" => Offset,
            "DISTINCT" => Distinct,
            "ALL" => All,
            "ON" => On,
            "AND" => And,
            "OR" => Or,
            "NOT" => Not,
            "IN" => In,
            "EXISTS" => Exists,
            "IS" => Is,
            "NULL" => Null,
            "LIKE" => Like,
            "ILIKE" => ILike,
            "PRIMARY" => Primary,
            "KEY" => Key,
            "FOREIGN" => Foreign,
            "UNIQUE" => Unique,
            "AS" => As,
            "VALUES" => Values,
            "RETURNING" => Returning,
            "JOIN" => Join,
            "LEFT" => Left,
            "RIGHT" => Right,
            "INNER" => Inner,
            "FULL" => Full,
            "INTO" => Into,
            "SET" => Set,
            "TEXT" => Text,
            "INT" => Int,
            "FLOAT" => Float,
            "BOOLEAN" => Boolean,
            "DEFAULT" => Default,
            "CONSTRAINT" => Constraint,
            "REFERENCES" => References,
            "TABLE" => Table,
            "BEGIN" => Begin,
            "COMMIT" => Commit,
            _ => Identifier,
        };

        // Map boolean literals to LiteralValue
        let literal = match upper_ident.as_str() {
            "TRUE" => Some(LiteralValue::Boolean(true)),
            "FALSE" => Some(LiteralValue::Boolean(false)),
            _ => None,
        };

        // Remove quotes if it's an identifier
        if kind == Identifier {
            ident = ident.replace("\"", "");
        }

        Token::new(kind, ident, literal, self.line_no, self.column_no)
    }

    fn scan_number(&mut self, first: char) -> Token {
        let mut num = String::new();
        num.push(first);

        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                // Decimal point detected
                is_float = true;
                num.push(c);
                self.advance();
            } else if (c == 'e' || c == 'E') && !is_float {
                // Exponential part
                is_float = true;
                num.push(c);
                self.advance();
                // Optional + or - after exponent
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        num.push(sign);
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        // Try parsing into number types
        let literal = if is_float {
            num.parse::<f64>().map(LiteralValue::Float).ok()
        } else {
            num.parse::<i64>().map(LiteralValue::Int).ok()
        };

        Token::new(Literal, num, literal, self.line_no, self.column_no)
    }

    fn scan_string(&mut self) -> Token {
        let mut value = String::new();

        while let Some(c) = self.peek() {
            if c == '\'' {
                self.advance();
                break;
            } else {
                value.push(c);
                self.advance();
            }
        }

        Token::new(
            Literal,
            value.clone(),
            Some(LiteralValue::Text(value.clone())),
            self.line_no,
            self.column_no,
        )
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.current_pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input[self.current_pos..].chars().next()?;
        self.current_pos += c.len_utf8();

        if c == '\n' {
            self.line_no += 1;
            self.column_no = 0;
        } else {
            self.column_no += 1;
        }

        Some(c)
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.scan()
    }
}
