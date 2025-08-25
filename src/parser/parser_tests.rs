use crate::{
    errors::MokErrors,
    parser::{lexer::Lexer, parser::Parser},
};

use super::ast::*;
use pretty_assertions::assert_eq;

#[derive(Debug)]
struct SqlQuery<'a> {
    number: u64,
    name: &'a str,
    stmt: &'a str,
    expected: Result<Vec<Statement>, MokErrors>,
}

#[test]
fn test_drop_statement() {
    let tests = vec![SqlQuery {
        number: 0,
        name: "drop table",
        stmt: "DRoP \"order\";",
        expected: Ok(vec![Statement::DropStmt(Drop {
            table: "order".into(),
        })]),
    }];
    assert_parser_expectation(tests)
}

#[test]
fn test_insert_statement() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic insert with values",
            stmt: "INSERT INTO users (id, name, active) VALUES (1, 'foo', true);",
            expected: Ok(vec![Statement::InsertStmt(Insert {
                table: "users".into(),
                columns: vec!["id".into(), "name".into(), "active".into()],
                sub_select: None,
                values: Some(vec![
                    Expression::Literal{data_type: DataType::Int(1),alias: None},
                    Expression::Literal{data_type:DataType::Text("foo".into()),alias:None},
                    Expression::Literal{data_type:DataType::Boolean(true),alias:None},
                ]),
                returning: None,
            })]),
        },
        SqlQuery {
            number: 1,
            name: "insert from select subquery",
            stmt: "INSERT INTO archive_users (id, name) SELECT id, name FROM users WHERE active = false;",
            expected: Ok(vec![Statement::InsertStmt(Insert {
                table: "archive_users".into(),
                columns: vec!["id".into(), "name".into()],
                sub_select: Some(Select {
                    with: None,
                    distinct: None,
                    projection: Projections::Specified(vec![
                        ProjectionsItem {
                            expr: Expression::Identifier { table: None, name: "id".into() },
                            alias: None,
                        },
                        ProjectionsItem {
                            expr: Expression::Identifier { table: None, name: "name".into() },
                            alias: None,
                        },
                    ]),
                    from: TableRef::Table { name: "users".into(), alias: None },
                    where_clause: Some(Condition::Comparison(Equality {
                        lhs: Expression::Identifier { table: None, name: "active".into() },
                        rhs: Expression::Literal{data_type: DataType::Boolean(false),alias:None},
                        operator: ComparisonOperator::Eq,
                    })),
                    group_by: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                }),
                values: None,
                returning: None,
            })]),
        },
    ];
    assert_parser_expectation(tests);
}

#[test]
fn test_update_statement() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic update with where",
            stmt: "UPDATE users SET name = 'bar' WHERE id = 1;",
            expected: Ok(vec![Statement::UpdateStmt(Update {
                table: "users".into(),
                assignments: vec![(
                    "name".into(),
                    Expression::Literal{data_type: DataType::Text("bar".into()),alias:None},
                )],
                where_clause: Some(Condition::Comparison(Equality {
                    lhs: Expression::Identifier {
                        table: None,
                        name: "id".into(),
                    },
                    rhs: Expression::Literal{data_type:DataType::Int(1),alias:None},
                    operator: ComparisonOperator::Eq,
                })),
                returning: None,
            })]),
        },
        SqlQuery {
            number: 1,
            name: "update with returning",
            stmt: "UPDATE users SET active = false RETURNING id, name;",
            expected: Ok(vec![Statement::UpdateStmt(Update {
                table: "users".into(),
                assignments: vec![(
                    "active".into(),
                    Expression::Literal{data_type:DataType::Boolean(false),alias:None},
                )],
                where_clause: None,
                returning: Some(vec![
                    Expression::Identifier {
                        table: None,
                        name: "id".into(),
                    },
                    Expression::Identifier {
                        table: None,
                        name: "name".into(),
                    },
                ]),
            })]),
        },
    ];
    assert_parser_expectation(tests);
}

