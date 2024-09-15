use std::any::Any;

use crate::parsor::sql_token_types::SQLTokenTypes;
#[derive(Debug)]
pub struct Token {
    pub token_type: SQLTokenTypes,
    pub lexeme: String,
    pub literal: dyn Any,
}
