use crate::{
    errors::MokErrors,
    parser::ast::{
        ColumnDeclaration, ComparisonOperator, Create, Delete, Drop, TableLevelConstraints, Update,
        ValueType,
    },
};

use super::{
    ast::{
        BinaryOp, BinaryOperator, Condition, Cte, DataType, Distinct, Equality, Expression, Insert,
        JoinType, Logical, LogicalOperator, OrderBy, OrderDirection, Projections, ProjectionsItem,
        Select, Statement, TableRef,
    },
    lexer::Lexer,
    tokens::{LiteralValue, Syntax, Token},
};

pub struct Parser {
    lexer: Lexer,
    current: Option<Token>,
    peeked: Option<Token>,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let current = lexer.next();
        Self {
            lexer,
            current,
            peeked: None,
        }
    }

    fn advance(&mut self) {
        self.current = self.peeked.take().or_else(|| self.lexer.next());
    }

    fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = self.lexer.next();
        }
        self.peeked.as_ref()
    }

    fn current(&mut self) -> Option<&Token> {
        self.current.as_ref()
    }

    fn consume_current(&mut self) -> Option<Token> {
        let token = self.current.take();
        self.advance();
        token
    }

    fn consume_syntax(&mut self, syntax_token: Syntax) -> Result<Token, MokErrors> {
        let current = self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
        if current.kind == syntax_token {
            return Ok(current);
        }
        Err(MokErrors::UnexpectedToken {
            expected: syntax_token.to_string(),
            found: current.kind.to_string(),
            line: current.line,
            column: current.column,
        })
    }

    fn parse_select_statement(&mut self, expect_semicolon: bool) -> Result<Statement, MokErrors> {
        let ctes = self.parse_ctes()?;
        self.consume_syntax(Syntax::Select)?;
        let distinct = self.parse_distinct()?;
        let projections = self.parse_projections()?;
        let from = self.parse_from()?;
        let condition = self.parse_where_clause()?;
        let group_by = self.parse_group_by()?;
        let order_by = self.parse_order_by()?;
        let limit_and_offset = self.parse_limit_and_offset()?;
        self.check_for_semicolon(expect_semicolon)?;

        let select = Select {
            with: ctes,
            distinct,
            projection: projections,
            from,
            where_clause: condition,
            group_by,
            order_by,
            limit: limit_and_offset.0,
            offset: limit_and_offset.1,
        };
        Ok(Statement::SelectStmt(select))
    }

    fn check_for_semicolon(&mut self, expect_semicolon: bool) -> Result<(), MokErrors> {
        if expect_semicolon {
            self.consume_syntax(Syntax::Semicolon)?;
        }

        Ok(())
    }

    fn parse_projections(&mut self) -> Result<Projections, MokErrors> {
        let mut projections = vec![];
        loop {
            let current = self.current().ok_or(MokErrors::UnexpectedEof)?.clone();
            if current.kind == Syntax::Star {
                self.advance();
                return Ok(Projections::All);
            }
            if current.kind == Syntax::Comma {
                self.advance();
                continue;
            }
            if current.kind == Syntax::Identifier || current.kind == Syntax::Literal {
                let expression = self.parse_expression()?;

                if let Some(v) = self.current() {
                    if v.kind == Syntax::As {
                        self.consume_syntax(Syntax::As)?;
                        let token = self.consume_syntax(Syntax::Identifier)?;
                        projections.push(ProjectionsItem {
                            expr: expression,
                            alias: Some(token.lexeme.to_string()),
                        });
                        continue;
                    }
                }
                projections.push(ProjectionsItem {
                    expr: expression,
                    alias: None,
                });
                continue;
            }
            break;
        }

        Ok(Projections::Specified(projections))
    }

    fn parse_identifier_parts(
        &self,
        ident_token: &Token,
    ) -> Result<(Option<String>, String), MokErrors> {
        let parts: Vec<&str> = ident_token.lexeme.split('.').collect();

        if parts.len() > 2 {
            return Err(MokErrors::UnexpectedToken {
                expected: "to have at most 2 . separators for column".into(),
                found: ident_token.lexeme.clone(),
                line: ident_token.line,
                column: ident_token.column,
            });
        }

        let table = if parts.len() == 2 {
            Some(parts[0].to_string())
        } else {
            None
        };

        let name = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            parts[0].to_string()
        };

        Ok((table, name))
    }
    fn parse_expression(&mut self) -> Result<Expression, MokErrors> {
        let current = self.current().ok_or(MokErrors::UnexpectedEof)?.clone();

        match current.kind {
            Syntax::Identifier => {
                let ident_token = self.consume_syntax(Syntax::Identifier)?;
                let (table, name) = self.parse_identifier_parts(&ident_token)?;
                if let Some(op_token) = self.current() {
                    if let Some(operator) = Self::parse_binary_operator(op_token.kind.clone()) {
                        self.advance();
                        let rhs = self.parse_expression()?;
                        return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                            lhs: Expression::Identifier { table, name },
                            rhs,
                            operator,
                        })));
                    }
                }

                let (table, name) = self.parse_identifier_parts(&ident_token)?;
                Ok(Expression::Identifier { table, name })
            }

            Syntax::Literal => {
                let lit_token = self.consume_syntax(Syntax::Literal)?;

                let data_type = lit_token
                    .data_type
                    .clone()
                    .ok_or(MokErrors::InvalidInputType {
                        line: lit_token.line,
                        column: lit_token.column,
                        value: lit_token.lexeme.clone(),
                    })?;

                if let Some(op_token) = self.current() {
                    if let Some(operator) = Self::parse_binary_operator(op_token.kind.clone()) {
                        self.advance(); // consume the operator
                        let rhs = self.parse_expression()?;
                        return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                            lhs: Expression::Literal {
                                data_type: Self::literal_value(data_type),
                                alias: None,
                            },
                            rhs,
                            operator,
                        })));
                    }
                }

                Ok(Expression::Literal {
                    data_type: Self::literal_value(data_type),
                    alias: None,
                })
            }

            Syntax::OpenParen => {
                self.consume_syntax(Syntax::OpenParen)?;
                let expr = self.parse_expression()?;
                self.consume_syntax(Syntax::CloseParen)?;
                Ok(expr)
            }

            _ => Err(MokErrors::UnexpectedToken {
                expected: "expression".to_string(),
                found: current.lexeme.clone(),
                line: current.line,
                column: current.column,
            }),
        }
    }

    fn literal_value(input: LiteralValue) -> DataType {
        match input {
            LiteralValue::Text(v) => DataType::Text(v),
            LiteralValue::Int(v) => DataType::Int(v),
            LiteralValue::Float(v) => DataType::Float(v),
            LiteralValue::Boolean(v) => DataType::Boolean(v),
        }
    }

    fn parse_binary_operator(syntax_token: Syntax) -> Option<BinaryOperator> {
        match syntax_token {
            Syntax::Plus => Some(BinaryOperator::Add),
            Syntax::Minus => Some(BinaryOperator::Sub),
            Syntax::Star => Some(BinaryOperator::Multi),
            Syntax::Slash => Some(BinaryOperator::Div),
            Syntax::Percent => Some(BinaryOperator::Modulo),
            _ => None,
        }
    }

    fn parse_with(&mut self) -> Result<Cte, MokErrors> {
        todo!()
    }

    fn parse_where_clause(&mut self) -> Result<Option<Condition>, MokErrors> {
        match self.current() {
            Some(token) if token.kind == Syntax::Where => {
                self.advance();
                Ok(Some(self.parse_condition()?))
            }
            Some(token)
                if token.kind == Syntax::Order
                    || token.kind == Syntax::Group
                    || token.kind == Syntax::Limit
                    || token.kind == Syntax::Offset
                    || token.kind == Syntax::Returning
                    || token.kind == Syntax::Semicolon =>
            {
                Ok(None)
            }
            Some(token) if token.kind == Syntax::CloseParen => Ok(None),
            Some(token) => Err(MokErrors::UnexpectedToken {
                expected: Syntax::Where.to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            }),
            None => Ok(None),
        }
    }

    fn parse_condition(&mut self) -> Result<Condition, MokErrors> {
        self.parse_or_condition()
    }

    fn parse_or_condition(&mut self) -> Result<Condition, MokErrors> {
        let mut left = self.parse_and_condition()?;
        while let Some(token) = self.current() {
            if token.kind == Syntax::Or {
                self.advance();
                let right = self.parse_and_condition()?;
                left = Condition::Logical(Logical {
                    lhs: Box::new(left),
                    rhs: Box::new(right),
                    operator: LogicalOperator::Or,
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and_condition(&mut self) -> Result<Condition, MokErrors> {
        let mut left = self.parse_not_condition()?;
        while let Some(token) = self.current() {
            if token.kind == Syntax::And {
                self.advance();
                let right = self.parse_not_condition()?;
                left = Condition::Logical(Logical {
                    lhs: Box::new(left),
                    rhs: Box::new(right),
                    operator: LogicalOperator::And,
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_not_condition(&mut self) -> Result<Condition, MokErrors> {
        if let Some(token) = self.current() {
            if token.kind == Syntax::Not {
                self.advance();
                let inner = self.parse_primary_condition()?;
                return Ok(Condition::Not(Box::new(inner)));
            }
        }
        self.parse_primary_condition()
    }

    fn parse_primary_condition(&mut self) -> Result<Condition, MokErrors> {
        match self.current().ok_or(MokErrors::UnexpectedEof)?.kind {
            Syntax::OpenParen => {
                self.advance();
                let cond = self.parse_condition()?;
                self.consume_syntax(Syntax::CloseParen)?;
                Ok(cond)
            }

            Syntax::Identifier | Syntax::Literal => {
                let lhs = self.parse_expression()?;

                if let Some(op_token) = self.current() {
                    let comparison = match op_token.kind {
                        Syntax::Eq => Some(ComparisonOperator::Eq),
                        Syntax::Neq => Some(ComparisonOperator::Neq),
                        Syntax::Gt => Some(ComparisonOperator::Gt),
                        Syntax::Gte => Some(ComparisonOperator::Gte),
                        Syntax::Lt => Some(ComparisonOperator::Lt),
                        Syntax::Lte => Some(ComparisonOperator::Lte),
                        _ => None,
                    };

                    if let Some(op) = comparison {
                        self.advance();
                        let rhs = self.parse_expression()?;
                        return Ok(Condition::Comparison(Equality {
                            lhs,
                            rhs,
                            operator: op,
                        }));
                    }
                }

                Ok(Condition::Comparison(Equality {
                    lhs,
                    rhs: Expression::Literal {
                        data_type: DataType::Boolean(true),
                        alias: None,
                    },
                    operator: ComparisonOperator::Eq,
                }))
            }

            _ => {
                let token = self.consume_current().unwrap();
                Err(MokErrors::UnexpectedToken {
                    expected: "condition".to_string(),
                    found: token.lexeme.clone(),
                    line: token.line,
                    column: token.column,
                })
            }
        }
    }

    fn parse_group_by(&mut self) -> Result<Option<Vec<Expression>>, MokErrors> {
        if let Some(token) = self.current() {
            if token.kind == Syntax::Group {
                self.advance();
                self.consume_syntax(Syntax::By)?;
                let mut exprs = vec![];
                loop {
                    let expr = self.parse_expression()?;
                    exprs.push(expr);
                    if let Some(next) = self.current() {
                        if next.kind == Syntax::Comma {
                            self.advance();
                            continue;
                        }
                    }
                    break;
                }
                return Ok(Some(exprs));
            }
        }
        Ok(None)
    }

    fn parse_order_by(&mut self) -> Result<Option<Vec<OrderBy>>, MokErrors> {
        if let Some(token) = self.current() {
            if token.kind == Syntax::Order {
                self.advance();
                self.consume_syntax(Syntax::By)?;
                let mut order_by_items = vec![];
                loop {
                    let expr = self.parse_expression()?;
                    // optional ASC or DESC
                    let order = if let Some(next) = self.current() {
                        match next.kind {
                            Syntax::Asc => {
                                self.advance();
                                OrderDirection::Asc
                            }
                            Syntax::Desc => {
                                self.advance();
                                OrderDirection::Desc
                            }
                            _ => OrderDirection::Asc, // default
                        }
                    } else {
                        OrderDirection::Asc
                    };
                    order_by_items.push(OrderBy { expr, order });

                    if let Some(next) = self.current() {
                        if next.kind == Syntax::Comma {
                            self.advance();
                            continue;
                        }
                    }
                    break;
                }
                return Ok(Some(order_by_items));
            }
        }
        Ok(None)
    }

    fn parse_insert_statement(
        &mut self,
        mandatory_returning: bool,
        expect_eof: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        self.consume_syntax(Syntax::Insert)?;
        self.consume_syntax(Syntax::Into)?;
        let target_table = self.consume_syntax(Syntax::Identifier)?;
        let columns = self.parse_column_names_with_parenthesis()?;

        let mut values = None;
        let mut sub_select = None;

        if let Some(token) = self.current() {
            match token.kind {
                Syntax::Values => {
                    values = self.parse_values()?;
                }
                Syntax::Select => {
                    sub_select = self.parse_select_after_insert()?;
                }
                _ => {}
            }
        }

        if values.is_none() && sub_select.is_none() {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            return Err(MokErrors::UnexpectedToken {
                expected: "VALUES or SELECT".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            });
        }

        let returning = self.parse_returning()?;

        if mandatory_returning && returning.is_none() {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            return Err(MokErrors::UnexpectedToken {
                expected: "RETURNING clause".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            });
        }

        self.check_for_semicolon(expect_semicolon)?;

        Ok(Statement::InsertStmt(Insert {
            table: target_table.lexeme.clone(),
            columns,
            values,
            sub_select,
            returning,
        }))
    }

    fn parse_returning(&mut self) -> Result<Option<Vec<Expression>>, MokErrors> {
        let mut returning = None;
        if let Some(token) = self.current() {
            if token.kind == Syntax::Returning {
                self.advance();
                let mut returning_items = vec![];
                loop {
                    let expr = self.parse_expression()?;
                    returning_items.push(expr);
                    if let Some(next) = self.current() {
                        if next.kind == Syntax::Comma {
                            self.advance();
                            continue;
                        }
                    }
                    break;
                }
                returning = Some(returning_items);
            }
        }
        return Ok(returning);
    }

    fn parse_values(&mut self) -> Result<Option<Vec<Expression>>, MokErrors> {
        let mut values = None;
        self.consume_syntax(Syntax::Values)?;
        self.consume_syntax(Syntax::OpenParen)?;
        let mut list_of_values = vec![];
        loop {
            list_of_values.push(self.parse_expression()?);
            if let Some(next) = self.current() {
                if next.kind == Syntax::Comma {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        let token = self.consume_syntax(Syntax::CloseParen)?;
        if list_of_values.is_empty() {
            return Err(MokErrors::UnexpectedToken {
                expected: "values to contains actualy".to_string(),
                found: "empty lines".to_string(),
                line: token.line,
                column: token.column,
            });
        }
        values = Some(list_of_values);
        Ok(values)
    }

    fn parse_select_after_insert(&mut self) -> Result<Option<Select>, MokErrors> {
        let mut select_query = None;
        let next_token = self.current().ok_or(MokErrors::UnexpectedEof)?.clone();
        if next_token.kind == Syntax::Select {
            select_query = match self.parse_select_statement(false)? {
                Statement::SelectStmt(select) => Some(select),
                _ => None,
            };
        }
        Ok(select_query)
    }

    fn parse_column_names_with_parenthesis(&mut self) -> Result<Vec<String>, MokErrors> {
        let mut columns = vec![];
        if self.current().ok_or(MokErrors::UnexpectedEof)?.clone().kind == Syntax::OpenParen {
            self.advance();
            loop {
                let col = self.consume_syntax(Syntax::Identifier)?;
                columns.push(col.lexeme);
                if let Some(t) = self.current() {
                    if t.kind == Syntax::CloseParen {
                        self.advance();
                        return Ok(columns);
                    }
                }
                self.consume_syntax(Syntax::Comma)?;
            }
        }
        if columns.is_empty() {
            columns.push("*".to_string());
        }

        Ok(columns)
    }

    fn parse_delete_statement(
        &mut self,
        mandatory_returning: bool,
        expect_eof: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        self.consume_syntax(Syntax::Delete)?;
        self.consume_syntax(Syntax::From)?;
        let table = {
            let name = self.consume_syntax(Syntax::Identifier)?.lexeme;
            let mut alias = None;
            if let Some(token) = self.current() {
                if token.kind == Syntax::As {
                    self.advance();
                    let token = self.consume_syntax(Syntax::Identifier)?;
                    alias = Some(token.lexeme)
                }
            }
            TableRef::Table { name, alias }
        };
        let where_clause = self.parse_where_clause()?;
        let returning = self.parse_returning()?;

        if mandatory_returning && returning.is_none() {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            return Err(MokErrors::UnexpectedToken {
                expected: "RETURNING clause".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            });
        }

        self.check_for_semicolon(expect_semicolon)?;

        Ok(Statement::DeleteSmt(Delete {
            table,
            where_clause,
            returning,
        }))
    }

    fn parse_update_statement(
        &mut self,
        mandatory_returning: bool,
        expect_eof: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        self.consume_syntax(Syntax::Update)?;
        let table_name = self.consume_syntax(Syntax::Identifier)?;
        self.consume_syntax(Syntax::Set)?;
        let assignments = self.update_assignments()?;
        let where_clause = self.parse_where_clause()?;
        let returning = self.parse_returning()?;

        if mandatory_returning && returning.is_none() {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            return Err(MokErrors::UnexpectedToken {
                expected: "RETURNING clause".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            });
        }

        self.check_for_semicolon(expect_semicolon)?;

        Ok(Statement::UpdateStmt(Update {
            table: table_name.lexeme,
            assignments,
            where_clause,
            returning,
        }))
    }

    fn update_assignments(&mut self) -> Result<Vec<(String, Expression)>, MokErrors> {
        let mut set_columns = vec![];
        loop {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            if token.kind != Syntax::Identifier {
                break;
            }

            let column = self.consume_syntax(Syntax::Identifier)?;
            self.consume_syntax(Syntax::Eq)?;
            let expression = self.parse_expression()?;
            set_columns.push((column.lexeme, expression));
        }

        Ok(set_columns)
    }

    fn parse_create_statement(
        &mut self,
        expect_eof: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        self.consume_syntax(Syntax::Create)?;
        self.consume_syntax(Syntax::Table)?;
        let table_name = self.consume_syntax(Syntax::Identifier)?;
        if table_name.lexeme.len() > 100 {
            return Err(MokErrors::TableNameMaxLimitCrossed);
        }

        self.consume_syntax(Syntax::OpenParen)?;
        let columns = self.parse_column_declartions()?;
        let constraints = self.parse_constraints_declartions()?;
        self.consume_syntax(Syntax::CloseParen)?;

        self.check_for_semicolon(expect_semicolon)?;

        Ok(Statement::CreateStmt(Create {
            table: table_name.lexeme,
            columns,
            constraints,
        }))
    }

    fn parse_drop_statement(
        &mut self,
        expect_eof: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        self.consume_syntax(Syntax::Drop)?;
        let table_name = self.consume_syntax(Syntax::Identifier)?;

        self.check_for_semicolon(expect_semicolon)?;

        Ok(Statement::DropStmt(Drop {
            table: table_name.lexeme,
        }))
    }

    fn parse_ctes(&mut self) -> Result<Option<Vec<Cte>>, MokErrors> {
        if let Some(token) = self.current() {
            let token = token.clone();
            if token.kind != Syntax::With {
                return Ok(None);
            }
        }

        let mut ctes = vec![];
        self.consume_syntax(Syntax::With)?;
        loop {
            let cte_name = self.consume_syntax(Syntax::Identifier)?;
            self.consume_syntax(Syntax::As)?;
            self.consume_syntax(Syntax::OpenParen)?;
            let stmt = self.parse_statement(true, false)?;
            self.consume_syntax(Syntax::CloseParen)?;
            ctes.push(Cte {
                name: cte_name.lexeme.clone(),
                query: Box::new(stmt),
            });
            if let Some(current_token) = self.current() {
                if current_token.kind == Syntax::Comma {
                    self.advance();
                    continue;
                }
            }
            break;
        }

        Ok(Some(ctes))
    }

    fn parse_distinct(&mut self) -> Result<Option<Distinct>, MokErrors> {
        if let Some(token) = self.current() {
            if token.kind == Syntax::Distinct {
                self.advance();

                if let Some(next) = self.current() {
                    if next.kind == Syntax::On {
                        self.advance();
                        self.consume_syntax(Syntax::OpenParen)?;
                        let mut exprs = vec![];
                        loop {
                            let expr = self.parse_expression()?;
                            exprs.push(expr);
                            if let Some(comma) = self.current() {
                                if comma.kind == Syntax::Comma {
                                    self.advance();
                                    continue;
                                }
                            }
                            break;
                        }
                        self.consume_syntax(Syntax::CloseParen)?;
                        return Ok(Some(Distinct::On(exprs)));
                    }
                }
                return Ok(Some(Distinct::All));
            }
        }
        Ok(None)
    }

    fn parse_limit_and_offset(&mut self) -> Result<(Option<u64>, Option<u64>), MokErrors> {
        let mut limit = None;
        let mut offset = None;

        for _ in 0..2 {
            let current_token = match self.current() {
                Some(t) => t.clone(),
                None => break,
            };

            match current_token.kind {
                Syntax::Limit | Syntax::Offset => {
                    let keyword_token = self.consume_current().unwrap();
                    let value_token = match self.consume_current() {
                        Some(t) => t,
                        None => {
                            return Err(MokErrors::UnexpectedToken {
                                expected: "an integer value".to_string(),
                                found: "eof".to_string(),
                                line: keyword_token.line,
                                column: keyword_token.column,
                            });
                        }
                    };

                    if value_token.kind != Syntax::Literal || value_token.data_type.is_none() {
                        return Err(MokErrors::UnexpectedToken {
                            expected: "an integer value".to_string(),
                            found: value_token.lexeme.clone(),
                            line: value_token.line,
                            column: value_token.column,
                        });
                    }

                    let value = match Self::literal_value(value_token.data_type.clone().unwrap()) {
                        DataType::Int(i) if i >= 0 => i as u64,
                        DataType::Int(_) => {
                            return Err(MokErrors::UnexpectedToken {
                                expected: "a positive integer value".to_string(),
                                found: value_token.lexeme.clone(),
                                line: value_token.line,
                                column: value_token.column,
                            });
                        }
                        _ => {
                            return Err(MokErrors::UnexpectedToken {
                                expected: "an integer value".to_string(),
                                found: value_token.lexeme.clone(),
                                line: value_token.line,
                                column: value_token.column,
                            });
                        }
                    };

                    match keyword_token.kind {
                        Syntax::Limit => {
                            if limit.is_some() {
                                return Err(MokErrors::UnexpectedToken {
                                    expected: "only one LIMIT clause".to_string(),
                                    found: keyword_token.lexeme.clone(),
                                    line: keyword_token.line,
                                    column: keyword_token.column,
                                });
                            }
                            limit = Some(value);
                        }
                        Syntax::Offset => {
                            if offset.is_some() {
                                return Err(MokErrors::UnexpectedToken {
                                    expected: "only one OFFSET clause".to_string(),
                                    found: keyword_token.lexeme.clone(),
                                    line: keyword_token.line,
                                    column: keyword_token.column,
                                });
                            }
                            offset = Some(value);
                        }
                        _ => {}
                    }
                }
                _ => break,
            }
        }

        Ok((limit, offset))
    }

    fn parse_from(&mut self) -> Result<TableRef, MokErrors> {
        self.consume_syntax(Syntax::From)?;
        let table_name = self.consume_syntax(Syntax::Identifier)?;
        let mut table_ref = TableRef::Table {
            name: table_name.lexeme.clone(),
            alias: None,
        };
        if let Some(token) = self.current() {
            if token.kind == Syntax::As {
                self.advance();
                table_ref = TableRef::Table {
                    alias: Some(self.consume_syntax(Syntax::Identifier)?.lexeme),
                    name: table_name.lexeme.clone(),
                };
            }
        }

        table_ref = self.parse_join(table_ref)?;

        Ok(table_ref)
    }

    fn parse_join(&mut self, mut left_table: TableRef) -> Result<TableRef, MokErrors> {
        loop {
            let join_type = match self.current() {
                Some(token)
                    if matches!(
                        token.kind,
                        Syntax::Join | Syntax::Left | Syntax::Right | Syntax::Full | Syntax::Inner
                    ) =>
                {
                    self.parse_join_keyword()?
                }
                _ => break,
            };

            let right_table_token = self.consume_syntax(Syntax::Identifier)?;
            let mut right_table = TableRef::Table {
                name: right_table_token.lexeme.clone(),
                alias: None,
            };

            if let Some(token) = self.current() {
                if token.kind == Syntax::As {
                    self.advance();
                    let alias_token = self.consume_syntax(Syntax::Identifier)?;
                    right_table = TableRef::Table {
                        name: right_table_token.lexeme.clone(),
                        alias: Some(alias_token.lexeme),
                    };
                }
            }

            self.consume_syntax(Syntax::On)?;
            let on_condition = self.parse_condition()?;

            left_table = TableRef::Join {
                left: Box::new(left_table),
                right: Box::new(right_table),
                join_type,
                on: on_condition,
            };
        }

        Ok(left_table)
    }

    fn parse_join_keyword(&mut self) -> Result<JoinType, MokErrors> {
        match self.current() {
            Some(token) if token.kind == Syntax::Left => {
                self.advance();
                self.consume_syntax(Syntax::Join)?;
                Ok(JoinType::Left)
            }
            Some(token) if token.kind == Syntax::Right => {
                self.advance();
                self.consume_syntax(Syntax::Join)?;
                Ok(JoinType::Right)
            }
            Some(token) if token.kind == Syntax::Inner => {
                self.advance();
                self.consume_syntax(Syntax::Join)?;
                Ok(JoinType::Inner)
            }
            Some(token) if token.kind == Syntax::Full => {
                self.advance();
                self.consume_syntax(Syntax::Join)?;
                Ok(JoinType::FullOuter)
            }
            Some(token) if token.kind == Syntax::Join => {
                self.advance();
                Ok(JoinType::Inner)
            }
            Some(token) => Err(MokErrors::UnexpectedToken {
                expected: "JOIN".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            }),
            None => Err(MokErrors::UnexpectedEof),
        }
    }

    fn parse_column_declartions(&mut self) -> Result<Vec<ColumnDeclaration>, MokErrors> {
        let mut column_declarations = vec![];
        loop {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;
            if token.kind != Syntax::Identifier {
                break;
            }

            let column_name = self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
            let data_type = {
                let column_data_type = self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
                match column_data_type.kind {
                    Syntax::Text => Ok(ValueType::Text),
                    Syntax::Int => Ok(ValueType::Int),
                    Syntax::Float => Ok(ValueType::Float),
                    Syntax::Boolean => Ok(ValueType::Boolean),
                    _ => Err(MokErrors::UnexpectedToken {
                        expected: "to be int, float, text, boolean".into(),
                        found: column_data_type.lexeme,
                        line: column_data_type.line,
                        column: column_data_type.column,
                    }),
                }
            }?;
            let mut is_nullable = true;
            if self.current().ok_or(MokErrors::UnexpectedEof)?.kind == Syntax::Not {
                self.advance();
                if self.current().ok_or(MokErrors::UnexpectedEof)?.kind == Syntax::Null {
                    self.advance();
                    is_nullable = false;
                }
            }
            let mut default = None;
            if self.current().ok_or(MokErrors::UnexpectedEof)?.kind == Syntax::Default {
                self.advance();
                let token = self.consume_syntax(Syntax::Literal)?;
                if token.data_type.is_none() {
                    return Err(MokErrors::InvalidInputType {
                        line: token.line,
                        column: token.column,
                        value: token.lexeme,
                    });
                }

                default = Some(Self::literal_value(token.data_type.unwrap()))
            }

            column_declarations.push(ColumnDeclaration {
                name: column_name.lexeme,
                data_type,
                is_nullable,
                default,
            });
            match self.current() {
                Some(t) if t.kind == Syntax::Comma => {
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(column_declarations)
    }

    fn parse_constraints_declartions(
        &mut self,
    ) -> Result<Option<Vec<TableLevelConstraints>>, MokErrors> {
        let mut table_level_constraints = vec![];
        loop {
            let token = self.current().ok_or(MokErrors::UnexpectedEof)?;

            if !matches!(
                token.kind,
                Syntax::Constraint | Syntax::Primary | Syntax::Unique | Syntax::Foreign
            ) {
                break;
            }

            let mut constraint_name = None;
            let mut token = self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
            if token.kind == Syntax::Constraint {
                let constraint_ident = self.consume_syntax(Syntax::Identifier)?;
                constraint_name = Some(constraint_ident.lexeme);
                token = self.current().ok_or(MokErrors::UnexpectedEof)?.clone();

                if !matches!(
                    token.kind,
                    Syntax::Primary | Syntax::Unique | Syntax::Foreign
                ) {
                    break;
                }
                self.advance();
            }

            if token.kind == Syntax::Primary {
                self.consume_syntax(Syntax::Key)?;
                self.consume_syntax(Syntax::OpenParen)?;
                let pk_col_ident = self.consume_syntax(Syntax::Identifier)?;
                self.consume_syntax(Syntax::CloseParen)?;
                table_level_constraints.push(TableLevelConstraints::PrimaryKey {
                    column_name: pk_col_ident.lexeme,
                    constraint_name,
                });
            } else if token.kind == Syntax::Unique {
                self.consume_syntax(Syntax::Key)?;
                self.consume_syntax(Syntax::OpenParen)?;
                let pk_col_ident = self.consume_syntax(Syntax::Identifier)?;
                self.consume_syntax(Syntax::CloseParen)?;
                table_level_constraints.push(TableLevelConstraints::Unique {
                    column_name: pk_col_ident.lexeme,
                    constraint_name,
                });
            } else if token.kind == Syntax::Foreign {
                self.consume_syntax(Syntax::Key)?;
                self.consume_syntax(Syntax::OpenParen)?;
                let pk_col_ident = self.consume_syntax(Syntax::Identifier)?;
                self.consume_syntax(Syntax::CloseParen)?;
                self.consume_syntax(Syntax::References)?;
                let ref_table_name = self.consume_syntax(Syntax::Identifier)?;
                let mut ref_col_name = None;
                let next_token = self.current();
                if next_token.is_some() && next_token.unwrap().kind == Syntax::OpenParen {
                    self.advance();
                    ref_col_name = Some(self.consume_syntax(Syntax::Identifier)?.lexeme);
                    self.consume_syntax(Syntax::CloseParen)?;
                }
                table_level_constraints.push(TableLevelConstraints::ForeignKey {
                    column_name: pk_col_ident.lexeme,
                    constraint_name,
                    references_table: ref_table_name.lexeme,
                    references_column: ref_col_name,
                });
            }

            match self.current() {
                Some(t) if t.kind == Syntax::Comma => {
                    self.advance();
                }
                _ => break,
            }
        }
        if table_level_constraints.is_empty() {
            return Ok(None);
        }
        Ok(Some(table_level_constraints))
    }
}

impl Parser {
    pub fn parse_script(&mut self) -> Result<Vec<Statement>, MokErrors> {
        let mut stmts = Vec::new();

        while self.current().is_some() {
            // Parse a single statement; let it handle semicolons if required
            let stmt = self.parse_statement(false, true)?;
            stmts.push(stmt);

            // Consume trailing semicolons (optional, depending on require_semicolons)
            while matches!(self.current(), Some(t) if t.kind == Syntax::Semicolon) {
                self.advance();
            }
        }

        // EOF check happens once at the end of the script
        if self.current().is_some() {
            let token = self.current().unwrap();
            return Err(MokErrors::UnexpectedToken {
                expected: "end of input".to_string(),
                found: token.lexeme.clone(),
                line: token.line,
                column: token.column,
            });
        }

        Ok(stmts)
    }

    pub fn parse_statement(
        &mut self,
        mandatory_returning: bool,
        expect_semicolon: bool,
    ) -> Result<Statement, MokErrors> {
        match self.current() {
            Some(token) => match token.kind {
                Syntax::OpenParen => {
                    self.advance();
                    let stmt = self.parse_statement(false, mandatory_returning)?;
                    self.consume_syntax(Syntax::CloseParen)?;
                    Ok(stmt)
                }
                Syntax::Begin => {
                    self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
                    if expect_semicolon {
                        self.consume_syntax(Syntax::Semicolon)?;
                    }
                    Ok(Statement::ExplicitTransaction)
                }
                Syntax::Rollback => {
                    self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
                    self.consume_syntax(Syntax::Semicolon)?;
                    Ok(Statement::ExplicitRollBack)
                }
                Syntax::Commit => {
                    self.consume_current().ok_or(MokErrors::UnexpectedEof)?;
                    self.consume_syntax(Syntax::Semicolon)?;
                    Ok(Statement::ExplicitTransactionCommit)
                }
                Syntax::Select | Syntax::With => self.parse_select_statement(expect_semicolon),
                Syntax::Insert => {
                    self.parse_insert_statement(mandatory_returning, false, expect_semicolon)
                }
                Syntax::Update => {
                    self.parse_update_statement(mandatory_returning, false, expect_semicolon)
                }
                Syntax::Delete => {
                    self.parse_delete_statement(mandatory_returning, false, expect_semicolon)
                }
                Syntax::Create => self.parse_create_statement(false, expect_semicolon),
                Syntax::Drop => self.parse_drop_statement(false, expect_semicolon),
                _ => Err(MokErrors::UnexpectedToken {
                    expected:
                        "select, insert, update, delete, create, drop, or transaction keyword"
                            .to_string(),
                    found: token.lexeme.clone(),
                    line: token.line,
                    column: token.column,
                }),
            },
            None => Err(MokErrors::UnexpectedEof),
        }
    }
}
