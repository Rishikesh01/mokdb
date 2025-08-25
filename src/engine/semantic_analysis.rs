use std::{collections::HashMap, ops::Deref};

use crate::{
    engine::ir::{
        BinaryOp, BinaryOperator, Column, ComparisonOperator, Condition, Cte, DataType, Equality,
        Expression, InList, JoinType, LikeKind, Logical, LogicalOperator, OrderBy, OrderDirection,
        ProjectionsItem, QueryIR, Select, TableRef, ValueType,
    },
    errors::MokErrors,
    parser::ast::Statement,
    storage::catalog_manager::CatalogManager,
};

#[derive(Default)]
pub struct Scope<'a> {
    tables: HashMap<String, Vec<Column>>,
    ctes: HashMap<String, Vec<Column>>,
    table_and_cte_alias: HashMap<String, String>,
    parent_scope: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn new_child(scope: &'a Scope<'a>) -> Self {
        Self {
            tables: HashMap::new(),
            ctes: HashMap::new(),
            table_and_cte_alias: HashMap::new(),
            parent_scope: Some(scope),
        }
    }

    pub fn get_column_details(
        &self,
        column_name: &String,
        table_name_or_alias: Option<String>,
    ) -> Result<(String, Column), MokErrors> {
        if table_name_or_alias.is_none() {
            for (table_name, columns) in self.tables.iter() {
                if let Some(column) = columns.iter().find(|x| x.name == column_name.deref()) {
                    return Ok((table_name.clone(), column.clone()));
                }
            }
            for (table_name, columns) in self.ctes.iter() {
                if let Some(column) = columns.iter().find(|x| x.name == column_name.deref()) {
                    return Ok((table_name.clone(), column.clone()));
                }
            }
            if let Some(scope) = self.parent_scope {
                return scope.get_column_details(column_name, table_name_or_alias);
            }
        }
        let table_name_ref = &table_name_or_alias.clone().unwrap();
        let table_name = self
            .table_and_cte_alias
            .get(table_name_ref)
            .unwrap_or(table_name_ref);

        if let Some(columns) = self.ctes.get(table_name) {
            if let Some(column) = columns.iter().find(|x| x.name == column_name.deref()) {
                return Ok((table_name.clone(), column.clone()));
            }
        }
        if let Some(columns) = self.tables.get(table_name) {
            if let Some(column) = columns.iter().find(|x| x.name == column_name.deref()) {
                return Ok((table_name.clone(), column.clone()));
            }
        }

        if let Some(scope) = self.parent_scope {
            return scope.get_column_details(column_name, table_name_or_alias);
        }

        Err(MokErrors::UnknownColumnProvided {
            column_name: column_name.clone(),
            table_name: table_name_ref.clone(),
        })
    }
}

pub struct SemanticAnalyzer<T>
where
    T: CatalogManager,
{
    catalog_manager: T,
}

