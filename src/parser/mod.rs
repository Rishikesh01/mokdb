use std::vec;

use self::{
    ast::{
        Assignment, ColumnConstraint, ColumnDefinition, ComparisonCondition, ComparisonOperator,
        Condition, CreateStatement, DataType, DropStatement, Expression, InsertStatement, Literal,
        LogicalCondition, LogicalOperator, SQLStatement, SelectColumn, SelectStatement,
        WhereClause,
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
            SQLTokenTypes::SELECT => self.select_statement(),
            SQLTokenTypes::INSERT => self.insert_statement(),
            SQLTokenTypes::UPDATE => self.update_statement(),
            SQLTokenTypes::DELETE => self.delete_statement(),
            SQLTokenTypes::CREATE => self.create_statement(),
            SQLTokenTypes::DROP => self.drop_statement(),
            _ => Err("Unexpected statement type".to_string()),
        }
    }

    fn select_statement(&mut self) -> Result<SQLStatement, String> {
        self.advance();
        let mut columns = Vec::new();
        loop {
            if self.check(SQLTokenTypes::STAR) {
                self.advance();
                columns.push(SelectColumn::All);
                break;
            } else if self.check(SQLTokenTypes::IDENTIFIER) {
                columns.push(SelectColumn::Column(self.advance().lexeme.clone()));
            } else {
                return Err("Expected column name or *".to_string());
            }

            if !self.match_token(SQLTokenTypes::COMMA) {
                break;
            }
        }

        self.consume(SQLTokenTypes::FROM, "Expect FROM after select columns")?;
        let from = if self.check(SQLTokenTypes::IDENTIFIER) {
            Some(self.advance().lexeme.clone())
        } else {
            return Err("Expected table name after FROM".to_string());
        };

        let where_clause = if self.match_token(SQLTokenTypes::WHERE) {
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
        self.consume(SQLTokenTypes::INSERT, "Expect INSERT")?;
        self.consume(SQLTokenTypes::INTO, "Expect INTO after INSERT")?;
        let table = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect table name")?
            .lexeme
            .clone();

        let columns = if self.match_token(SQLTokenTypes::LEFTPAREN) {
            self.parse_column_list()?
        } else {
            Vec::new()
        };

        self.consume(SQLTokenTypes::VALUES, "Expect VALUES")?;
        self.consume(SQLTokenTypes::LEFTPAREN, "Expect ( after VALUES")?;
        let values = self.parse_expression_list()?;
        self.consume(SQLTokenTypes::RIGHTPAREN, "Expect ) after values")?;

        Ok(SQLStatement::Insert(InsertStatement {
            table,
            columns,
            values,
        }))
    }

    fn update_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::UPDATE, "Expect UPDATE")?;
        let table = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect table name")?
            .lexeme
            .clone();
        self.consume(SQLTokenTypes::SET, "Expect SET after table name")?;

        let assignments = self.parse_assignments()?;

        let where_clause = if self.match_token(SQLTokenTypes::WHERE) {
            Some(WhereClause {
                condition: self.condition()?,
            })
        } else {
            None
        };

        Ok(SQLStatement::Update(ast::UpdateStatement {
            table,
            assignments,
            where_clause,
        }))
    }

    fn delete_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::DELETE, "Expect DELETE")?;
        self.consume(SQLTokenTypes::FROM, "Expect FROM after DELETE")?;
        let table = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect table name")?
            .lexeme
            .clone();

        let where_clause = if self.match_token(SQLTokenTypes::WHERE) {
            Some(WhereClause {
                condition: self.condition()?,
            })
        } else {
            None
        };

        Ok(SQLStatement::Delete(ast::DeleteStatement {
            table,
            where_clause,
        }))
    }

    fn create_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::CREATE, "Expect CREATE")?;
        self.consume(SQLTokenTypes::TABLE, "Expect TABLE after CREATE")?;
        let table = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect table name")?
            .lexeme
            .clone();

        self.consume(SQLTokenTypes::LEFTPAREN, "Expect ( after table name")?;
        let columns = self.parse_column_definitions()?;
        self.consume(
            SQLTokenTypes::RIGHTPAREN,
            "Expect ) after column definitions",
        )?;

        Ok(SQLStatement::Create(CreateStatement { table, columns }))
    }

    fn drop_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenTypes::DROP, "Expect DROP")?;
        self.consume(SQLTokenTypes::TABLE, "Expect TABLE after DROP")?;
        let table = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect table name")?
            .lexeme
            .clone();

        Ok(SQLStatement::Drop(DropStatement { table }))
    }

    fn condition(&mut self) -> Result<Condition, String> {
        let mut condition = self.comparison()?;

        while self.match_token(SQLTokenTypes::AND) || self.match_token(SQLTokenTypes::OR) {
            let operator = match self.previous().token_type {
                SQLTokenTypes::AND => LogicalOperator::And,
                SQLTokenTypes::OR => LogicalOperator::Or,
                _ => unreachable!(),
            };
            let right = self.comparison()?;
            condition = Condition::Logical(LogicalCondition {
                left: Box::new(condition),
                operator,
                right: Box::new(right),
            });
        }

        Ok(condition)
    }

    fn where_clause(&mut self) -> Result<WhereClause, String> {
        let mut stack: Vec<Condition> = Vec::new();
        let mut balanced_parenthesis = Vec::new();
        let mut current_condition: Option<Condition> = None;

        while self.has_more_tokens() {
            if self.check(SQLTokenTypes::LEFTPAREN) {
                self.advance();
                balanced_parenthesis.push("(");
                continue;
            }

            if self.check(SQLTokenTypes::IDENTIFIER) {
                let left = Expression::Identifier(self.advance().lexeme.clone());

                let operator = match self.advance().token_type {
                    SQLTokenTypes::EQUAL => ComparisonOperator::Equal,
                    SQLTokenTypes::GREATER => ComparisonOperator::GreaterThan,
                    SQLTokenTypes::LESSER => ComparisonOperator::LessThan,
                    SQLTokenTypes::GREATER_OR_EQUAL => ComparisonOperator::GreaterThanOrEqual,
                    SQLTokenTypes::LESSER_OR_EQUAL => ComparisonOperator::LessThanOrEqual,
                    SQLTokenTypes::NOT_EQUAL => ComparisonOperator::NotEqual,
                    _ => return Err("Expected comparison operator in WHERE clause".to_string()),
                };

                let right = if self.check(SQLTokenTypes::IDENTIFIER) {
                    Expression::Identifier(self.advance().lexeme.clone())
                } else if self.check(SQLTokenTypes::NUMBER) {
                    let lexeme = self.advance().lexeme.clone();
                    match lexeme.parse::<f64>() {
                        Ok(number) => Expression::Literal(Literal::Number(number)),
                        Err(_) => return Err(format!("Failed to parse number: {}", lexeme)),
                    }
                } else if self.check(SQLTokenTypes::STRING) {
                    Expression::Literal(Literal::String(self.advance().lexeme.clone()))
                } else {
                    return Err("Expected value in WHERE clause".to_string());
                };

                current_condition = Some(Condition::Comparison(ComparisonCondition {
                    left,
                    operator,
                    right,
                }));

                if self.check(SQLTokenTypes::AND) || self.check(SQLTokenTypes::OR) {
                    let logical_operator = self.advance().token_type;
                    if let Some(condition) = current_condition.take() {
                        stack.push(condition);
                    }

                    let logical_condition = Condition::Logical(LogicalCondition {
                        left: Box::new(
                            stack
                                .pop()
                                .ok_or("No condition to apply logical operator")?,
                        ),
                        operator: match logical_operator {
                            SQLTokenTypes::AND => LogicalOperator::And,
                            SQLTokenTypes::OR => LogicalOperator::Or,
                            _ => unreachable!(),
                        },
                        right: Box::new(Condition::Comparison(ComparisonCondition {
                            // Placeholder; this will need to be correctly populated
                            left: Expression::Identifier("logical_condition".to_string()), // Placeholder
                            operator: ComparisonOperator::Equal, // Placeholder
                            right: Expression::Identifier("logical_condition".to_string()), // Placeholder
                        })),
                    });
                    stack.push(logical_condition); // Push logical condition back onto stack
                }
                current_condition = None; // Reset for the next condition
            } else if self.check(SQLTokenTypes::RIGHTPAREN) {
                self.advance(); // Consume ')'
                balanced_parenthesis.pop(); // Track closing parenthesis
                if let Some(condition) = current_condition.take() {
                    stack.push(condition); // Push the last condition before closing parenthesis
                }
            } else if self.check(SQLTokenTypes::SELECT) {
                // If we encounter SELECT, handle it as a subquery
                let subquery = self.parse_subquery()?;
                current_condition = Some(Condition::Comparison(ComparisonCondition {
                    left: Expression::Identifier("subquery_result".to_string()), // Placeholder
                    operator: ComparisonOperator::Equal,                         // Adjust as needed
                    right: subquery,
                }));
            } else {
                break; // No more valid tokens to process
            }
        }

        // Ensure all parentheses are balanced
        if !balanced_parenthesis.is_empty() {
            return Err("Unbalanced parentheses in WHERE clause".to_string());
        }

        // Final condition assembly
        let final_condition = if !stack.is_empty() {
            Some(stack.pop().unwrap()) // Pop the last condition from the stack
        } else {
            None
        };

        Ok(WhereClause {
            condition: match final_condition {
                Some(cond) => vec![cond],
                None => vec![], // No conditions found
            },
        })
    }

    // Function to parse a subquery
    fn parse_subquery(&mut self) -> Result<Expression, String> {
        self.advance(); // Consume 'SELECT'
                        // Here we would need to parse the select statement
        let select_statement = self.select_statement()?;

        // Return the subquery as an expression
        // This can be structured based on how you want to handle subqueries in your AST
        Ok(Expression::Identifier("subquery_result".to_string())) // Placeholder for the result
    }

    fn has_more_tokens(self) -> bool {
        return self.current < self.tokens.len();
    }

    fn comparison(&mut self) -> Result<Condition, String> {
        if self.match_token(SQLTokenTypes::NOT) {
            let condition = self.comparison()?;
            return Ok(Condition::Not(Box::new(condition)));
        }

        let left = self.expression()?;

        let operator = match self.advance().token_type {
            SQLTokenTypes::EQUAL => ComparisonOperator::Equal,
            SQLTokenTypes::GREATER => ComparisonOperator::GreaterThan,
            SQLTokenTypes::LESSER => ComparisonOperator::LessThan,
            SQLTokenTypes::GREATER_EQUAL => ComparisonOperator::GreaterThanOrEqual,
            SQLTokenTypes::LESSER_EQUAL => ComparisonOperator::LessThanOrEqual,
            SQLTokenTypes::NOT_EQUAL => ComparisonOperator::NotEqual,
            _ => return Err("Expected comparison operator".to_string()),
        };

        let right = self.expression()?;

        Ok(Condition::Comparison(ComparisonCondition {
            left,
            operator,
            right,
        }))
    }

    fn expression(&mut self) -> Result<Expression, String> {
        if self.check(SQLTokenTypes::IDENTIFIER) {
            Ok(Expression::Identifier(self.advance().lexeme.clone()))
        } else if self.check(SQLTokenTypes::STRING) {
            Ok(Expression::Literal(Literal::String(
                self.advance().lexeme.clone(),
            )))
        } else if self.check(SQLTokenTypes::NUMBER) {
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
                self.consume(SQLTokenTypes::IDENTIFIER, "Expect column name")?
                    .lexeme
                    .clone(),
            );
            if !self.match_token(SQLTokenTypes::COMMA) {
                break;
            }
        }
        self.consume(SQLTokenTypes::RIGHTPAREN, "Expect ) after column list")?;
        Ok(columns)
    }

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut expressions = Vec::new();
        loop {
            expressions.push(self.expression()?);
            if !self.match_token(SQLTokenTypes::COMMA) {
                break;
            }
        }
        Ok(expressions)
    }

    fn parse_assignments(&mut self) -> Result<Vec<Assignment>, String> {
        let mut assignments = Vec::new();
        loop {
            let column = self
                .consume(SQLTokenTypes::IDENTIFIER, "Expect column name")?
                .lexeme
                .clone();
            self.consume(SQLTokenTypes::EQUAL, "Expect = after column name")?;
            let value = self.expression()?;
            assignments.push(Assignment { column, value });
            if !self.match_token(SQLTokenTypes::COMMA) {
                break;
            }
        }
        Ok(assignments)
    }

    fn parse_column_definitions(&mut self) -> Result<Vec<ColumnDefinition>, String> {
        let mut columns = Vec::new();
        loop {
            let name = self
                .consume(SQLTokenTypes::IDENTIFIER, "Expect column name")?
                .lexeme
                .clone();
            let data_type = self.parse_data_type()?;
            let constraints = self.parse_column_constraints()?;
            columns.push(ColumnDefinition {
                name,
                data_type,
                constraints,
            });
            if !self.match_token(SQLTokenTypes::COMMA) {
                break;
            }
        }
        Ok(columns)
    }

    fn parse_data_type(&mut self) -> Result<DataType, String> {
        let type_name = self
            .consume(SQLTokenTypes::IDENTIFIER, "Expect data type")?
            .lexeme
            .to_uppercase();
        match type_name.as_str() {
            "INTEGER" => Ok(DataType::Integer),
            "FLOAT" => Ok(DataType::Float),
            "VARCHAR" => {
                if self.match_token(SQLTokenTypes::LEFTPAREN) {
                    let size = self
                        .consume(SQLTokenTypes::NUMBER, "Expect size for VARCHAR")?
                        .lexeme
                        .parse()
                        .map_err(|_| "Invalid VARCHAR size".to_string())?;
                    self.consume(SQLTokenTypes::RIGHTPAREN, "Expect ) after VARCHAR size")?;
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
        while self.match_token(SQLTokenTypes::PRIMARY)
            || self.match_token(SQLTokenTypes::NOT)
            || self.match_token(SQLTokenTypes::UNIQUE)
        {
            match self.previous().token_type {
                SQLTokenTypes::PRIMARY => {
                    self.consume(SQLTokenTypes::KEY, "Expect KEY after PRIMARY")?;
                    constraints.push(ColumnConstraint::PrimaryKey);
                }
                SQLTokenTypes::NOT => {
                    self.consume(SQLTokenTypes::NULL, "Expect NULL after NOT")?;
                    constraints.push(ColumnConstraint::NotNull);
                }
                SQLTokenTypes::UNIQUE => constraints.push(ColumnConstraint::Unique),
                _ => unreachable!(),
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
        self.peek().token_type == SQLTokenTypes::EOF
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
            "SELECT name, age FROM users WHERE (age > 18 or name IS NOT NULL) AND (name = 'data')"
                .to_string(),
        );
        let result = parser.parse();
        assert!(result.is_ok());
        if let Ok(SQLStatement::Select(select_stmt)) = result {
            assert_eq!(select_stmt.columns.len(), 2);
            assert_eq!(select_stmt.from, Some("users".to_string()));
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected Select statement");
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
