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
        &mut self,
        ast: SQLStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        match ast {
            SQLStatement::Select(select_statement) => self.projection(select_statement),
            SQLStatement::Insert(insert_statement) => self.insertion_of_rows(insert_statement),
            SQLStatement::Update(update_statement) => self.updation_of_rows(update_statement),
            SQLStatement::Delete(delete_statement) => self.deletion_of_rows(delete_statement),
            SQLStatement::Create(create_statement) => todo!(),
            SQLStatement::Drop(drop_statement) => todo!(),
        }
    }

    fn projection(
        &mut self,
        select_statement: SelectStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        {
            let metadata = self
                .catalog_manager
                .get_table_metadata(&select_statement.from)
                .ok_or(DatabaseErrors::TableNotFound(select_statement.from.clone()))?;

            let columns_to_select = Self::verify_select_column_exists(
                &select_statement.from,
                select_statement.columns.iter(),
                &metadata.column_set,
            )?;

            let mut query_node = QueryNodes::Scan(Scan {
                table_name: select_statement.from.clone(),
            });

            let column_set = metadata.column_set.clone();
            if let Some(filter) = select_statement.where_clause {
                let columns_to_filter_on = self.walk_where_clause(filter.condition);

                for col in columns_to_filter_on.iter().filter_map(|expr| {
                    if let Expr::Column(col) = expr {
                        Some(col)
                    } else {
                        None
                    }
                }) {
                    if !column_set.contains(col) {
                        return Err(DatabaseErrors::ColumnNotFoundInTable {
                            column: col.to_string(),
                            table: select_statement.from.clone(),
                        });
                    }
                }

                query_node = QueryNodes::Filter(Filter {
                    conditions: columns_to_filter_on,
                    scan_node: Box::new(query_node),
                });
            }

            if select_statement.limit.is_some() || select_statement.offset.is_some() {
                query_node = QueryNodes::Limit(Limit {
                    limit: select_statement.limit,
                    offset: select_statement.offset,
                    query_node: Box::new(query_node),
                })
            }

            if let Some(order) = select_statement.order_by {
                for col in order.iter() {
                    if !column_set.contains(&col.column_name.clone()) {
                        return Err(DatabaseErrors::ColumnNotFoundInTable {
                            column: col.column_name.to_string(),
                            table: select_statement.from.clone(),
                        });
                    }
                }

                query_node = QueryNodes::Sort(Sorting {
                    column_to_sort: order
                        .iter()
                        .map(|x| Expr::Column(x.column_name.clone()))
                        .collect(),
                    ordering_type: order
                        .iter()
                        .map(|x| Expr::SortingOrder(SortingOrder { is_asc: x.is_asec }))
                        .collect(),
                    query_node: Box::new(query_node),
                });
            }

            return Ok(QueryNodes::Project(Project {
                projections: columns_to_select,
                query_node: Box::new(query_node),
            }));
        }
    }

    fn insertion_of_rows(
        &mut self,
        insert_statement: InsertStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        if let Some(metadata) = self
            .catalog_manager
            .get_table_metadata(&insert_statement.table)
        {
            for col in &insert_statement.columns {
                if !metadata.column_set.contains(col) {
                    return Err(DatabaseErrors::ColumnNotFoundInTable {
                        column: col.to_string(),
                        table: insert_statement.table.clone(),
                    });
                }
            }
            let columns = insert_statement
                .columns
                .iter()
                .map(|x| Expr::Column(x.clone()))
                .collect();

            return Ok(QueryNodes::Insert(Insert {
                table_name: insert_statement.table.clone(),
                columns,
                values: insert_statement
                    .values
                    .iter()
                    .map(|e| {
                        if let Expression::Literal(lit) = e {
                            Some(Self::expression_literal_to_data_type(lit))
                        } else {
                            None
                        }
                    })
                    .collect(),
                sub_query: None,
            }));
        }

        Err(DatabaseErrors::TableNotFound(
            insert_statement.table.clone(),
        ))
    }

    fn updation_of_rows(
        &mut self,
        update_statement: UpdateStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        if let Some(metadata) = self
            .catalog_manager
            .get_table_metadata(&update_statement.table)
        {
            for col in &update_statement.assignments {
                if !metadata.column_set.contains(&col.column) {
                    return Err(DatabaseErrors::ColumnNotFoundInTable {
                        column: col.column.to_string(),
                        table: update_statement.table.clone(),
                    });
                }
            }

            let columns = update_statement
                .assignments
                .into_iter()
                .map(|e| (e.column, Self::expression_to_expr(&e.value)))
                .collect();
            return Ok(QueryNodes::Update(Update {
                table_name: update_statement.table.clone(),
                columns: Some(columns),
                filter: None,
            }));
        }
        Err(DatabaseErrors::TableNotFound(update_statement.table))
    }

    fn deletion_of_rows(
        &mut self,
        delete_statement: DeleteStatement,
    ) -> Result<QueryNodes, DatabaseErrors> {
        if let Some(metadata) = self
            .catalog_manager
            .get_table_metadata(&delete_statement.table)
        {
            let mut query_node = QueryNodes::Scan(Scan {
                table_name: delete_statement.table.clone(),
            });

            let column_set = metadata.column_set.clone();

            if let Some(filter) = delete_statement.where_clause {
                let conditions = self.walk_where_clause(filter.condition);
                for col in conditions.iter().filter_map(|expr| {
                    if let Expr::Column(col) = expr {
                        Some(col)
                    } else {
                        None
                    }
                }) {
                    if !column_set.contains(col) {
                        return Err(DatabaseErrors::ColumnNotFoundInTable {
                            column: col.clone(),
                            table: delete_statement.table.clone(),
                        });
                    }
                }

                query_node = QueryNodes::Filter(Filter {
                    conditions,
                    scan_node: Box::new(query_node),
                });
            }

            return Ok(QueryNodes::Project(Project {
                projections: vec![],
                query_node: Box::new(query_node),
            }));
        }
        Err(DatabaseErrors::TableNotFound(delete_statement.table))
    }

    fn walk_where_clause(&mut self, condition: Condition) -> Vec<Expr> {
        let mut filter_conditions = vec![];
        self.condition_walker(condition, &mut filter_conditions);
        filter_conditions
    }

    fn condition_walker(&mut self, condition: Condition, filter_conditions: &mut Vec<Expr>) {
        match condition {
            Condition::Comparison(comparison_condition) => {
                filter_conditions.push(match comparison_condition.left {
                    Expression::Identifier(e) => Expr::Column(e),
                    Expression::Literal(e) => {
                        Expr::Const(Self::expression_literal_to_data_type(&e))
                    }
                    Expression::BinaryOp { left, right, op } => todo!(),
                });
                filter_conditions.push(match comparison_condition.right {
                    Expression::Identifier(e) => Expr::Column(e),
                    Expression::Literal(e) => {
                        Expr::Const(Self::expression_literal_to_data_type(&e))
                    }
                    Expression::BinaryOp { left, right, op } => todo!(),
                });
                filter_conditions.push(match comparison_condition.operator {
                    ComparisonOperator::EqualTo => Expr::BinaryOp(BinaryOp::Eq),
                    ComparisonOperator::NotEqual => Expr::BinaryOp(BinaryOp::Ne),
                    ComparisonOperator::GreaterThan => Expr::BinaryOp(BinaryOp::Gt),
                    ComparisonOperator::LessThan => Expr::BinaryOp(BinaryOp::Lt),
                    ComparisonOperator::GreaterThanOrEqual => Expr::BinaryOp(BinaryOp::Gte),
                    ComparisonOperator::LessThanOrEqual => Expr::BinaryOp(BinaryOp::Lte),
                });
            }
            Condition::Logical(logical_condition) => {
                filter_conditions.append(&mut self.walk_where_clause(*logical_condition.left));
                filter_conditions.append(&mut self.walk_where_clause(*logical_condition.right));
                filter_conditions.push(match logical_condition.operator {
                    LogicalOperator::And => Expr::LogicalOp(LogicalOp::And),
                    LogicalOperator::Or => Expr::LogicalOp(LogicalOp::Or),
                });
            }
            Condition::Not(condition) => {
                filter_conditions.append(&mut self.walk_where_clause(*condition));
                filter_conditions.push(Expr::LogicalOp(LogicalOp::Not));
            }
            Condition::NullCheck(null_check_condition) => match null_check_condition {
                NullCheckCondition::IsNull(expression) => match expression {
                    Expression::Identifier(e) => {
                        filter_conditions.push(Expr::Column(e));
                        filter_conditions.push(Expr::LogicalOp(LogicalOp::IsNull));
                    }
                    Expression::Literal(_) => (),
                    Expression::BinaryOp { left, right, op } => todo!(),
                },
                NullCheckCondition::IsNotNull(expression) => match expression {
                    Expression::Identifier(e) => {
                        filter_conditions.push(Expr::Column(e));
                        filter_conditions.push(Expr::LogicalOp(LogicalOp::NotNull));
                    }
                    Expression::Literal(_) => (),
                    Expression::BinaryOp { left, right, op } => todo!(),
                },
            },

            Condition::In(in_condition) => {
                let left_expr = Self::expression_to_expr(&in_condition.left);
                filter_conditions.push(left_expr);

                match in_condition.values {
                    InValues::List(Some(values)) => {
                        let list_exprs = values.iter().map(Self::expression_to_expr).collect();
                        filter_conditions.push(Expr::List(list_exprs));
                        filter_conditions.push(Expr::LogicalOp(LogicalOp::In));
                    }
                    InValues::Subquery(Some(subquery)) => {
                        filter_conditions.push(Expr::SubQuery(Box::new(
                            self.prepare_logical_query_plan(SQLStatement::Select(*subquery))
                                .unwrap(),
                        )));
                        filter_conditions.push(Expr::LogicalOp(LogicalOp::In));
                    }
                    _ => (),
                }
            }
        }
    }

    fn expression_to_expr(value: &Expression) -> Expr {
        match value {
            Expression::Identifier(s) => Expr::Column(s.clone()),
            Expression::Literal(lit) => Expr::Const(Self::expression_literal_to_data_type(lit)),
            Expression::BinaryOp { left, right, op } => todo!(),
        }
    }

    fn expression_literal_to_data_type(value: &Literal) -> DataType {
        match value {
            Literal::String(s) => DataType::String(s.clone()),
            Literal::Number(n) => DataType::Int(*n),
            Literal::Decimal(d) => DataType::Decimal(*d),
            Literal::Boolean(b) => DataType::Boolean(*b),
        }
    }
    fn verify_select_column_exists(
        table_name: &String,
        projected_columns: Iter<SelectColumn>,
        actual_columns: &HashSet<String>,
    ) -> Result<Vec<String>, DatabaseErrors> {
        let mut columns_to_select = vec![];
        for col in projected_columns {
            match col {
                SelectColumn::All => columns_to_select.extend(actual_columns.iter().cloned()),
                SelectColumn::Column(v) => {
                    if actual_columns.contains(v) {
                        columns_to_select.push(v.clone());
                    }
                    return Err(DatabaseErrors::ColumnNotFoundInTable {
                        column: v.to_string(),
                        table: table_name.clone(),
                    });
                }
            }
        }
        Ok(columns_to_select)
    }
}