#[test]
fn test_delete_statement() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic delete",
            stmt: "DELETE FROM users WHERE id = 42;",
            expected: Ok(vec![Statement::DeleteSmt(Delete {
                table: TableRef::Table {
                    name: "users".into(),
                    alias: None,
                },
                where_clause: Some(Condition::Comparison(Equality {
                    lhs: Expression::Identifier {
                        table: None,
                        name: "id".into(),
                    },
                    rhs: Expression::Literal{data_type:DataType::Int(42),alias:None},
                    operator: ComparisonOperator::Eq,
                })),
                returning: None,
            })]),
        },
        SqlQuery {
            number: 1,
            name: "delete with returning",
            stmt: "DELETE FROM orders WHERE status = 'cancelled' RETURNING id;",
            expected: Ok(vec![Statement::DeleteSmt(Delete {
                table: TableRef::Table {
                    name: "orders".into(),
                    alias: None,
                },
                where_clause: Some(Condition::Comparison(Equality {
                    lhs: Expression::Identifier {
                        table: None,
                        name: "status".into(),
                    },
                    rhs: Expression::Literal{data_type:DataType::Text("cancelled".into()),alias:None},
                    operator: ComparisonOperator::Eq,
                })),
                returning: Some(vec![Expression::Identifier {
                    table: None,
                    name: "id".into(),
                }]),
            })]),
        },
    ];
    assert_parser_expectation(tests);
}

#[test]
fn test_create_statement() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic create table",
            stmt: "CREATE TABLE users (id INT, name TEXT NOT NULL, active BOOLEAN DEFAULT true);",
            expected: Ok(vec![Statement::CreateStmt(Create {
                table: "users".into(),
                columns: vec![
                    ColumnDeclaration {
                        name: "id".into(),
                        data_type: ValueType::Int,
                        is_nullable: true,
                        default: None,
                    },
                    ColumnDeclaration {
                        name: "name".into(),
                        data_type: ValueType::Text,
                        is_nullable: false,
                        default: None,
                    },
                    ColumnDeclaration {
                        name: "active".into(),
                        data_type: ValueType::Boolean,
                        is_nullable: true,
                        default: Some(DataType::Boolean(true)),
                    },
                ],
                constraints: None,
            })]),
        },
        SqlQuery {
            number: 1,
            name: "create table with constraints",
            stmt: "CREATE TABLE orders (id INT, user_id INT, PRIMARY KEY (id), CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id));",
            expected: Ok(vec![Statement::CreateStmt(Create {
                table: "orders".into(),
                columns: vec![
                    ColumnDeclaration {
                        name: "id".into(),
                        data_type: ValueType::Int,
                        is_nullable: true,
                        default: None,
                    },
                    ColumnDeclaration {
                        name: "user_id".into(),
                        data_type: ValueType::Int,
                        is_nullable: true,
                        default: None,
                    },
                ],
                constraints: Some(vec![
                    TableLevelConstraints::PrimaryKey {
                        column_name: "id".into(),
                        constraint_name: None,
                    },
                    TableLevelConstraints::ForeignKey {
                        column_name: "user_id".into(),
                        references_table: "users".into(),
                        references_column: Some("id".into()),
                        constraint_name: Some("fk_user".into()),
                    },
                ]),
            })]),
        },
    ];
    assert_parser_expectation(tests);
}

