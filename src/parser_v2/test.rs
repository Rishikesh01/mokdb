#[cfg(test)]
mod parser_tests {
    use crate::parser_v2::{
        ast::SQLStatement,
        parser::Parser,
        scanner::{SQLInput, Scanner},
    };

    struct SelectTestCase<'a> {
        name: &'a str,
        sql: &'a str,
        expected_columns: usize,
        expected_table: &'a str,
        expect_where: bool,
    }

    #[test]
    fn test_select_statements_table_driven() {
        let test_cases = vec![
            SelectTestCase {
                name: "1. Basic select with WHERE and NOT",
                sql: "SELECT name, age FROM users WHERE NOT(((foo = 'bar' AND fuzz = 'fuzz0') OR (foo = 'baz' AND fuz = 'dazz')) AND (IS_ACTIVE = FALSE AND IS_ENABLED = TRUE))",
                expected_columns: 2,
                expected_table: "users",
                expect_where: true,
            },
            SelectTestCase {
                name: "2. Simple AND",
                sql: "SELECT * FROM users WHERE age > 30 AND status = 'active'",
                expected_columns: 1, // * counts as one
                expected_table: "users",
                expect_where: true,
            },
            SelectTestCase {
                name: "3. Nested OR and AND",
                sql: "SELECT id FROM orders WHERE (total > 100 OR discount > 0.1) AND status = 'confirmed'",
                expected_columns: 1,
                expected_table: "orders",
                expect_where: true,
            },
            SelectTestCase {
                name: "3. No WHERE clause",
                sql: "SELECT id, name FROM customers",
                expected_columns: 2,
                expected_table: "customers",
                expect_where: false,
            },
            SelectTestCase {
                name: "4. Precedence check (OR then AND)",
                sql: "SELECT * FROM data WHERE a = 1 OR b = 2 AND c = 3",
                expected_columns: 1,
                expected_table: "data",
                expect_where: true,
            },
        ];

        for case in test_cases {
            println!("Running test case: {}", case.name);
            let tokens = Scanner::new(case.sql.to_string()).tokenize();
            assert!(
                tokens.is_ok(),
                "Tokenizer failed for case: {} with error: {}",
                case.name,
                tokens.unwrap_err()
            );
            let mut parser = Parser::new(tokens.unwrap());
            let result = parser.parse_and_build_ast();
            match result {
                Ok(SQLStatement::Select(select_stmt)) => {
                    assert_eq!(
                        select_stmt.columns.len(),
                        case.expected_columns,
                        "Column count mismatch in case: {}",
                        case.name
                    );
                    assert_eq!(
                        select_stmt.from.to_lowercase(),
                        case.expected_table,
                        "Table name mismatch in case: {}",
                        case.name
                    );
                    assert_eq!(
                        select_stmt.where_clause.is_some(),
                        case.expect_where,
                        "WHERE clause presence mismatch in case: {}",
                        case.name
                    );
                    if let Some(where_clause) = select_stmt.where_clause {
                        println!(
                            "WHERE condition ({}): {:?}",
                            case.name, where_clause.condition
                        );
                    }
                }
                _ => panic!(
                    "Expected Select statement in case: {:?}, got: {:?}",
                    case.name, result
                ),
            }
        }
    }
}
