use std::{collections::HashSet, slice::Iter};

use crate::{
    err::DatabaseErrors,
    parser_v2::ast::{
        ComparisonOperator, Condition, DeleteStatement, Expression, InValues, InsertStatement,
        Literal, LogicalOperator, NullCheckCondition, SQLStatement, SelectColumn, SelectStatement,
        UpdateStatement,
    },
    storage::catalog_manager::CatalogManager,
};

use super::query_structure::{
    BinaryOp, DataType, Expr, Filter, Insert, Limit, LogicalOp, Project, QueryNodes, Scan, Sorting,
    SortingOrder, Update,
};

pub struct QueryPlanner {
    catalog_manager: CatalogManager,
}

impl QueryPlanner {
    pub fn new(catalog_manager: CatalogManager) -> Self {
        Self { catalog_manager }
    }

    pub fn prepare_logical_query_plan(
        &self,
        ast: SQLStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        match ast {
            SQLStatement::Select(stmt) => self.plan_select(stmt),
            SQLStatement::Insert(stmt) => self.plan_insert(stmt),
            SQLStatement::Update(stmt) => self.plan_update(stmt),
            SQLStatement::Delete(stmt) => self.plan_delete(stmt),
            SQLStatement::Create(_) => todo!("CREATE statement planning"),
            SQLStatement::Drop(_) => todo!("DROP statement planning"),
        }
    }

    // ===== SELECT QUERY PLANNING =====

    fn plan_select(&self, stmt: SelectStatement) -> Result<QueryNodes, DatabaseErrors> {
        let metadata = self.get_table_metadata(&stmt.from)?;
        let projections =
            self.verify_select_columns(&stmt.from, stmt.columns.iter(), &metadata.column_set)?;

        let mut query_node = QueryNodes::Scan(Scan {
            table_name: stmt.from.clone(),
        });

        query_node = self.apply_where_clause(
            query_node,
            stmt.where_clause,
            &stmt.from,
            &metadata.column_set,
        )?;
        query_node = self.apply_limit_offset(query_node, stmt.limit, stmt.offset);
        query_node =
            self.apply_order_by(query_node, stmt.order_by, &stmt.from, &metadata.column_set)?;

        Ok(QueryNodes::Project(Project {
            projections,
            query_node: Box::new(query_node),
        }))
    }

    fn apply_where_clause(
        &self,
        query_node: QueryNodes,
        where_clause: Option<crate::parser_v2::ast::WhereClause>,
        table_name: &str,
        column_set: &HashSet<String>,
    ) -> Result<QueryNodes, DatabaseErrors> {
        let Some(filter) = where_clause else {
            return Ok(query_node);
        };

        let conditions = self.walk_where_clause(filter.condition);
        self.validate_filter_columns(&conditions, table_name, column_set)?;

        Ok(QueryNodes::Filter(Filter {
            conditions,
            scan_node: Box::new(query_node),
        }))
    }