#[test]
fn test_select_statements() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic SELECT",
            stmt: "SELECT * FROM users;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: None,
                projection: Projections::All,
                from: TableRef::Table {
                    name: "users".to_string(),
                    alias: None,
                },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 1,
            name: "bad SELECT",
            stmt: "SELEC * FROM;", // typo
            expected: Err(MokErrors::UnexpectedToken {
                expected: "select, insert, update, delete, create, drop, or transaction keyword".to_string(),
                found: "SELEC".to_string(),
                line: 1,
                column: 5,
            }),
        },
        SqlQuery {
            number: 2,
            name: "select with where clause",
            stmt: "SELECT * FROM users WHERE name = 'foo' AND username='bar' OR username='chippy';",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: None,
                projection: Projections::All,
                from: TableRef::Table {
                    name: "users".to_string(),
                    alias: None,
                },
                where_clause: Some(Condition::Logical(Logical {
                    lhs: Box::new(Condition::Logical(Logical {
                        lhs: Box::new(Condition::Comparison(Equality {
                            lhs: Expression::Identifier {
                                table: None,
                                name: "name".into(),
                            },
                            rhs: Expression::Literal{data_type:DataType::Text("foo".to_string()),alias:None},
                            operator: ComparisonOperator::Eq,
                        })),
                        rhs: Box::new(Condition::Comparison(Equality {
                            lhs: Expression::Identifier {
                                table: None,
                                name: "username".into(),
                            },
                            rhs: Expression::Literal{data_type:DataType::Text("bar".to_string()),alias:None},
                            operator: ComparisonOperator::Eq,
                        })),
                        operator: LogicalOperator::And,
                    })),
                    rhs: Box::new(Condition::Comparison(Equality {
                        lhs: Expression::Identifier {
                            table: None,
                            name: "username".into(),
                        },
                        rhs: Expression::Literal{data_type:DataType::Text("chippy".to_string()),alias:None},
                        operator: ComparisonOperator::Eq,
                    })),
                    operator: LogicalOperator::Or,
                })),
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 3,
            name: "select with where clause with explicit parentheses for ordering",
            stmt: "SELECT * FROM users WHERE name = 'foo' AND (username='bar' OR username='chippy');",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: None,
                projection: Projections::All,
                from: TableRef::Table {
                    name: "users".to_string(),
                    alias: None,
                },
                where_clause: Some(Condition::Logical(Logical {
                    lhs: Box::new(Condition::Comparison(Equality {
                        lhs: Expression::Identifier {
                            table: None,
                            name: "name".into(),
                        },
                        rhs: Expression::Literal{data_type:DataType::Text("foo".to_string()),alias:None},
                        operator: ComparisonOperator::Eq,
                    })),
                    rhs: Box::new(Condition::Logical(Logical {
                        lhs: Box::new(Condition::Comparison(Equality {
                            lhs: Expression::Identifier {
                                table: None,
                                name: "username".into(),
                            },
                            rhs: Expression::Literal{data_type:DataType::Text("bar".to_string()),alias:None},
                            operator: ComparisonOperator::Eq,
                        })),
                        rhs: Box::new(Condition::Comparison(Equality {
                            lhs: Expression::Identifier {
                                table: None,
                                name: "username".into(),
                            },
                            rhs: Expression::Literal{data_type:DataType::Text("chippy".to_string()),alias:None},
                            operator: ComparisonOperator::Eq,
                        })),
                        operator: LogicalOperator::Or,
                    })),
                    operator: LogicalOperator::And,
                })),
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 4,
            name: "select with distinct",
            stmt: "SELECT DISTINCT username, name FROM users;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: Some(Distinct::All),
                projection: Projections::Specified(vec![
                    ProjectionsItem {
                        expr: Expression::Identifier {
                            table: None,
                            name: "username".into(),
                        },
                        alias: None,
                    },
                    ProjectionsItem {
                        expr: Expression::Identifier {
                            table: None,
                            name: "name".into(),
                        },
                        alias: None,
                    },
                ]),
                from: TableRef::Table {
                    name: "users".to_string(),
                    alias: None,
                },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 5,
            name: "select with distinct on specific columns",
            stmt: "SELECT DISTINCT on (name, last_name) name, last_name, id FROM users;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: Some(Distinct::On(vec![
                    Expression::Identifier { table: None, name: "name".into() },
                    Expression::Identifier { table: None, name: "last_name".into() },
                ])),
                projection: Projections::Specified(vec![
                    ProjectionsItem {
                        expr: Expression::Identifier { table: None, name: "name".into() },
                        alias: None,
                    },
                    ProjectionsItem {
                        expr: Expression::Identifier { table: None, name: "last_name".into() },
                        alias: None,
                    },
                    ProjectionsItem {
                        expr: Expression::Identifier { table: None, name: "id".into() },
                        alias: None,
                    },
                ]),
                from: TableRef::Table { name: "users".to_string(), alias: None },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 6,
            name: "select using CTE and doing join",
            stmt: "WITH orders AS (
                SELECT id, item, user_id FROM orders
               )
              SELECT u.name, o.item
              FROM orders AS o
              JOIN users AS u ON o.user_id = u.id;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: Some(vec![Cte {
                    name: "orders".to_string(),
                    query: Box::new(Statement::SelectStmt(Select {
                        with: None,
                        distinct: None,
                        projection: Projections::Specified(vec![
                            ProjectionsItem { expr: Expression::Identifier { table: None, name: "id".to_string() }, alias: None },
                            ProjectionsItem { expr: Expression::Identifier { table: None, name: "item".to_string() }, alias: None },
                            ProjectionsItem { expr: Expression::Identifier { table: None, name: "user_id".to_string() }, alias: None },
                        ]),
                        from: TableRef::Table { name: "orders".to_string(), alias: None },
                        where_clause: None,
                        group_by: None,
                        order_by: None,
                        limit: None,
                        offset: None,
                    })),
                }]),
                distinct: None,
                projection: Projections::Specified(vec![
                    ProjectionsItem { expr: Expression::Identifier { table: Some("u".to_string()), name: "name".to_string() }, alias: None },
                    ProjectionsItem { expr: Expression::Identifier { table: Some("o".to_string()), name: "item".to_string() }, alias: None },
                ]),
                from: TableRef::Join {
                    left: Box::new(TableRef::Table { name: "orders".to_string(), alias: Some("o".to_string()) }),
                    right: Box::new(TableRef::Table { name: "users".to_string(), alias: Some("u".to_string()) }),
                    join_type: JoinType::Inner,
                    on: Condition::Comparison(Equality {
                        lhs: Expression::Identifier { table: Some("o".to_string()), name: "user_id".to_string() },
                        rhs: Expression::Identifier { table: Some("u".to_string()), name: "id".to_string() },
                        operator: ComparisonOperator::Eq,
                    }),
                },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 7,
            name: "select with multiple chained joins",
            stmt: "SELECT a.id, b.name, c.city FROM accounts AS a JOIN users AS b ON a.user_id = b.id JOIN locations as c ON b.location_id = c.id;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: None,
                projection: Projections::Specified(vec![
                    ProjectionsItem { expr: Expression::Identifier { table: Some("a".to_string()), name: "id".to_string() }, alias: None },
                    ProjectionsItem { expr: Expression::Identifier { table: Some("b".to_string()), name: "name".to_string() }, alias: None },
                    ProjectionsItem { expr: Expression::Identifier { table: Some("c".to_string()), name: "city".to_string() }, alias: None },
                ]),
                from: TableRef::Join {
                    left: Box::new(TableRef::Join {
                        left: Box::new(TableRef::Table { name: "accounts".to_string(), alias: Some("a".to_string()) }),
                        right: Box::new(TableRef::Table { name: "users".to_string(), alias: Some("b".to_string()) }),
                        join_type: JoinType::Inner,
                        on: Condition::Comparison(Equality {
                            lhs: Expression::Identifier { table: Some("a".to_string()), name: "user_id".to_string() },
                            rhs: Expression::Identifier { table: Some("b".to_string()), name: "id".to_string() },
                            operator: ComparisonOperator::Eq,
                        }),
                    }),
                    right: Box::new(TableRef::Table { name: "locations".to_string(), alias: Some("c".to_string()) }),
                    join_type: JoinType::Inner,
                    on: Condition::Comparison(Equality {
                        lhs: Expression::Identifier { table: Some("b".to_string()), name: "location_id".to_string() },
                        rhs: Expression::Identifier { table: Some("c".to_string()), name: "id".to_string() },
                        operator: ComparisonOperator::Eq,
                    }),
                },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery {
            number: 8,
            name: "select with multiple chained joins and aliased column names",
            stmt: "SELECT a.id as id, b.name as name, c.city as city FROM accounts AS a JOIN users AS b ON a.user_id = b.id JOIN locations as c ON b.location_id = c.id;",
            expected: Ok(vec![Statement::SelectStmt(Select {
                with: None,
                distinct: None,
                projection: Projections::Specified(vec![
                    ProjectionsItem { expr: Expression::Identifier { table: Some("a".to_string()), name: "id".to_string() }, alias: Some("id".into()) },
                    ProjectionsItem { expr: Expression::Identifier { table: Some("b".to_string()), name: "name".to_string() }, alias: Some("name".into()) },
                    ProjectionsItem { expr: Expression::Identifier { table: Some("c".to_string()), name: "city".to_string() }, alias: Some("city".into()) },
                ]),
                from: TableRef::Join {
                    left: Box::new(TableRef::Join {
                        left: Box::new(TableRef::Table { name: "accounts".to_string(), alias: Some("a".to_string()) }),
                        right: Box::new(TableRef::Table { name: "users".to_string(), alias: Some("b".to_string()) }),
                        join_type: JoinType::Inner,
                        on: Condition::Comparison(Equality {
                            lhs: Expression::Identifier { table: Some("a".to_string()), name: "user_id".to_string() },
                            rhs: Expression::Identifier { table: Some("b".to_string()), name: "id".to_string() },
                            operator: ComparisonOperator::Eq,
                        }),
                    }),
                    right: Box::new(TableRef::Table { name: "locations".to_string(), alias: Some("c".to_string()) }),
                    join_type: JoinType::Inner,
                    on: Condition::Comparison(Equality {
                        lhs: Expression::Identifier { table: Some("b".to_string()), name: "location_id".to_string() },
                        rhs: Expression::Identifier { table: Some("c".to_string()), name: "id".to_string() },
                        operator: ComparisonOperator::Eq,
                    }),
                },
                where_clause: None,
                group_by: None,
                order_by: None,
                limit: None,
                offset: None,
            })]),
        },
        SqlQuery{ 
            number: 9, 
            name: "JOIN without alias",
            stmt:"WITH orders AS (
                SELECT id, item, user_id FROM orders
               )
              SELECT name, item
              FROM orders AS o
              JOIN users AS u ON o.user_id = u.id;" ,
            expected: Ok(vec![
                Statement::SelectStmt(
                    Select {
                        with: Some(vec![
                            Cte {
                                name: "orders".to_string(),
                                query: Box::new(Statement::SelectStmt(
                                    Select {
                                        with: None,
                                        distinct: None,
                                        projection: Projections::Specified(vec![
                                            ProjectionsItem {
                                                expr: Expression::Identifier {
                                                    table: None,
                                                    name: "id".to_string(),
                                                },
                                                alias: None,
                                            },
                                            ProjectionsItem {
                                                expr: Expression::Identifier {
                                                    table: None,
                                                    name: "item".to_string(),
                                                },
                                                alias: None,
                                            },
                                            ProjectionsItem {
                                                expr: Expression::Identifier {
                                                    table: None,
                                                    name: "user_id".to_string(),
                                                },
                                                alias: None,
                                            },
                                        ]),
                                        from: TableRef::Table {
                                            name: "orders".to_string(),
                                            alias: None,
                                        },
                                        where_clause: None,
                                        group_by: None,
                                        order_by: None,
                                        limit: None,
                                        offset: None,
                                    }
                                )),
                            },
                        ]),
                        distinct: None,
                        projection: Projections::Specified(vec![
                            ProjectionsItem {
                                expr: Expression::Identifier {
                                    table: None,
                                    name: "name".to_string(),
                                },
                                alias: None,
                            },
                            ProjectionsItem {
                                expr: Expression::Identifier {
                                    table: None,
                                    name: "item".to_string(),
                                },
                                alias: None,
                            },
                        ]),
                        from: TableRef::Join {
                            left: Box::new(TableRef::Table {
                                name: "orders".to_string(),
                                alias: Some("o".to_string()),
                            }),
                            right: Box::new(TableRef::Table {
                                name: "users".to_string(),
                                alias: Some("u".to_string()),
                            }),
                            join_type: JoinType::Inner,
                            on: Condition::Comparison(
                                Equality {
                                    lhs: Expression::Identifier {
                                        table: Some("o".to_string()),
                                        name: "user_id".to_string(),
                                    },
                                    rhs: Expression::Identifier {
                                        table: Some("u".to_string()),
                                        name: "id".to_string(),
                                    },
                                    operator: ComparisonOperator::Eq,
                                }
                            ),
                        },
                        where_clause: None,
                        group_by: None,
                        order_by: None,
                        limit: None,
                        offset: None,
                    }
                )
            ]),
 },
    ];
    assert_parser_expectation(tests);
}

