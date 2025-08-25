use mokdb::engine::ir::{Column, ProjectionsItem, QueryIR};
use mokdb::engine::semantic_analysis::{Scope, SemanticAnalyzer};
use mokdb::errors::MokErrors;
use mokdb::parser::lexer::Lexer;
use mokdb::parser::parser::Parser;
use mokdb::storage::catalog_manager::{CatalogManagerImpl, Table};

#[derive(Debug)]
struct SqlQuery<'a> {
    number: u64,
    name: &'a str,
    stmt: &'a str,
    tables: Vec<Table>,
    expected: Result<QueryIR, MokErrors>,
}

#[test]
fn select_stmt_parsing_and_semantic_validation() {
    let test_cases = vec![SqlQuery {
        number: 1,
        name: "simple select statement",
        stmt: "SELECT * FROM users",
        tables: vec![Table::new(
            "users".into(),
            vec![
                Column {
                    table: Some("users".into()),
                    cte: None,
                    name: "id".into(),
                    data_type: mokdb::engine::ir::ValueType::Int,
                    value: None,
                    constraints: None,
                },
                Column {
                    table: Some("users".into()),
                    cte: None,
                    name: "username".into(),
                    data_type: mokdb::engine::ir::ValueType::Text,
                    value: None,
                    constraints: Some(mokdb::engine::ir::TableLevelConstraints::Unique {
                        column_name: "username".into(),
                        constraint_name: Some("indx_unique_users".into()),
                    }),
                },
                Column {
                    table: Some("users".into()),
                    cte: None,
                    name: "name".into(),
                    data_type: mokdb::engine::ir::ValueType::Text,
                    value: None,
                    constraints: None,
                },
            ],
            None,
        )],
        expected: Ok(QueryIR::SelectStmt(mokdb::engine::ir::Select {
            with: None,
            distinct: None,
            projection: vec![
                ProjectionsItem {
                    expr: mokdb::engine::ir::Expression::Identifier {
                        table: "users".into(),
                        name: "id".into(),
                        constraints: None,
                    },
                    alias: None,
                    data_type: mokdb::engine::ir::ValueType::Int,
                },
                ProjectionsItem {
                    expr: mokdb::engine::ir::Expression::Identifier {
                        table: "users".into(),
                        name: "username".into(),
                        constraints: Some(mokdb::engine::ir::TableLevelConstraints::Unique {
                            column_name: "username".into(),
                            constraint_name: Some("indx_unique_users".into()),
                        }),
                    },
                    alias: None,
                    data_type: mokdb::engine::ir::ValueType::Text,
                },
                ProjectionsItem {
                    expr: mokdb::engine::ir::Expression::Identifier {
                        table: "users".into(),
                        name: "name".into(),
                        constraints: None,
                    },
                    alias: None,
                    data_type: mokdb::engine::ir::ValueType::Text,
                },
            ],
            from: mokdb::engine::ir::TableRef::Table {
                name: "users".into(),
                alias: None,
            },
            where_clause: None,
            group_by: None,
            order_by: None,
            limit: None,
            offset: None,
        })),
    }];

    for test in test_cases {
        let mut parser = Parser::new(Lexer::new(test.stmt.to_string()));
        let statements = parser.parse_statement(false, false);
        let analyzer = SemanticAnalyzer::new(CatalogManagerImpl::new(test.tables));
        assert!(statements.is_ok());
        let result = analyzer.analyze(Scope::default(), statements.unwrap());

        assert_eq!(
            result, test.expected,
            "\nTest number #{}\nDescription of case: ('{}') failed:\nInput: {}\n",
            test.number, test.name, test.stmt
        );
    }
}
