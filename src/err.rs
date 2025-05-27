use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseErrors {
    #[error("table: {0} does not exits")]
    TableNotFound(String),
    #[error("column: {column} does not exists in table: {table}")]
    ColumnNotFoundInTable { column: String, table: String },
}
