use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MokErrors {
    #[error(
        "Unexpected token at line {line}, column {column}: expected {expected:?}, found {found:?}"
    )]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },
    #[error("data type is not supported at {line}, column:{column}, value:{value}")]
    InvalidInputType {
        line: usize,
        column: usize,
        value: String,
    },
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("{0}")]
    Custom(String),
    #[error("missing join condition at {line}, column: {column}")]
    MissingJoinCondition { line: usize, column: usize },
}