    fn apply_limit_offset(
        &self,
        query_node: QueryNodes,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> QueryNodes {
        if limit.is_some() || offset.is_some() {
            QueryNodes::Limit(Limit {
                limit,
                offset,
                query_node: Box::new(query_node),
            })
        } else {
            query_node
        }
    }

    fn apply_order_by(
        &self,
        query_node: QueryNodes,
        order_by: Option<Vec<crate::parser_v2::ast::OrderByClause>>,
        table_name: &str,
        column_set: &HashSet<String>,
    ) -> Result<QueryNodes, DatabaseErrors> {
        let Some(order) = order_by else {
            return Ok(query_node);
        };

        // Validate all columns exist
        for col in &order {
            if !column_set.contains(&col.column_name) {
                return Err(DatabaseErrors::ColumnNotFoundInTable {
                    column: col.column_name.clone(),
                    table: table_name.to_string(),
                });
            }
        }

        let (column_to_sort, ordering_type): (Vec<_>, Vec<_>) = order
            .into_iter()
            .map(|col| {
                (
                    Expr::Column(col.column_name),
                    Expr::SortingOrder(SortingOrder {
                        is_asc: col.is_asec,
                    }),
                )
            })
            .unzip();

        Ok(QueryNodes::Sort(Sorting {
            column_to_sort,
            ordering_type,
            query_node: Box::new(query_node),
        }))
    }

    // ===== INSERT QUERY PLANNING =====

    fn plan_insert(&self, stmt: InsertStatement) -> Result<QueryNodes, DatabaseErrors> {
        let metadata = self.get_table_metadata(&stmt.table)?;
        self.validate_columns_exist(&stmt.columns, &stmt.table, &metadata.column_set)?;

        let columns = stmt.columns.into_iter().map(Expr::Column).collect();
        let values = stmt
            .values
            .iter()
            .map(|expr| match expr {
                Expression::Literal(lit) => Some(Self::literal_to_data_type(lit)),
                _ => None,
            })
            .collect();

        Ok(QueryNodes::Insert(Insert {
            table_name: stmt.table,
            columns,
            values,
            sub_query: None,
        }))
    }

    // ===== UPDATE QUERY PLANNING =====

    fn plan_update(&self, stmt: UpdateStatement) -> Result<QueryNodes, DatabaseErrors> {
        let metadata = self.get_table_metadata(&stmt.table)?;

        let assignment_columns: Vec<_> = stmt.assignments.iter().map(|a| &a.column).collect();
        self.validate_columns_exist(&assignment_columns, &stmt.table, &metadata.column_set)?;

        let columns = stmt
            .assignments
            .into_iter()
            .map(|assignment| {
                (
                    assignment.column,
                    Self::expression_to_expr(&assignment.value),
                )
            })
            .collect();

        Ok(QueryNodes::Update(Update {
            table_name: stmt.table,
            columns: Some(columns),
            filter: None,
        }))
    }

    // ===== DELETE QUERY PLANNING =====

    fn plan_delete(&self, stmt: DeleteStatement) -> Result<QueryNodes, DatabaseErrors> {
        let metadata = self.get_table_metadata(&stmt.table)?;

        let mut query_node = QueryNodes::Scan(Scan {
            table_name: stmt.table.clone(),
        });

        query_node = self.apply_where_clause(
            query_node,
            stmt.where_clause,
            &stmt.table,
            &metadata.column_set,
        )?;

        Ok(QueryNodes::Project(Project {
            projections: vec![],
            query_node: Box::new(query_node),
        }))
    }

    // ===== WHERE CLAUSE PROCESSING =====

    fn walk_where_clause(&self, condition: Condition) -> Vec<Expr> {
        let mut expressions = Vec::new();
        self.process_condition(condition, &mut expressions);
        expressions
    }

    fn process_condition(&self, condition: Condition, expressions: &mut Vec<Expr>) {
        match condition {
            Condition::Comparison(comp) => {
                expressions.push(Self::expression_to_expr(&comp.left));
                expressions.push(Self::expression_to_expr(&comp.right));
                expressions.push(Self::comparison_op_to_expr(comp.operator));
            }

            Condition::Logical(logical) => {
                expressions.extend(self.walk_where_clause(*logical.left));
                expressions.extend(self.walk_where_clause(*logical.right));
                expressions.push(Self::logical_op_to_expr(logical.operator));
            }

            Condition::Not(condition) => {
                expressions.extend(self.walk_where_clause(*condition));
                expressions.push(Expr::LogicalOp(LogicalOp::Not));
            }

            Condition::NullCheck(null_check) => {
                self.process_null_check(null_check, expressions);
            }

            Condition::In(in_condition) => {
                self.process_in_condition(in_condition, expressions);
            }
        }
    }

    fn process_null_check(&self, null_check: NullCheckCondition, expressions: &mut Vec<Expr>) {
        let (expr, op) = match null_check {
            NullCheckCondition::IsNull(Expression::Identifier(col)) => {
                (Expr::Column(col), Expr::LogicalOp(LogicalOp::IsNull))
            }
            NullCheckCondition::IsNotNull(Expression::Identifier(col)) => {
                (Expr::Column(col), Expr::LogicalOp(LogicalOp::NotNull))
            }
            _ => return, // Skip literals and binary ops for now
        };

        expressions.push(expr);
        expressions.push(op);
    }

    fn process_in_condition(
        &self,
        in_condition: crate::parser_v2::ast::InCondition,
        expressions: &mut Vec<Expr>,
    ) {
        expressions.push(Self::expression_to_expr(&in_condition.left));

        match in_condition.values {
            InValues::List(Some(values)) => {
                let list_exprs = values.iter().map(Self::expression_to_expr).collect();
                expressions.push(Expr::List(list_exprs));
                expressions.push(Expr::LogicalOp(LogicalOp::In));
            }
            InValues::Subquery(Some(subquery)) => {
                if let Ok(query_plan) =
                    self.prepare_logical_query_plan(SQLStatement::Select(*subquery))
                {
                    expressions.push(Expr::SubQuery(Box::new(query_plan)));
                    expressions.push(Expr::LogicalOp(LogicalOp::In));
                }
            }
            _ => {} // Skip None cases
        }
    }

    // ===== HELPER METHODS =====

    fn get_table_metadata(
        &self,
        table_name: &str,
    ) -> Result<&crate::storage::catalog_manager::TableMetadata, DatabaseErrors> {
        self.catalog_manager
            .get_table_metadata(table_name)
            .ok_or_else(|| DatabaseErrors::TableNotFound(table_name.to_string()))
    }

    fn validate_columns_exist<T: AsRef<str>>(
        &self,
        columns: &[T],
        table_name: &str,
        column_set: &HashSet<String>,
    ) -> Result<(), DatabaseErrors> {
        for col in columns {
            if !column_set.contains(col.as_ref()) {
                return Err(DatabaseErrors::ColumnNotFoundInTable {
                    column: col.as_ref().to_string(),
                    table: table_name.to_string(),
                });
            }
        }
        Ok(())
    }

    fn validate_filter_columns(
        &self,
        conditions: &[Expr],
        table_name: &str,
        column_set: &HashSet<String>,
    ) -> Result<(), DatabaseErrors> {
        for expr in conditions {
            if let Expr::Column(col) = expr {
                if !column_set.contains(col) {
                    return Err(DatabaseErrors::ColumnNotFoundInTable {
                        column: col.clone(),
                        table: table_name.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn verify_select_columns(
        &self,
        table_name: &str,
        projected_columns: Iter<SelectColumn>,
        actual_columns: &HashSet<String>,
    ) -> Result<Vec<String>, DatabaseErrors> {
        let mut columns_to_select = Vec::new();

        for col in projected_columns {
            match col {
                SelectColumn::All => {
                    columns_to_select.extend(actual_columns.iter().cloned());
                }
                SelectColumn::Column(name) => {
                    if actual_columns.contains(name) {
                        columns_to_select.push(name.clone());
                    } else {
                        return Err(DatabaseErrors::ColumnNotFoundInTable {
                            column: name.clone(),
                            table: table_name.to_string(),
                        });
                    }
                }
            }
        }

        Ok(columns_to_select)
    }

    // ===== CONVERSION UTILITIES =====

    fn expression_to_expr(expr: &Expression) -> Expr {
        match expr {
            Expression::Identifier(name) => Expr::Column(name.clone()),
            Expression::Literal(lit) => Expr::Const(Self::literal_to_data_type(lit)),
            Expression::BinaryOp { .. } => todo!("Binary operations not yet implemented"),
        }
    }

    fn literal_to_data_type(literal: &Literal) -> DataType {
        match literal {
            Literal::String(s) => DataType::String(s.clone()),
            Literal::Number(n) => DataType::Int(*n),
            Literal::Decimal(d) => DataType::Decimal(*d),
            Literal::Boolean(b) => DataType::Boolean(*b),
        }
    }

    fn comparison_op_to_expr(op: ComparisonOperator) -> Expr {
        let binary_op = match op {
            ComparisonOperator::EqualTo => BinaryOp::Eq,
            ComparisonOperator::NotEqual => BinaryOp::Ne,
            ComparisonOperator::GreaterThan => BinaryOp::Gt,
            ComparisonOperator::LessThan => BinaryOp::Lt,
            ComparisonOperator::GreaterThanOrEqual => BinaryOp::Gte,
            ComparisonOperator::LessThanOrEqual => BinaryOp::Lte,
        };
        Expr::BinaryOp(binary_op)
    }

    fn logical_op_to_expr(op: LogicalOperator) -> Expr {
        let logical_op = match op {
            LogicalOperator::And => LogicalOp::And,
            LogicalOperator::Or => LogicalOp::Or,
        };
        Expr::LogicalOp(logical_op)
    }
}
