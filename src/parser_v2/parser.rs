use super::{
    ast::{
        Assignment, ColumnConstraint, ColumnDefinition, ComparisonCondition, ComparisonOperator,
        Condition, CreateStatement, DataType, DeleteStatement, DropStatement, Expression,
        InsertStatement, Literal, LogicalCondition, LogicalOperator, OrderByClause, SQLStatement,
        SelectColumn, SelectStatement, UpdateStatement, WhereClause,
    },
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
        &self.tokens[self.current]
    }

    fn check(&mut self, token_type: SQLTokenType) -> bool {
        self.peek().token_type == token_type
    }

    fn is_at_end(&mut self) -> bool {
        self.current >= self.tokens.len()
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

    fn select_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Select, "expected SELECT keyword")?;
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
            self.consume(SQLTokenType::Comma, "expected comma")?;
        }

        self.consume(SQLTokenType::From, "expected FROM keyword")?;
        let table_name = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        let mut where_clause: Option<WhereClause> = None;
        let mut order_by_clause: Option<Vec<OrderByClause>> = None;

        while !self.is_at_end() {
            if self.check(SQLTokenType::Where) {
                self.consume(SQLTokenType::Where, "expected WHERE keyword")?;
                let condition = self.handle_where_clause()?;
                where_clause = Some(WhereClause { condition });
            }
            if self.check(SQLTokenType::OrderBy) {
                self.consume(SQLTokenType::OrderBy, "expected ORDER BY keyword")?;
                let mut order_by_columns = Vec::new();
                loop {
                    let column = self
                        .consume(SQLTokenType::Identifier, "expected column name")?
                        .lexeme
                        .clone();
                    let order = if self.check(SQLTokenType::AcendingOrder) {
                        self.consume(SQLTokenType::AcendingOrder, "expected ASC")?;
                        Some(true)
                    } else if self.check(SQLTokenType::DecendingOrder) {
                        self.consume(SQLTokenType::DecendingOrder, "expected DESC")?;
                        Some(false)
                    } else {
                        None
                    };
                    order_by_columns.push(OrderByClause {
                        column_name: column,
                        is_asec: order.unwrap_or(true),
                    });
                    if !self.check(SQLTokenType::Comma) {
                        break;
                    }
                    self.consume(SQLTokenType::Comma, "expected comma")?;
                }
                order_by_clause = Some(order_by_columns);
            }
        }

        Ok(SQLStatement::Select(SelectStatement {
            columns,
            from: table_name,
            where_clause,
            order_by: order_by_clause,
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
        let mut condition = self.parse_condition()?;

        while !self.is_at_end() {
            if self.check(SQLTokenType::And) {
                self.consume(SQLTokenType::And, "expected AND")?;
                let right_condition = self.parse_condition()?;
                condition = Condition::Logical(LogicalCondition {
                    left: Box::new(condition),
                    operator: LogicalOperator::And,
                    right: Box::new(right_condition),
                });
            } else if self.check(SQLTokenType::OR) {
                self.consume(SQLTokenType::OR, "expected OR")?;
                let right_condition = self.parse_condition()?;
                condition = Condition::Logical(LogicalCondition {
                    left: Box::new(condition),
                    operator: LogicalOperator::Or,
                    right: Box::new(right_condition),
                });
            } else if self.check(SQLTokenType::OrderBy) {
                // Stop the WHERE clause parsing when ORDER BY starts
                break;
            } else {
                break; // Break when we encounter an unexpected token
            }
        }

        Ok(condition)
    }

    fn parse_condition(&mut self) -> Result<Condition, String> {
        let left = self
            .consume(SQLTokenType::Identifier, "expected column name")?
            .lexeme
            .clone();
        let operator = self
            .consume(SQLTokenType::Equal, "expected comparison operator")?
            .lexeme
            .clone();
        let right = self.parse_expression()?;

        let comparison_operator = match operator.as_str() {
            "=" => ComparisonOperator::Equal,
            "!=" => ComparisonOperator::NotEqual,
            ">" => ComparisonOperator::GreaterThan,
            "<" => ComparisonOperator::LessThan,
            ">=" => ComparisonOperator::GreaterThanOrEqual,
            "<=" => ComparisonOperator::LessThanOrEqual,
            _ => return Err("Unexpected operator".to_string()),
        };

        Ok(Condition::Comparison(ComparisonCondition {
            operator: comparison_operator,
            left: Expression::Identifier(left),
            right,
        }))
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        if self.check(SQLTokenType::String) {
            Ok(Expression::Literal(Literal::String(
                self.consume(SQLTokenType::String, "expected string")?
                    .lexeme
                    .clone(),
            )))
        } else if self.check(SQLTokenType::Number) {
            Ok(Expression::Literal(Literal::Number(
                self.consume(SQLTokenType::Number, "expected number")?
                    .lexeme
                    .parse()
                    .unwrap(),
            )))
        } else if self.check(SQLTokenType::Boolean) {
            Ok(Expression::Literal(Literal::Boolean(
                self.consume(SQLTokenType::Boolean, "expected boolean")?
                    .lexeme
                    == "true",
            )))
        } else {
            Err("Expected a valid expression".to_string())
        }
    }

    // Insert statement parsing
    fn insert_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Insert, "expected INSERT keyword")?;
        self.consume(SQLTokenType::Into, "expected INTO keyword")?;
        let table = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(SQLTokenType::Leftparen, "expected (")?;
        let mut columns = Vec::new();
        loop {
            columns.push(
                self.consume(SQLTokenType::Identifier, "expected column name")?
                    .lexeme
                    .clone(),
            );
            if !self.check(SQLTokenType::Comma) {
                break;
            }
            self.consume(SQLTokenType::Comma, "expected comma")?;
        }
        self.consume(SQLTokenType::Rightparen, "expected )")?;

        self.consume(SQLTokenType::Values, "expected VALUES keyword")?;
        self.consume(SQLTokenType::Leftparen, "expected (")?;
        let mut values = Vec::new();
        loop {
            let value = self.parse_expression()?;
            values.push(value);
            if !self.check(SQLTokenType::Comma) {
                break;
            }
            self.consume(SQLTokenType::Comma, "expected comma")?;
        }
        self.consume(SQLTokenType::Rightparen, "expected )")?;

        Ok(SQLStatement::Insert(InsertStatement {
            table,
            columns,
            values,
        }))
    }

    // Update statement parsing
    fn update_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Update, "expected UPDATE keyword")?;
        let table = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(SQLTokenType::Set, "expected SET keyword")?;
        let mut assignments = Vec::new();
        loop {
            let column = self
                .consume(SQLTokenType::Identifier, "expected column name")?
                .lexeme
                .clone();
            self.consume(SQLTokenType::Equal, "expected = sign")?;
            let value = self.parse_expression()?;
            assignments.push(Assignment { column, value });
            if !self.check(SQLTokenType::Comma) {
                break;
            }
            self.consume(SQLTokenType::Comma, "expected comma")?;
        }

        let mut where_clause: Option<WhereClause> = None;
        if self.check(SQLTokenType::Where) {
            self.consume(SQLTokenType::Where, "expected WHERE keyword")?;
            let condition = self.handle_where_clause()?;
            where_clause = Some(WhereClause { condition });
        }

        Ok(SQLStatement::Update(UpdateStatement {
            table,
            assignments,
            where_clause,
        }))
    }

    // Delete statement parsing
    fn delete_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Delete, "expected DELETE keyword")?;
        self.consume(SQLTokenType::From, "expected FROM keyword")?;
        let table = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        let mut where_clause: Option<WhereClause> = None;
        if self.check(SQLTokenType::Where) {
            self.consume(SQLTokenType::Where, "expected WHERE keyword")?;
            let condition = self.handle_where_clause()?;
            where_clause = Some(WhereClause { condition });
        }

        Ok(SQLStatement::Delete(DeleteStatement {
            table,
            where_clause,
        }))
    }

    // Create statement parsing
    fn create_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Create, "expected CREATE keyword")?;
        self.consume(SQLTokenType::Table, "expected TABLE keyword")?;
        let table = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(SQLTokenType::Leftparen, "expected (")?;
        let mut columns = Vec::new();
        loop {
            let column_name = self
                .consume(SQLTokenType::Identifier, "expected column name")?
                .lexeme
                .clone();
            self.consume(SQLTokenType::Identifier, "expected data type")?;
            let data_type = match self.peek().lexeme.as_str() {
                "INTEGER" => DataType::Integer,
                "FLOAT" => DataType::Float,
                "VARCHAR" => {
                    self.consume(SQLTokenType::Leftparen, "expected (")?;
                    let size = self
                        .consume(SQLTokenType::Number, "expected size")?
                        .lexeme
                        .parse()
                        .unwrap();
                    DataType::Varchar(Some(size))
                }
                "BOOLEAN" => DataType::Boolean,
                _ => return Err("Expected a valid data type".to_string()),
            };
            let mut constraints = Vec::new();

            while self.check(SQLTokenType::Primary)
                || self.check(SQLTokenType::Not)
                || self.check(SQLTokenType::Unique)
            {
                if self.check(SQLTokenType::Primary) {
                    self.consume(SQLTokenType::Primary, "expected PRIMARY")?;
                    self.consume(SQLTokenType::Key, "expected KEY")?;
                    constraints.push(ColumnConstraint::PrimaryKey);
                } else if self.check(SQLTokenType::Not) {
                    self.consume(SQLTokenType::Not, "expected NOT")?;
                    self.consume(SQLTokenType::Null, "expected NULL")?;
                    constraints.push(ColumnConstraint::NotNull);
                } else if self.check(SQLTokenType::Unique) {
                    self.consume(SQLTokenType::Unique, "expected UNIQUE")?;
                    constraints.push(ColumnConstraint::Unique);
                }
            }
            columns.push(ColumnDefinition {
                name: column_name,
                data_type,
                constraints,
            });
            if !self.check(SQLTokenType::Comma) {
                break;
            }
            self.consume(SQLTokenType::Comma, "expected comma")?;
        }
        self.consume(SQLTokenType::Rightparen, "expected )")?;

        Ok(SQLStatement::Create(CreateStatement { table, columns }))
    }

    fn drop_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(SQLTokenType::Drop, "expected DROP keyword")?;
        self.consume(SQLTokenType::Table, "expected TABLE keyword")?;
        let table = self
            .consume(SQLTokenType::Identifier, "expected table name")?
            .lexeme
            .clone();

        Ok(SQLStatement::Drop(DropStatement { table }))
    }
}