#[test]
fn test_transaction_block() {
    let tests = vec![
        SqlQuery {
            number: 0,
            name: "basic transaction begin",
            stmt: "BEGIN;",
            expected: Ok(vec![Statement::ExplicitTransaction]),
        },
        SqlQuery {
            number: 1,
            name: "basic transaction commit",
            stmt: "COMMIT;",
            expected: Ok(vec![Statement::ExplicitTransactionCommit]),
        },
        SqlQuery {
            number: 2,
            name: "transaction with statements",
            stmt: "BEGIN; INSERT INTO users (id, name) VALUES (1, 'foo'); SELECT id FROM users WHERE id = 1; COMMIT;",
            expected: Ok(vec![
                Statement::ExplicitTransaction,
                Statement::InsertStmt(Insert {
                    table: "users".into(),
                    columns: vec!["id".into(), "name".into()],
                    sub_select: None,
                    values: Some(vec![
                        Expression::Literal{data_type: DataType::Int(1), alias: None},
                        Expression::Literal{data_type:DataType::Text("foo".into()), alias: None},
                    ]),
                    returning: None,
                }),
                Statement::SelectStmt(Select { 
                    with: None,
                    distinct: None,
                    projection: Projections::Specified(
                        vec![                        ProjectionsItem {
                                expr: Expression::Identifier {
                                    table: None,
                                    name: "id".into(),
                                },
                                alias: None,
                            },
                        ],
                    ),
                    from: TableRef::Table {
                        name: "users".into(),
                        alias: None,
                    },
                    where_clause: Some(
                        Condition::Comparison(
                            Equality {
                                lhs: Expression::Identifier {
                                    table: None,
                                    name: "id".into(),
                                },
                                rhs: Expression::Literal{data_type:DataType::Int(1), alias: None },
                                operator: ComparisonOperator::Eq,
                            },
                        ),
                    ),
                    group_by: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                },
            ),
                Statement::ExplicitTransactionCommit,
            ]),
        },
        SqlQuery {
            number: 3,
            name: "invalid transaction syntax",
            stmt: "BEGIN COMMIT;",
            expected: Err(MokErrors::UnexpectedToken {
                expected: ";".to_string(),
                found: "commit".to_string(),
                line: 1,
                column: 12,
            }),
        },
    ];

    assert_parser_expectation(tests);
}

fn assert_parser_expectation(test_inputs: Vec<SqlQuery>) {
    for test in test_inputs {
        let result = Parser::new(Lexer::new(test.stmt.to_string())).parse_script();

        assert_eq!(
            result, test.expected,
            "\nTest number #{}\nDescription of case: ('{}') failed:\nInput: {}\n",
            test.number, test.name, test.stmt
        );
    }
}
