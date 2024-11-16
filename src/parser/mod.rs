#![allow(dead_code, clippy::needless_return)]
use self::{
    ast::{
        Assignment, ColumnConstraint, ColumnDefinition, ComparisonCondition, ComparisonOperator,
        Condition, CreateStatement, DataType, DropStatement, Expression, InsertStatement, Literal,
        LogicalCondition, LogicalOperator, NullCheckCondition, SQLStatement, SelectColumn,
        SelectStatement, WhereClause,
    },
    scanner::Scanner,
    sql_token_types::SQLTokenTypes,
    token::Token,
};

pub mod ast;
pub mod scanner;
pub mod sql_token_types;
pub mod token;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(source: String) -> Self {
        let mut scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<SQLStatement, String> {
        match self.peek().token_type {
            SQLTokenTypes::Select => self.select_statement(),
            SQLTokenTypes::Insert => self.insert_statement(),
            SQLTokenTypes::Update => self.update_statement(),
            SQLTokenTypes::Delete => self.delete_statement(),
            SQLTokenTypes::Create => self.create_statement(),
            SQLTokenTypes::Drop => self.drop_statement(),
            _ => Err("Unexpected statement type".to_string()),
        }
    }

    fn select_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Select, "expected select keyword")?;
        let mut columns = Vec::new();
        loop {
            if self.check(SQLTokenTypes::Star) {
                self.consume(SQLTokenTypes::Star, "expected *")?;
                columns.push(SelectColumn::All);
                break;
            } else if self.check(SQLTokenTypes::Identifier) {
                columns.push(SelectColumn::Column(self.advance().lexeme.clone()));
            } else {
                return Err("Expected column name or *".to_string());
            }

            if !self.match_token(SQLTokenTypes::Comma) {
                break;
            }
        }

        self.consume(SQLTokenTypes::From, "Expect FROM after select columns")?;
        let from = if self.check(SQLTokenTypes::Identifier) {
            Some(self.advance().lexeme.clone())
        } else {
            return Err("Expected table name after FROM".to_string());
        };

        let where_clause = if self.match_token(SQLTokenTypes::Where) {
            Some(self.where_clause()?)
        } else {
            None
        };

        Ok(SQLStatement::Select(SelectStatement {
            columns,
            from,
            where_clause,
        }))
    }

    fn insert_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Insert, "Expect INSERT")?;
        self.consume(SQLTokenTypes::Into, "Expect INTO after INSERT")?;
        let table = self
            .consume(SQLTokenTypes::Identifier, "Expect table name")?
            .lexeme
            .clone();

        let columns = if self.match_token(SQLTokenTypes::Leftparen) {
            self.parse_column_list()?
        } else {
            Vec::new()
        };

        self.consume(SQLTokenTypes::Values, "Expect VALUES")?;
        self.consume(SQLTokenTypes::Leftparen, "Expect ( after VALUES")?;
        let values = self.parse_expression_list()?;
        self.consume(SQLTokenTypes::Rightparen, "Expect ) after values")?;

        Ok(SQLStatement::Insert(InsertStatement {
            table,
            columns,
            values,
        }))
    }

    fn update_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Update, "Expect UPDATE")?;
        let table = self
            .consume(SQLTokenTypes::Identifier, "Expect table name")?
            .lexeme
            .clone();
        self.consume(SQLTokenTypes::Set, "Expect SET after table name")?;

        let assignments = self.parse_assignments()?;

        let where_clause = match self.match_token(SQLTokenTypes::Where) {
            true => Some(self.where_clause()?),
            false => None,
        };

        Ok(SQLStatement::Update(ast::UpdateStatement {
            table,
            assignments,
            where_clause,
        }))
    }

    fn delete_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Delete, "Expect DELETE")?;
        self.consume(SQLTokenTypes::From, "Expect FROM after DELETE")?;
        let table = self
            .consume(SQLTokenTypes::Identifier, "Expect table name")?
            .lexeme
            .clone();

        let where_clause = if self.match_token(SQLTokenTypes::Where) {
            Some(self.where_clause()?)
        } else {
            None
        };

        Ok(SQLStatement::Delete(ast::DeleteStatement {
            table,
            where_clause,
        }))
    }

    fn create_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Create, "Expect CREATE")?;
        self.consume(SQLTokenTypes::Table, "Expect TABLE after CREATE")?;
        let table = self
            .consume(SQLTokenTypes::Identifier, "Expect table name")?
            .lexeme
            .clone();

        self.consume(SQLTokenTypes::Leftparen, "Expect ( after table name")?;
        let columns = self.parse_column_definitions()?;
        self.consume(
            SQLTokenTypes::Rightparen,
            "Expect ) after column definitions",
        )?;

        Ok(SQLStatement::Create(CreateStatement { table, columns }))
    }

    fn drop_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::Drop, "Expect DROP")?;
        self.consume(SQLTokenTypes::Table, "Expect TABLE after DROP")?;
        let table = self
            .consume(SQLTokenTypes::Identifier, "Expect table name")?
            .lexeme
            .clone();

        Ok(SQLStatement::Drop(DropStatement { table }))
    }

    // The entry point for parsing the WHERE clause
    // WHERE foo = 'bar'
    // WHERE foo = 'bar' AND fuzz = 'fuzz0'
    // WHERE (foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')
    // WHERE (foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')
    // WHERE ((foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')) AND (IS_ACTIVE AND IS_ENABLED))
    // WHERE ((foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')) AND (IS_ACTIVE = FALSE AND IS_ENABLED))
    // WHERE IS_ACTIVE = FALSE AND IS_ENABLED
    // WHERE foo = 'bar' AND IS_ACTIVE
    fn where_clause(&mut self) -> Result<WhereClause, String> {
        let condition = self.parse_or_condition()?;
        Ok(WhereClause { condition })
    }

    fn parse_or_condition(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_and_condition()?;
        while self.check(SQLTokenTypes::OR) {
            self.consume(SQLTokenTypes::OR, "Expected 'OR' operator")?;
            let right = self.parse_and_condition()?;
            left = Condition::Logical(LogicalCondition {
                operator: LogicalOperator::Or,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_and_condition(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_primary_condition()?;

        while self.check(SQLTokenTypes::And) {
            self.consume(SQLTokenTypes::And, "Expected 'AND' operator")?;
            let right = self.parse_primary_condition()?;
            left = Condition::Logical(LogicalCondition {
                operator: LogicalOperator::And,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_primary_condition(&mut self) -> Result<Condition, String> {
        if self.check(SQLTokenTypes::Not) {
            // Handle NOT operator
            self.consume(SQLTokenTypes::Not, "Expected 'NOT' operator")?;
            let condition = self.parse_primary_condition()?; // Recursively parse the condition after NOT
            return Ok(Condition::Not(Box::new(condition)));
        }

        if self.check(SQLTokenTypes::Leftparen) {
            // Handle grouped conditions or subqueries.
            self.consume(SQLTokenTypes::Leftparen, "Expected '('")?;
            let condition = self.parse_or_condition()?;
            self.consume(SQLTokenTypes::Rightparen, "Expected ')'")?;
            return Ok(condition);
        }

        self.parse_comparison_condition()
    }

    fn parse_comparison_condition(&mut self) -> Result<Condition, String> {
        if self.check(SQLTokenTypes::Identifier) {
            let left = self.peek().lexeme.clone();
            self.consume(SQLTokenTypes::Identifier, "expected an identifier")?;

            if self.check(SQLTokenTypes::Equal)
                || self.check(SQLTokenTypes::GreaterThanOrEqualTo)
                || self.check(SQLTokenTypes::LesserThanOrEqualTo)
                || self.check(SQLTokenTypes::Lesser)
                || self.check(SQLTokenTypes::Greater)
            {
                let operator = match self.peek().token_type {
                    SQLTokenTypes::NotEqual => ComparisonOperator::NotEqual,
                    SQLTokenTypes::Equal => ComparisonOperator::Equal,
                    SQLTokenTypes::GreaterThanOrEqualTo => ComparisonOperator::GreaterThanOrEqual,
                    SQLTokenTypes::LesserThanOrEqualTo => ComparisonOperator::LessThanOrEqual,
                    SQLTokenTypes::Lesser => ComparisonOperator::LessThan,
                    SQLTokenTypes::Greater => ComparisonOperator::GreaterThan,
                    _ => {
                        return Err(
                            "unexpected token found, expected comparison operator".to_string()
                        )
                    }
                };

                self.consume(
                    self.peek().token_type.clone(),
                    "expected comparison operator",
                )?;

                // Ensure the right-hand side is a valid literal (string, number, or boolean).
                let right = self.expression()?;

                return Ok(Condition::Comparison(ComparisonCondition {
                    operator,
                    left: Expression::Identifier(left),
                    right,
                }));
            } else if self.check(SQLTokenTypes::Null)
                || self.check(SQLTokenTypes::IS)
                || self.check(SQLTokenTypes::Not)
            {
                if self.check(SQLTokenTypes::IS) {
                    self.consume(SQLTokenTypes::IS, "expected IS operator")?;
                    if self.check(SQLTokenTypes::Not) {
                        self.consume(SQLTokenTypes::Not, "expected NOT operator")?;
                        if self.check(SQLTokenTypes::Null) {
                            self.consume(SQLTokenTypes::Null, "expected NULL operator")?;
                            return Ok(Condition::NullCheck(NullCheckCondition::IsNotNull {
                                identifier: left,
                            }));
                        }
                    }
                    if self.check(SQLTokenTypes::Null) {
                        self.consume(SQLTokenTypes::Null, "expected NULL operator")?;
                        return Ok(Condition::NullCheck(NullCheckCondition::IsNull {
                            identifier: left,
                        }));
                    }

                    return Err("unexpected token found".to_string());
                }
                return Err("unexpected token found".to_string());
            } else {
                // If no comparison operator, treat the identifier as a boolean condition (i.e., equals true).
                return Ok(Condition::Comparison(ComparisonCondition {
                    operator: ComparisonOperator::Equal,
                    left: Expression::Identifier(left),
                    right: Expression::Literal(Literal::Boolean(true)),
                }));
            }
        }

        Err("Expected identifier on the left-hand side of the comparison.".to_string())
    }

    fn expression(&mut self) -> Result<Expression, String> {
        if self.check(SQLTokenTypes::Identifier) {
            Ok(Expression::Identifier(self.advance().lexeme.clone()))
        } else if self.check(SQLTokenTypes::String) {
            Ok(Expression::Literal(Literal::String(
                self.advance().lexeme.clone(),
            )))
        } else if self.check(SQLTokenTypes::Number) {
            let number: f64 = self
                .advance()
                .lexeme
                .parse()
                .map_err(|_| "Invalid number".to_string())?;
            Ok(Expression::Literal(Literal::Number(number)))
        } else {
            Err("Expected expression".to_string())
        }
    }

    fn parse_column_list(&mut self) -> Result<Vec<String>, String> {
        let mut columns = Vec::new();
        loop {
            columns.push(
                self.consume(SQLTokenTypes::Identifier, "Expect column name")?
                    .lexeme
                    .clone(),
            );
            if !self.match_token(SQLTokenTypes::Comma) {
                break;
            }
        }
        self.consume(SQLTokenTypes::Rightparen, "Expect ) after column list")?;
        Ok(columns)
    }

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut expressions = Vec::new();
        loop {
            expressions.push(self.expression()?);
            if !self.match_token(SQLTokenTypes::Comma) {
                break;
            }
        }
        Ok(expressions)
    }

    fn parse_assignments(&mut self) -> Result<Vec<Assignment>, String> {
        let mut assignments = Vec::new();
        loop {
            let column = self
                .consume(SQLTokenTypes::Identifier, "Expect column name")?
                .lexeme
                .clone();
            self.consume(SQLTokenTypes::Equal, "Expect = after column name")?;
            let value = self.expression()?;
            assignments.push(Assignment { column, value });
            if !self.match_token(SQLTokenTypes::Comma) {
                break;
            }
        }
        Ok(assignments)
    }

    fn parse_column_definitions(&mut self) -> Result<Vec<ColumnDefinition>, String> {
        let mut columns = Vec::new();
        loop {
            let name = self
                .consume(SQLTokenTypes::Identifier, "Expect column name")?
                .lexeme
                .clone();
            let data_type = self.parse_data_type()?;
            let constraints = self.parse_column_constraints()?;
            columns.push(ColumnDefinition {
                name,
                data_type,
                constraints,
            });
            if !self.match_token(SQLTokenTypes::Comma) {
                break;
            }
        }
        Ok(columns)
    }

    fn parse_data_type(&mut self) -> Result<DataType, String> {
        let type_name = self
            .consume(SQLTokenTypes::Identifier, "Expect data type")?
            .lexeme
            .to_uppercase();
        match type_name.as_str() {
            "INTEGER" => Ok(DataType::Integer),
            "FLOAT" => Ok(DataType::Float),
            "VARCHAR" => {
                if self.match_token(SQLTokenTypes::Leftparen) {
                    let size = self
                        .consume(SQLTokenTypes::Number, "Expect size for VARCHAR")?
                        .lexeme
                        .parse()
                        .map_err(|_| "Invalid VARCHAR size".to_string())?;
                    self.consume(SQLTokenTypes::Rightparen, "Expect ) after VARCHAR size")?;
                    Ok(DataType::Varchar(Some(size)))
                } else {
                    Ok(DataType::Varchar(None))
                }
            }
            "BOOLEAN" => Ok(DataType::Boolean),
            _ => Err(format!("Unsupported data type: {}", type_name)),
        }
    }

    fn parse_column_constraints(&mut self) -> Result<Vec<ColumnConstraint>, String> {
        let mut constraints = Vec::new();
        while self.match_token(SQLTokenTypes::Primary)
            || self.match_token(SQLTokenTypes::Not)
            || self.match_token(SQLTokenTypes::Unique)
        {
            match self.previous().token_type {
                SQLTokenTypes::Primary => {
                    self.consume(SQLTokenTypes::Key, "Expect KEY after PRIMARY")?;
                    constraints.push(ColumnConstraint::PrimaryKey);
                }
                SQLTokenTypes::Not => {
                    self.consume(SQLTokenTypes::Null, "Expect NULL after NOT")?;
                    constraints.push(ColumnConstraint::NotNull)
                }
                SQLTokenTypes::Unique => {
                    self.consume(SQLTokenTypes::Unique, "Expected Unique constraints")?;
                    constraints.push(ColumnConstraint::Unique)
                }
                _ => return Err("unknown token found".to_string()),
            }
        }
        Ok(constraints)
    }

    fn consume(&mut self, token_type: SQLTokenTypes, message: &str) -> Result<&Token, String> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(message.to_string())
        }
    }

    fn match_token(&mut self, token_type: SQLTokenTypes) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token_type: SQLTokenTypes) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().token_type == token_type
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == SQLTokenTypes::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_statement() {
        let mut parser = Parser::new(
            "SELECT name, age FROM users WHERE NOT(((foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')) AND (IS_ACTIVE = FALSE AND IS_ENABLED))"
                .to_string(),
        );
        let result = parser.parse();
        if let Ok(SQLStatement::Select(select_stmt)) = result {
            assert_eq!(select_stmt.columns.len(), 2);
            assert_eq!(select_stmt.from, Some("users".to_string()));
            assert!(select_stmt.where_clause.is_some());
            if let Some(where_clause) = select_stmt.where_clause {
                println!("{:?}", where_clause.condition)
            }
        } else {
            panic!("Expected Select statement, got error{:?}", result);
        }
    }

    #[test]
    fn test_insert_statement() {
        let mut parser =
            Parser::new("INSERT INTO users (name, age) VALUES ('John Doe', 30)".to_string());
        let result = parser.parse();
        assert!(result.is_ok());
        if let Ok(SQLStatement::Insert(insert_stmt)) = result {
            assert_eq!(insert_stmt.table, "users");
            assert_eq!(insert_stmt.columns, vec!["name", "age"]);
            assert_eq!(insert_stmt.values.len(), 2);
        } else {
            panic!("Expected Insert statement");
        }
    }

    #[test]
    fn test_update_statement() {
        let mut parser =
            Parser::new("UPDATE users SET age = 31 WHERE name = 'John Doe'".to_string());
        let result = parser.parse();
        assert!(result.is_ok());
        if let Ok(SQLStatement::Update(update_stmt)) = result {
            assert_eq!(update_stmt.table, "users");
            assert_eq!(update_stmt.assignments.len(), 1);
            assert!(update_stmt.where_clause.is_some());
        } else {
            panic!("Expected Update statement");
        }
    }

    #[test]
    fn test_delete_statement() {
        let mut parser = Parser::new("DELETE FROM users WHERE age < 18".to_string());
        let result = parser.parse();
        println!("{:?}", result);
        assert!(result.is_ok());
        if let Ok(SQLStatement::Delete(delete_stmt)) = result {
            assert_eq!(delete_stmt.table, "users");
            assert!(delete_stmt.where_clause.is_some());
        } else {
            panic!("Expected Delete statement");
        }
    }

    #[test]
    fn test_create_table_statement() {
        let mut parser = Parser::new("CREATE TABLE products (id INTEGER PRIMARY KEY, name VARCHAR(100) NOT NULL, price FLOAT)".to_string());
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Failed to parse create table: {:?}",
            result.unwrap_err()
        );
        if let Ok(SQLStatement::Create(create_stmt)) = result {
            assert_eq!(create_stmt.table, "products");
            assert_eq!(create_stmt.columns.len(), 3);
        } else {
            panic!("Expected Create statement");
        }
    }

    #[test]
    fn test_drop_table_statement() {
        let mut parser = Parser::new("DROP TABLE old_users".to_string());
        let result = parser.parse();
        assert!(result.is_ok());
        if let Ok(SQLStatement::Drop(drop_stmt)) = result {
            assert_eq!(drop_stmt.table, "old_users");
        } else {
            panic!("Expected Drop statement");
        }
    }
}