impl<T> SemanticAnalyzer<T>
where
    T: CatalogManager,
{
    pub fn new(catalog_manager: T) -> Self {
        Self { catalog_manager }
    }
    pub fn analyze<'a>(&self, mut scope: Scope<'a>, ast: Statement) -> Result<QueryIR, MokErrors> {
        match ast {
            Statement::SelectStmt(select) => self.analyze_select_statement(&mut scope, select),
            Statement::InsertStmt(insert) => self.analyze_insert_statement(&mut scope, insert),
            Statement::UpdateStmt(update) => self.analyze_update_statement(&mut scope, update),
            Statement::DeleteSmt(delete) => self.analyze_delete_statement(&mut scope, delete),
            Statement::DropStmt(drop) => self.analyze_drop_statement(&mut scope, drop),
            Statement::CreateStmt(create) => self.analyze_create_statement(&mut scope, create),
            Statement::ExplicitTransaction => Err(MokErrors::ShouldNotReachHere),
            Statement::ExplicitTransactionCommit => Err(MokErrors::ShouldNotReachHere),
            Statement::ExplicitRollBack => Err(MokErrors::ShouldNotReachHere),
        }
    }

    /*
     * Things to check
     * 1. table name, does it resolved to actual table or an CTE also is the table name actually
     *    inscope
     * 2. check if columns or expressions mentioned in projection, order by, group by clause valid
     *    i.e if it's a columns does it actually exists in the table and if it's a expression is
     *    the type of op done on expression valid i.e col_of_int_type can do binary op only
     * 3. are expressions in condition and in join on key valid.
     *
     */

    fn analyze_select_statement<'a>(
        &self,
        scope: &mut Scope<'a>,
        select: crate::parser::ast::Select,
    ) -> Result<QueryIR, MokErrors> {
        let with = if let Some(ctes) = select.with {
            Some(self.analyze_cte(scope, ctes)?)
        } else {
            None
        };

        let from = self.analyze_from(scope, select.from)?;
        let distinct = if let Some(distinct_columns) = select.distinct {
            Some(self.analyze_distinct(scope, distinct_columns)?)
        } else {
            None
        };
        let projections = self.analyze_projections(scope, select.projection)?;
        let where_clause = if let Some(ast_where_clause) = select.where_clause {
            Some(self.analyze_condition(scope, ast_where_clause)?)
        } else {
            None
        };

        let group_by = if let Some(grouping) = select.group_by {
            let grouped_result = grouping
                .into_iter()
                .map(|x| self.analyze_expression(scope, x))
                .collect::<Result<Vec<(Expression, ValueType)>, MokErrors>>()?;
            Some(grouped_result)
        } else {
            None
        };

        let order_by = if let Some(ordering) = select.order_by {
            let mut items = vec![];
            for order_item in ordering {
                let order_exp = self.analyze_expression(scope, order_item.expr)?;
                items.push(OrderBy {
                    expr: order_exp,
                    order: OrderDirection::map_ast(order_item.order),
                });
            }
            Some(items)
        } else {
            None
        };

        Ok(QueryIR::SelectStmt(Select {
            with,
            distinct,
            projection: projections,
            from,
            where_clause,
            group_by,
            order_by,
            limit: select.limit,
            offset: select.offset,
        }))
    }

    fn analyze_from<'a>(
        &self,
        scope: &mut Scope<'a>,
        from: crate::parser::ast::TableRef,
    ) -> Result<TableRef, MokErrors> {
        match from {
            crate::parser::ast::TableRef::Table { name, alias } => {
                if let Some(table) = self.catalog_manager.get_table(&name) {
                    let cols = table.get_columns().clone();
                    scope.tables.insert(name.clone(), cols);
                    if let Some(alias) = alias.as_ref() {
                        scope
                            .table_and_cte_alias
                            .insert(alias.clone(), name.clone());
                    }
                    return Ok(TableRef::Table { name, alias });
                }

                Err(MokErrors::UnknownDataSource { table_name: name })
            }
            crate::parser::ast::TableRef::SubQuery { query, alias } => {
                let query = match self.analyze_select_statement(scope, *query)? {
                    QueryIR::SelectStmt(select) => Ok(select),
                    QueryIR::InsertStmt(_) => Err(MokErrors::InvalidSubQuery {
                        stmt_type: "insert".into(),
                    }),
                    QueryIR::UpdateStmt(_) => Err(MokErrors::InvalidSubQuery {
                        stmt_type: "update".into(),
                    }),
                    QueryIR::DeleteSmt(_) => Err(MokErrors::InvalidSubQuery {
                        stmt_type: "delete".into(),
                    }),
                    QueryIR::DropStmt(_) => Err(MokErrors::InvalidSubQuery {
                        stmt_type: "drop".into(),
                    }),
                    QueryIR::CreateStmt(_) => Err(MokErrors::InvalidSubQuery {
                        stmt_type: "create".into(),
                    }),
                }?;
                let columns = query
                    .projection
                    .iter()
                    .map(|p| Column {
                        name: p.alias.clone().unwrap_or_else(|| {
                            if let Expression::Identifier { name, .. } = &p.expr {
                                name.clone()
                            } else {
                                "".into()
                            }
                        }),
                        data_type: p.data_type,
                        constraints: None,
                        table: None,
                        cte: alias.clone().into(),
                        value: None,
                    })
                    .collect();
                scope.tables.insert(alias.clone(), columns);
                Ok(TableRef::SubQuery {
                    query: Box::new(query),
                    alias,
                })
            }
            crate::parser::ast::TableRef::Join {
                left,
                right,
                join_type,
                on,
            } => {
                let left = Box::new(self.analyze_from(scope, *left)?);
                let right = Box::new(self.analyze_from(scope, *right)?);
                let on = self.analyze_condition(scope, on)?;
                Ok(TableRef::Join {
                    left,
                    right,
                    join_type: JoinType::map_ast(join_type),
                    on: Box::new(on),
                })
            }
        }
    }

    fn analyze_condition(
        &self,
        scope: &mut Scope,
        on: crate::parser::ast::Condition,
    ) -> Result<Condition, MokErrors> {
        match on {
            crate::parser::ast::Condition::Comparison(equality) => {
                let (lhs_exp, lhs_data_type) = self.analyze_expression(scope, equality.lhs)?;
                let (rhs_exp, rhs_data_type) = self.analyze_expression(scope, equality.rhs)?;
                let op = ComparisonOperator::map_asst(equality.operator);
                let (is_valid_lhs, is_valid_rhs) =
                    op.is_valid_operand(&lhs_data_type, &rhs_data_type);

                if !is_valid_lhs {
                    return Err(MokErrors::IncompatibleOperandsForComparsionOp {
                        op_name: op.to_string(),
                        op_side: format!(
                            "lhs side data_type is {}, and op being performed is not valid for it",
                            lhs_data_type
                        ),
                    });
                }

                if !is_valid_rhs {
                    return Err(MokErrors::IncompatibleOperandsForComparsionOp {
                        op_name: op.to_string(),
                        op_side: format!(
                            "rhs side data_type is {}, and op being performed is not valid for it",
                            rhs_data_type
                        ),
                    });
                }

                Ok(Condition::Comparison(Equality {
                    lhs: lhs_exp,
                    rhs: rhs_exp,
                    operator: op,
                }))
            }
            crate::parser::ast::Condition::Logical(logical) => {
                let lhs = self.analyze_condition(scope, *logical.lhs)?;
                let rhs = self.analyze_condition(scope, *logical.rhs)?;
                let op = LogicalOperator::map_ast(logical.operator);
                Ok(Condition::Logical(Logical {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    operator: op,
                }))
            }
            crate::parser::ast::Condition::In { expr, set } => {
                let (expr, lhs_data_type) = self.analyze_expression(scope, expr)?;
                let set = match set {
                    crate::parser::ast::InList::Expressions(expressions) => {
                        let mut exprs = vec![];
                        for exp in expressions.into_iter() {
                            let (exp, data_type) = self.analyze_expression(scope, exp)?;
                            if data_type != lhs_data_type {
                                return Err(MokErrors::IncompatibleRhsDatatypeInClause {
                                    lhs_data_type: lhs_data_type.to_string(),
                                    rhs_data_type: data_type.to_string(),
                                });
                            }
                            exprs.push((exp, data_type));
                        }
                        InList::Expressions(exprs)
                    }
                    crate::parser::ast::InList::SubQuery(select) => {
                        let QueryIR::SelectStmt(select) =
                            self.analyze_select_statement(scope, *select)?
                        else {
                            unreachable!("expected SelectStmt");
                        };
                        InList::SubQuery(Box::new(select))
                    }
                };
                Ok(Condition::In { expr, set })
            }
            crate::parser::ast::Condition::Exists(select) => {
                let QueryIR::SelectStmt(select) = self.analyze_select_statement(scope, *select)?
                else {
                    unreachable!("expected SelectStmt");
                };
                Ok(Condition::Exists(Box::new(select)))
            }
            crate::parser::ast::Condition::Not(condition) => Ok(Condition::Not(Box::new(
                self.analyze_condition(scope, *condition)?,
            ))),
            crate::parser::ast::Condition::IsNull(expression) => Ok(Condition::IsNull(
                self.analyze_expression(scope, expression)?,
            )),
            crate::parser::ast::Condition::IsNotNull(expression) => Ok(Condition::IsNotNull(
                self.analyze_expression(scope, expression)?,
            )),
            crate::parser::ast::Condition::Like {
                expr,
                pattern,
                kind,
            } => {
                let (expr, lhs_data_type) = self.analyze_expression(scope, expr)?;
                let (pattern, rhs_data_type) = self.analyze_expression(scope, pattern)?;
                if lhs_data_type != rhs_data_type {
                    return Err(MokErrors::IncompatibleRhsDatatypeInClause {
                        lhs_data_type: lhs_data_type.to_string(),
                        rhs_data_type: rhs_data_type.to_string(),
                    });
                }
                Ok(Condition::Like {
                    expr,
                    pattern,
                    kind: LikeKind::map_ast(kind),
                })
            }
        }
    }

    fn analyze_expression(
        &self,
        scope: &mut Scope,
        exp: crate::parser::ast::Expression,
    ) -> Result<(Expression, ValueType), MokErrors> {
        match exp {
            crate::parser::ast::Expression::Identifier { table, name } => {
                let mut table_and_column = scope.get_column_details(&name, table.clone());
                if table_and_column.is_err() {
                    if let Some(table_name) = &table {
                        let table_obj = self.catalog_manager.get_table(table_name.as_str()).ok_or(
                            MokErrors::UnknownDataSource {
                                table_name: table_name.clone(),
                            },
                        )?;

                        scope
                            .tables
                            .insert(table_name.clone(), table_obj.get_columns());
                        table_and_column = scope.get_column_details(&name, table.clone());
                    }
                }

                let (table_name, column) = table_and_column?;

                Ok((
                    Expression::Identifier {
                        table: table_name.clone(),
                        name,
                        constraints: None,
                    },
                    column.data_type,
                ))
            }
            crate::parser::ast::Expression::Literal { data_type, alias } => {
                let data_type = DataType::map_ast(data_type);
                Ok((
                    Expression::Literal {
                        alias,
                        data_type: data_type.clone(),
                    },
                    ValueType::map_data_type(&data_type),
                ))
            }
            crate::parser::ast::Expression::BinaryOp(binary_op) => {
                let lhs = self.analyze_expression(scope, binary_op.lhs)?;
                let rhs = self.analyze_expression(scope, binary_op.rhs)?;
                let operator = BinaryOperator::map_ast(binary_op.operator);
                let op_valid = operator.is_valid_operand(&lhs.1, &rhs.1);
                if !op_valid.0 {
                    return Err(MokErrors::IncompatibleOperandsForBinaryOp {
                        op_name: operator.to_string(),
                        operand_side: format!(
                            "lhs side data_type is {}, and op being performed is not valid for it",
                            lhs.1
                        ),
                    });
                }

                if !op_valid.1 {
                    return Err(MokErrors::IncompatibleOperandsForBinaryOp {
                        op_name: operator.to_string(),
                        operand_side: format!(
                            "rhs side data_type is {}, and op being performed is not valid for it",
                            rhs.1
                        ),
                    });
                }

                let data_type = lhs.1;
                Ok((
                    Expression::BinaryOp(Box::new(BinaryOp { lhs, rhs, operator })),
                    data_type,
                ))
            }
        }
    }

    fn analyze_cte<'a>(
        &self,
        scope: &mut Scope<'a>,
        ctes: Vec<crate::parser::ast::Cte>,
    ) -> Result<Vec<Cte>, MokErrors> {
        let mut ctes_ir: Vec<Cte> = vec![];
        for cte in ctes {
            let child_scope = Scope::new_child(scope);
            let ir = self.analyze(child_scope, *cte.query)?;
            match &ir {
                QueryIR::SelectStmt(select) => {
                    let mut columns = vec![];
                    for item in &select.projection {
                        let col = match &item.expr {
                            crate::engine::ir::Expression::Identifier {
                                table,
                                name,
                                constraints,
                            } => Column {
                                name: name.clone(),
                                data_type: item.data_type,
                                constraints: constraints.clone(),
                                table: None,
                                cte: Some(cte.name.clone()),
                                value: None,
                            },
                            crate::engine::ir::Expression::Literal { alias, data_type } => Column {
                                table: None,
                                cte: Some(cte.name.clone()),
                                name: alias.clone().unwrap_or_else(|| format!("{:?}", data_type)),
                                data_type: item.data_type,
                                constraints: None,
                                value: Some(data_type.clone()),
                            },
                            crate::engine::ir::Expression::BinaryOp(binary_op) => todo!(),
                        };
                        columns.push(col);
                    }
                    scope.ctes.insert(cte.name.clone(), columns);
                }
                QueryIR::InsertStmt(insert) => {
                    scope.ctes.insert(cte.name.clone(), vec![]);
                }
                QueryIR::UpdateStmt(update) => {
                    scope.ctes.insert(cte.name.clone(), vec![]);
                }
                QueryIR::DeleteSmt(delete) => {
                    scope.ctes.insert(cte.name.clone(), vec![]);
                }
                QueryIR::DropStmt(drop) => panic!("drop"),
                QueryIR::CreateStmt(create) => panic!("create"),
            };
            ctes_ir.push(Cte {
                name: cte.name,
                query: Box::new(ir),
            });
        }

        Ok(ctes_ir)
    }

    fn analyze_insert_statement(
        &self,
        scope: &mut Scope,
        insert: crate::parser::ast::Insert,
    ) -> Result<QueryIR, MokErrors> {
        todo!()
    }

    fn analyze_update_statement(
        &self,
        scope: &mut Scope,
        update: crate::parser::ast::Update,
    ) -> Result<QueryIR, MokErrors> {
        todo!()
    }

    fn analyze_delete_statement(
        &self,
        scope: &mut Scope,
        delete: crate::parser::ast::Delete,
    ) -> Result<QueryIR, MokErrors> {
        todo!()
    }

    fn analyze_drop_statement(
        &self,
        scope: &mut Scope,
        drop: crate::parser::ast::Drop,
    ) -> Result<QueryIR, MokErrors> {
        todo!()
    }

    fn analyze_create_statement(
        &self,
        scope: &mut Scope,
        create: crate::parser::ast::Create,
    ) -> Result<QueryIR, MokErrors> {
        todo!()
    }

    fn analyze_projections(
        &self,
        scope: &mut Scope,
        projection: crate::parser::ast::Projections,
    ) -> Result<Vec<ProjectionsItem>, MokErrors> {
        match projection {
            crate::parser::ast::Projections::All => {
                let mut items = vec![];
                for (table, columns) in &scope.tables {
                    for col in columns {
                        items.push(ProjectionsItem {
                            expr: Expression::Identifier {
                                table: table.clone(),
                                name: col.name.clone(),
                                constraints: col.constraints.clone(),
                            },
                            data_type: col.data_type,
                            alias: None,
                        });
                    }
                }
                for (table, columns) in &scope.ctes {
                    for col in columns {
                        items.push(ProjectionsItem {
                            expr: Expression::Identifier {
                                table: table.clone(),
                                name: col.name.clone(),
                                constraints: col.constraints.clone(),
                            },
                            data_type: col.data_type,
                            alias: None,
                        });
                    }
                }
                Ok(items)
            }
            crate::parser::ast::Projections::Specified(projection_items) => {
                let mut items = vec![];
                for item in projection_items {
                    let (exp, data_type) = self.analyze_expression(scope, item.expr)?;
                    items.push(ProjectionsItem {
                        expr: exp,
                        alias: item.alias,
                        data_type,
                    });
                }

                Ok(items)
            }
        }
    }

    fn analyze_distinct(
        &self,
        scope: &mut Scope,
        projection: crate::parser::ast::Distinct,
    ) -> Result<Vec<Column>, MokErrors> {
        match projection {
            crate::parser::ast::Distinct::All => {
                Ok(scope.tables.values().flatten().cloned().collect::<Vec<_>>())
            }
            crate::parser::ast::Distinct::On(expressions) => {
                let mut exps = vec![];
                for exp in expressions {
                    let (exp, _) = self.analyze_expression(scope, exp)?;
                    match exp {
                        Expression::Identifier {
                            table,
                            name,
                            constraints,
                        } => {
                            let (_, column) = scope.get_column_details(&name, Some(table))?;
                            exps.push(column);
                        }
                        _ => return Err(MokErrors::InvalidDistinctOnClause),
                    }
                }

                Ok(exps)
            }
        }
    }
}
