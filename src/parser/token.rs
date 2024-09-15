use super::sql_token_types::SQLTokenTypes;
use std::any::Any;

#[derive(Debug)]
pub struct Token {
    pub token_type: SQLTokenTypes,
    pub lexeme: String,
    pub literal: Option<Box<dyn Any>>,
}
