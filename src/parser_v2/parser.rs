use std::fmt::format;

use super::{
    ast::{Condition, SQLStatement, SelectColumn, SelectStatement},
    tokens::{SQLTokenType, Token},
};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_and_build_ast(mut self) -> Result<SQLStatement, String> {
        match self.peek().token_type {
            SQLTokenType::Select => self.select_statement(),
            SQLTokenType::Insert => self.insert_statement(),
            SQLTokenType::Update => self.update_statement(),
            SQLTokenType::Delete => self.delete_statement(),
            SQLTokenType::Create => self.create_statement(),
            SQLTokenType::Drop => self.drop_statement(),
            _ => Err("Unexpected statement type".to_string()),
        }
    }

    fn peek(&mut self) -> &Token {
        return &self.tokens[self.current];
    }

    fn check(&mut self, token_type: SQLTokenType) -> bool {
        return self.peek().token_type == token_type;
    }

    fn is_at_end(&mut self) -> bool {
        self.tokens.len() >= self.current
    }

    fn consume(&mut self, token_type: SQLTokenType, message: &str) -> Result<&Token, String> {
        if self.check(token_type) {
            self.current += 1;
            Ok(&self.tokens[self.current - 1])
        } else {
            Err(format!(
                "error at line and column: {}:{}\nerror message: {}",
                &self.tokens[self.current].line, &self.tokens[self.current].column, message
            ))
        }
    }
    /*
     * We will allow SELECT statements of these types
     * 1. SELECT (somecolumn,...|| *) FROM (sometablename) WHERE (Condition) ORDERY BY (columne name);
     * In future iteration we will add distinct
     */
    fn select_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Select, "expected select key word")?;
        let mut columns = Vec::new();
        loop {
            if self.check(SQLTokenType::Star) {
                self.consume(SQLTokenType::Star, "expected *")?;
                columns.push(SelectColumn::All);
                break;
            } else if self.check(SQLTokenType::Identifier) {
                columns.push(SelectColumn::Column(
                    self.consume(
                        SQLTokenType::Identifier,
                        "expected an identifier for column",
                    )?
                    .lexeme
                    .clone(),
                ));
            } else {
                return Err("Expected column name or *".to_string());
            }
            if !self.check(SQLTokenType::Comma) {
                break;
            }
        }

        self.consume(SQLTokenType::From, "expected from keyword")?;
        let table_name = self
            .consume(
                SQLTokenType::Identifier,
                "expected Identifier for table name",
            )?
            .lexeme
            .to_string();

        Ok(SQLStatement::Select(SelectStatement {
            columns,
            from: table_name,
            where_clause: None,
            order_by: None,
        }))
    }
    /*
     * We will handle the following conditions below
     * 1. WHERE somecolumn = || != someliteral
     * 2. WHERE somecolumn = || != someliteral AND somecolumn = || != someliteral
     * 3. WHERE somecolumn = || != someliteral or somecolumn = || != someliteral
     * 4. WHERE somecolumn IN (select query|| array) AND somecolumn = || != someliteral
     * We will also construct Condition AST based on operator precedent
     * Example:
     *   1. WHERE somecolumn = || != someliteral OR somecolumn = || != someliteral AND somecolumn = || != someliteral
     *   The above query would look like this with parenthesis:
     *       i. WHERE  somecolumn = || != someliteral OR (somecolumn = || != someliteral AND somecolumn = || != someliteral)
     * User should also be able to override this via providing his own parenthesis
     */
    fn handle_where_clause(&mut self) -> Result<Condition, String> {
        Ok(Condition())
    }

    fn insert_statement(&self) -> Result<SQLStatement, String> {
        todo!()
    }

    fn update_statement(&self) -> Result<SQLStatement, String> {
        todo!()
    }

    fn delete_statement(&self) -> Result<SQLStatement, String> {
        todo!()
    }

    fn create_statement(&self) -> Result<SQLStatement, String> {
        todo!()
    }

    fn drop_statement(&self) -> Result<SQLStatement, String> {
        todo!()
    }
}
