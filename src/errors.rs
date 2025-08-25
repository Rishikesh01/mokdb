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
    #[error("table name max limit crossed")]
    TableNameMaxLimitCrossed,
    #[error("missing join condition at {line}, column: {column}")]
    MissingJoinCondition { line: usize, column: usize },

    #[error("cte {name} contains invalid statement {statement}")]
    CteContainsInvalidStatementType { name: String, statement: String },

    #[error("should not reach here in semantic analyzer")]
    ShouldNotReachHere,

    #[error("unknown data source {table_name}")]
    UnknownDataSource { table_name: String },

    #[error("column {column_name} was not found in table {table_name}")]
    UnknownColumnProvided {
        column_name: String,
        table_name: String,
    },

    #[error("binary op: {op_name}, not valid for {operand_side} ")]
    IncompatibleOperandsForBinaryOp {
        op_name: String,
        operand_side: String,
    },

    #[error("comparsion op: {op_name}, not valid for {op_side} ")]
    IncompatibleOperandsForComparsionOp { op_name: String, op_side: String },

    #[error(
        "expression in rhs side of 'in' is not valid, lhs: {lhs_data_type} rhs:{rhs_data_type}"
    )]
    IncompatibleRhsDatatypeInClause {
        lhs_data_type: String,
        rhs_data_type: String,
    },

    #[error("subquery can only be select statement and not {stmt_type}")]
    InvalidSubQuery { stmt_type: String },

    #[error("distinct needs to have actual column names and they must not be ambigious")]
    InvalidDistinctOnClause,

    #[error("higher level lock already exists")]
    HigherLevelLockExists,
}
