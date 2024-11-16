#[derive(Debug, PartialEq)]
pub enum SQLTokenTypes {
    //Data Manipulation language
    Select,
    Insert,
    Delete,
    Update,
    //Data Definiation language
    Create,
    Drop,
    Truncate,
    Rename,
    Alter,
    //TCL
    Commit,
    Rollback,
    Savepoint,
    TableIdentifier,
    Identifier,
    Number,
    Eof,

    //other
    Leftparen,
    Rightparen,
    Star,
    Comma,
    Semicolon,
    Newline,

    //logical
    Greater,
    Lesser,
    Equal,

    Primary,
    Key,
    Not,
    Unique,
    Null,

    Into,
    Values,

    Set,
    Where,
    From,

    And,
    OR,
    NotEqual,
    String,
    Table,

    IS,
    GreaterThanOrEqualTo,
    LesserThanOrEqualTo,
}

impl Clone for SQLTokenTypes {
    fn clone(&self) -> Self {
        match self {
            Self::Select => Self::Select,
            Self::Insert => Self::Insert,
            Self::Delete => Self::Delete,
            Self::Update => Self::Update,
            Self::Create => Self::Create,
            Self::Drop => Self::Drop,
            Self::Truncate => Self::Truncate,
            Self::Rename => Self::Rename,
            Self::Alter => Self::Alter,
            Self::Commit => Self::Commit,
            Self::Rollback => Self::Rollback,
            Self::Savepoint => Self::Savepoint,
            Self::TableIdentifier => Self::TableIdentifier,
            Self::Identifier => Self::Identifier,
            Self::Number => Self::Number,
            Self::Eof => Self::Eof,
            Self::Leftparen => Self::Leftparen,
            Self::Rightparen => Self::Rightparen,
            Self::Star => Self::Star,
            Self::Comma => Self::Comma,
            Self::Semicolon => Self::Semicolon,
            Self::Newline => Self::Newline,
            Self::Greater => Self::Greater,
            Self::Lesser => Self::Lesser,
            Self::Equal => Self::Equal,
            Self::Primary => Self::Primary,
            Self::Key => Self::Key,
            Self::Not => Self::Not,
            Self::Unique => Self::Unique,
            Self::Null => Self::Null,
            Self::Into => Self::Into,
            Self::Values => Self::Values,
            Self::Set => Self::Set,
            Self::Where => Self::Where,
            Self::From => Self::From,
            Self::And => Self::And,
            Self::OR => Self::OR,
            Self::NotEqual => Self::NotEqual,
            Self::String => Self::String,
            Self::Table => Self::Table,
            Self::IS => Self::IS,
            Self::GreaterThanOrEqualTo => Self::GreaterThanOrEqualTo,
            Self::LesserThanOrEqualTo => Self::LesserThanOrEqualTo,
        }
    }
}
