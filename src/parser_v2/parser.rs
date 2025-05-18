use std::{collections::VecDeque, rc::Rc};

use super::{
    ast::{
        Assignment, ColumnConstraint, ColumnDefinition, ComparisonCondition, ComparisonOperator,
        Condition, CreateStatement, DataType, DeleteStatement, DropStatement, Expression,
        InCondition, InValues, InsertStatement, Literal, LogicalCondition, LogicalOperator,
        NullCheckCondition, OrderByClause, SQLStatement, SelectColumn, SelectStatement,
        UpdateStatement, WhereClause,
    },
    tokens::{ParsedLiteral, Token, Types},
};

pub struct Parser {
    tokens: Vec<Rc<Token>>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Rc<Token>>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_and_build_ast(mut self) -> Result<SQLStatement, String> {
        match self.peek().token_type {
            Types::Select => self.select_statement(),
            Types::Insert => self.insert_statement(),
            Types::Update => self.update_statement(),
            Types::Delete => self.delete_statement(),
            Types::Create => self.create_statement(),
            Types::Drop => self.drop_statement(),
            _ => Err("Unexpected statement type".to_string()),
        }
    }

    fn peek(&self) -> Rc<Token> {
        self.tokens[self.current].clone()
    }

    fn check(&mut self, token_type: Types) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().token_type == token_type
    }

    fn is_at_end(&mut self) -> bool {
        self.current >= self.tokens.len()
    }

    fn consume(&mut self, token_type: Types, message: &str) -> Result<Rc<Token>, String> {
        if self.check(token_type) {
            self.current += 1;
            Ok(self.tokens[self.current - 1].clone())
        } else {
            Err(format!(
                "error at line and column: {}:{}\nerror message: {}",
                &self.tokens[self.current].line, &self.tokens[self.current].column, message
            ))
        }
    }

    fn select_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(Types::Select, "expected SELECT keyword")?;
        let mut columns = Vec::new();
        loop {
            if self.check(Types::AllColumnsOrMultiplication) {
                self.consume(Types::AllColumnsOrMultiplication, "expected *")?;
                columns.push(SelectColumn::All);
                break;
            } else if self.check(Types::Identifier) {
                columns.push(SelectColumn::Column(
                    self.consume(Types::Identifier, "expected an identifier for column")?
                        .lexeme
                        .clone(),
                ));
            } else {
                return Err("Expected column name or *".to_string());
            }
            if !self.check(Types::Comma) {
                break;
            }
            self.consume(Types::Comma, "expected comma")?;
        }

        self.consume(Types::From, "expected FROM keyword")?;
        let table_name = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        let mut where_clause: Option<WhereClause> = None;
        let mut order_by_clause: Option<Vec<OrderByClause>> = None;

        while !self.is_at_end() {
            if self.check(Types::Where) {
                self.consume(Types::Where, "expected WHERE keyword")?;
                let condition = self.handle_where_clause()?;
                where_clause = Some(WhereClause { condition });
            }
            if self.check(Types::OrderBy) {
                self.consume(Types::OrderBy, "expected ORDER BY keyword")?;
                let mut order_by_columns = Vec::new();
                loop {
                    let column = self
                        .consume(Types::Identifier, "expected column name")?
                        .lexeme
                        .clone();
                    let order = if self.check(Types::AscendingOrder) {
                        self.consume(Types::AscendingOrder, "expected ASC")?;
                        Some(true)
                    } else if self.check(Types::DecendingOrder) {
                        self.consume(Types::DecendingOrder, "expected DESC")?;
                        Some(false)
                    } else {
                        None
                    };
                    order_by_columns.push(OrderByClause {
                        column_name: column,
                        is_asec: order.unwrap_or(true),
                    });
                    if !self.check(Types::Comma) {
                        break;
                    }
                    self.consume(Types::Comma, "expected comma")?;
                }
                order_by_clause = Some(order_by_columns);
            }
        }

        Ok(SQLStatement::Select(SelectStatement {
            columns,
            from: table_name,
            where_clause,
            order_by: order_by_clause,
            limit: None,
            offset: None,
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
        let mut operator_parenthesis_stack = Vec::<Rc<Token>>::new();
        let mut output_queue = VecDeque::<Condition>::new();

        /*
         * -------------------------------------------------------------------------
         * we will be using shunting yard
         * operator_parenthesis_stack will have AND and OR operators and Parenthesis
         * output_queue will hold conditions
         * ---------------------------------------------------------------------------
         *
         * pesudo algo:
         * 1. loop till current index is not at end of array or current token is not OrderBy
         * 2. if current token is AND or OR operator put them in operator_parenthesis_stack
         *  - if operator_parenthesis_stack is empty simply insert the token
         *  - if it's not empty check if priority current token is greater or equal to token at
         *       of stack, if it's the simply insert it else drain the stack and
         *
         */

        while !self.is_at_end() && !self.check(Types::OrderBy) {
            let token = self.peek();
            match token.token_type {
                Types::Literal | Types::Identifier => {
                    output_queue.push_back(self.parse_primary_condition(token)?);
                    continue;
                }
                Types::Not => {
                    self.consume(Types::Not, "")?;
                    let condition = if self.check(Types::OpenParen) {
                        let inner = self.handle_where_clause()?;
                        inner
                    } else {
                        let tok = self.peek();
                        self.parse_primary_condition(tok)?
                    };

                    output_queue.push_back(Condition::Not(Box::new(condition)));
                }
                Types::OpenParen => {
                    operator_parenthesis_stack.push(self.consume(token.token_type, "")?);
                    continue;
                }
                Types::CloseParen => {
                    if let Err(err) = self.consume(token.token_type, "") {
                        eprintln!("foo: {}", err);
                    }
                    while let Some(operator) = operator_parenthesis_stack.pop() {
                        if operator.token_type == Types::OpenParen {
                            break;
                        }
                        let right = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
                        let left = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
                        output_queue.push_back(Condition::Logical(LogicalCondition {
                            left: Box::new(left),
                            right: Box::new(right),
                            operator: LogicalOperator::match_sql_token_to_operator(
                                operator.token_type,
                            )?,
                        }));
                    }
                }
                Types::And | Types::Or => {
                    while let Some(stack_token) = operator_parenthesis_stack.last() {
                        if stack_token.token_type == Types::OpenParen
                            || self.operator_precedence(token.token_type)
                                > self.operator_precedence(stack_token.token_type)
                        {
                            break;
                        }
                        let operator = LogicalOperator::match_sql_token_to_operator(
                            operator_parenthesis_stack.pop().unwrap().token_type,
                        )?;
                        let right = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
                        let left = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
                        output_queue.push_back(Condition::Logical(LogicalCondition {
                            left: Box::new(left),
                            right: Box::new(right),
                            operator,
                        }));
                    }
                    operator_parenthesis_stack.push(self.consume(token.token_type, "")?);
                }
                _ => return Err("Unexpected token in WHERE clause".to_string()),
            }
        }
        while let Some(operator) = operator_parenthesis_stack.pop() {
            let right = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
            let left = output_queue.pop_back().ok_or("Invalid WHERE clause")?;
            output_queue.push_back(Condition::Logical(LogicalCondition {
                left: Box::new(left),
                right: Box::new(right),
                operator: LogicalOperator::match_sql_token_to_operator(operator.token_type)?,
            }));
        }

        output_queue
            .pop_back()
            .ok_or("Empty WHERE clause".to_string())
    }

    fn parse_primary_condition(&mut self, token: Rc<Token>) -> Result<Condition, String> {
        let lhs = self.consume(token.token_type, "")?.clone();
        let token = self.peek();
        match token.token_type {
            Types::Is => {
                self.consume(Types::Is, "")?;
                if self.peek().token_type == Types::Not {
                    self.consume(Types::Not, "")?;
                    self.consume(Types::Null, "expected NULL")?;

                    return Ok(Condition::NullCheck(NullCheckCondition::IsNotNull {
                        identifier: lhs.lexeme.clone(),
                    }));
                } else if self.peek().token_type == Types::Null {
                    return Ok(Condition::NullCheck(NullCheckCondition::IsNull {
                        identifier: lhs.lexeme.clone(),
                    }));
                } else {
                    Err(format!(
                        "unexpected token:{} found after IS at line:{}, column:{}",
                        self.peek().lexeme,
                        self.peek().line,
                        self.peek().column
                    ))
                }
            }
            Types::In => {
                let left = self.parse_expression_for_input(lhs)?;
                self.consume(Types::In, "")?;
                self.consume(Types::OpenParen, "expectd (")?;

                if self.check(Types::Identifier) {
                    let mut values = vec![];
                    loop {
                        if self.check(Types::Identifier) {
                            values.push(self.parse_expression()?);
                            continue;
                        }
                        if self.check(Types::Comma) {
                            self.consume(Types::Comma, "")?;
                            continue;
                        }
                        if self.check(Types::CloseParen) {
                            self.consume(Types::CloseParen, "")?;
                            break;
                        } else {
                            return Err("input of different types found".to_string());
                        }
                    }
                    return Ok(Condition::In(InCondition {
                        left,
                        values: super::ast::InValues::List(Some(values)),
                    }));
                } else if self.check(Types::Literal) {
                    let mut values = vec![];
                    loop {
                        if self.check(Types::Literal) {
                            let value = self.parse_expression()?;
                            if let Some(Expression::Literal(last_lit)) = values.last() {
                                if let Expression::Literal(_current_lit) = &value {
                                    if !matches!(last_lit, _current_lit) {
                                        return Err("input should be of the same type".to_string());
                                    }
                                }
                            }
                            values.push(value);
                            continue;
                        }
                        if self.check(Types::Comma) {
                            self.consume(Types::Comma, "")?;
                            continue;
                        }
                        if self.check(Types::CloseParen) {
                            self.consume(Types::CloseParen, "")?;
                            break;
                        } else {
                            return Err("input of different types found".to_string());
                        }
                    }
                    return Ok(Condition::In(InCondition {
                        left,
                        values: super::ast::InValues::List(Some(values)),
                    }));
                } else if self.check(Types::Select) {
                    let sql_statement = self.select_statement()?;
                    if let SQLStatement::Select(select) = sql_statement {
                        return Ok(Condition::In(InCondition {
                            left,
                            values: InValues::Subquery(Some(Box::new(select))),
                        }));
                    }
                }

                return Err("should not reach here".to_string());
            }

            Types::EqualTo
            | Types::NotEqualTo
            | Types::GreaterThan
            | Types::LessThan
            | Types::LessThanOrEqualTo
            | Types::GreaterThanOrEqualTo => {
                let left = self.parse_expression_for_input(lhs)?;
                let operator = match self.peek().token_type {
                    Types::EqualTo => ComparisonOperator::EqualTo,
                    Types::NotEqualTo => ComparisonOperator::NotEqual,
                    Types::GreaterThan => ComparisonOperator::GreaterThan,
                    Types::LessThan => ComparisonOperator::LessThan,
                    Types::LessThanOrEqualTo => ComparisonOperator::LessThanOrEqual,
                    Types::GreaterThanOrEqualTo => ComparisonOperator::GreaterThanOrEqual,
                    _ => return Err("invalid char found".to_string()),
                };
                self.consume(token.token_type, "")?;

                let right = self.parse_expression()?;

                return Ok(Condition::Comparison(ComparisonCondition {
                    operator,
                    left,
                    right,
                }));
            }
            _ => return Err("should not reach here".to_string()),
        }
    }

    fn operator_precedence(&mut self, sql_token: Types) -> i32 {
        match sql_token {
            Types::And => 2,
            Types::Or => 1,
            _ => 0,
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        if self.check(Types::Literal) {
            let token = self.consume(Types::Literal, "expected literal")?;
            let literal_type = match token.literal.as_ref().unwrap() {
                ParsedLiteral::Text(e) => Literal::String(e.to_owned()),
                ParsedLiteral::Number(e) => Literal::Number(e.to_owned()),
                ParsedLiteral::Decimal(e) => Literal::Decimal(e.to_owned()),
            };
            Ok(Expression::Literal(literal_type))
        } else if self.check(Types::Identifier) {
            return Ok(Expression::Identifier(
                self.consume(Types::Identifier, "")?.lexeme.to_string(),
            ));
        } else {
            Err("Expected a valid expression".to_string())
        }
    }

    fn parse_expression_for_input(&mut self, token: Rc<Token>) -> Result<Expression, String> {
        if token.token_type == Types::Literal {
            let literal_type = match token.literal.as_ref().unwrap() {
                ParsedLiteral::Text(e) => Literal::String(e.to_owned()),
                ParsedLiteral::Number(e) => Literal::Number(e.to_owned()),
                ParsedLiteral::Decimal(e) => Literal::Decimal(e.to_owned()),
            };
            return Ok(Expression::Literal(literal_type));
        }

        return Ok(Expression::Identifier(token.lexeme.to_string()));
    }

    // Insert statement parsing
    fn insert_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(Types::Insert, "expected INSERT keyword")?;
        self.consume(Types::Into, "expected INTO keyword")?;
        let table = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(Types::OpenParen, "expected (")?;
        let mut columns = Vec::new();
        loop {
            columns.push(
                self.consume(Types::Identifier, "expected column name")?
                    .lexeme
                    .clone(),
            );
            if !self.check(Types::Comma) {
                break;
            }
            self.consume(Types::Comma, "expected comma")?;
        }
        self.consume(Types::CloseParen, "expected )")?;

        self.consume(Types::Values, "expected VALUES keyword")?;
        self.consume(Types::OpenParen, "expected (")?;
        let mut values = Vec::new();
        loop {
            let value = self.parse_expression()?;
            values.push(value);
            if !self.check(Types::Comma) {
                break;
            }
            self.consume(Types::Comma, "expected comma")?;
        }
        self.consume(Types::CloseParen, "expected )")?;

        Ok(SQLStatement::Insert(InsertStatement {
            table,
            columns,
            values,
        }))
    }

    // Update statement parsing
    fn update_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(Types::Update, "expected UPDATE keyword")?;
        let table = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(Types::Set, "expected SET keyword")?;
        let mut assignments = Vec::new();
        loop {
            let column = self
                .consume(Types::Identifier, "expected column name")?
                .lexeme
                .clone();
            self.consume(Types::EqualTo, "expected = sign")?;
            let value = self.parse_expression()?;
            assignments.push(Assignment { column, value });
            if !self.check(Types::Comma) {
                break;
            }
            self.consume(Types::Comma, "expected comma")?;
        }

        let mut where_clause: Option<WhereClause> = None;
        if self.check(Types::Where) {
            self.consume(Types::Where, "expected WHERE keyword")?;
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
        self.consume(Types::Delete, "expected DELETE keyword")?;
        self.consume(Types::From, "expected FROM keyword")?;
        let table = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        let mut where_clause: Option<WhereClause> = None;
        if self.check(Types::Where) {
            self.consume(Types::Where, "expected WHERE keyword")?;
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
        self.consume(Types::Create, "expected CREATE keyword")?;
        self.consume(Types::Table, "expected TABLE keyword")?;
        let table = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        self.consume(Types::OpenParen, "expected (")?;
        let mut columns = Vec::new();
        loop {
            let column_name = self
                .consume(Types::Identifier, "expected column name")?
                .lexeme
                .clone();
            self.consume(Types::Identifier, "expected data type")?;
            let data_type = match self.peek().token_type {
                Types::Integer => DataType::Integer,
                Types::Decimal => DataType::Decimal,
                Types::Text => DataType::Text,
                Types::Boolean => DataType::Boolean,
                _ => return Err("Expected a valid data type".to_string()),
            };
            let mut constraints = Vec::new();

            while self.check(Types::PrimaryKey)
                || self.check(Types::Not)
                || self.check(Types::UniqueKey)
            {
                if self.check(Types::PrimaryKey) {
                    constraints.push(ColumnConstraint::PrimaryKey);
                } else if self.check(Types::Not) {
                    self.consume(Types::Not, "expected NOT")?;
                    self.consume(Types::Null, "expected NULL")?;
                    constraints.push(ColumnConstraint::NotNull);
                } else if self.check(Types::UniqueKey) {
                    self.consume(Types::UniqueKey, "expected UNIQUE KEY")?;
                    constraints.push(ColumnConstraint::UniqueKey);
                }
            }
            columns.push(ColumnDefinition {
                name: column_name,
                data_type,
                constraints,
            });
            if !self.check(Types::Comma) {
                break;
            }
            self.consume(Types::Comma, "expected comma")?;
        }
        self.consume(Types::CloseParen, "expected )")?;

        Ok(SQLStatement::Create(CreateStatement { table, columns }))
    }
    // syntax to support
    // DROP table table_name,table_name2;
    // DROP SCHEMA schema_name CASCADE;
    // DROP SCHEMA IF EXISTS NAME, NAME2 RISITRICT;
    fn drop_statement(&mut self) -> Result<SQLStatement, String> {
        self.consume(Types::Drop, "expected DROP keyword")?;
        if self.check(Types::Table) && self.check(Types::Schema) {
            return Err("expected TABLE or SCHEMA after DROP".to_string());
        }
        if self.check(Types::Table) {
            self.consume(Types::Table, "expected TABLE keyword")?;
        } else if self.check(Types::Schema) {
            self.consume(Types::Schema, "expected TABLE keyword")?;
        }
        let table = self
            .consume(Types::Identifier, "expected table name")?
            .lexeme
            .clone();

        Ok(SQLStatement::Drop(DropStatement { table }))
    }
}
