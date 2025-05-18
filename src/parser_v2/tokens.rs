#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Types {
    //DDL – Data Definition Language
    Create,
    Drop,
    Alter,
    Rename,
    Table,
    Schema,

    //DQL – Data Query Language
    Select,
    Distinct,
    From,
    Where,
    OrderBy,
    LeftJoin,
    RightJoin,
    FullOuterJoin,
    On,
    //DML – Data Manipulation Language
    Insert,
    Values,
    Into,
    Update,
    Set,
    Delete,
    Lock,

    //TCL – Transaction Control Language
    BeginTransaction,
    RollBack,
    Commit,

    //Arithmetic Operators
    Addition,
    Subtraction,
    Division,
    Modulus,

    // Comparison Operators
    EqualTo,
    GreaterThan,
    LessThan,
    GreaterThanOrEqualTo,
    LessThanOrEqualTo,
    NotEqualTo,

    // Logical Operators
    And,
    Or,
    Not,

    // Special operators
    In,

    // Data types
    Integer,
    Text,
    Decimal,
    Boolean,

    // Table related constraints
    PrimaryKey,
    UniqueKey,
    ForeginKey,

    // Other keywords
    Null,
    Constraint,
    Add,
    Truncate,
    Is,

    // prasing related keywords
    Identifier,
    Literal,
    OpenParen,
    CloseParen,
    Comma,
    Semicolon,
    Eof,
    AllColumnsOrMultiplication,
    Invalid,

    // Ordering
    AscendingOrder,
    DecendingOrder,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: Types,
    pub lexeme: String,
    pub literal: Option<ParsedLiteral>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum ParsedLiteral {
    Text(String),
    Number(i64),
    Decimal(f64),
}

impl Token {
    pub fn new(
        token_type: Types,
        lexeme: String,
        literal: Option<ParsedLiteral>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            token_type,
            lexeme,
            literal,
            line,
            column,
        }
    }
}
