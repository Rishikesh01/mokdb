#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    // Keywords
    Select,
    As,
    Insert,
    Update,
    Delete,
    Drop,
    With,
    Create,
    Table,
    From,
    Where,
    By,
    Group,
    Order,
    Returning,
    True,
    False,
    Values,
    Limit,
    Offset,
    Distinct,
    All,
    On,
    And,
    Or,
    Not,
    In,
    Exists,
    Is,
    Null,
    Like,
    ILike,
    Key,
    Primary,
    Foreign,
    Unique,
    //joins
    Join,
    Left,
    Right,
    Inner,
    Full,
    Set,

    // Operators
    Eq,         // =
    Gt,         // >
    Lt,         // <
    Gte,        // >=
    Lte,        // <=
    Neq,        // !=
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Comma,      // ,
    Dot,        // .
    Semicolon,  // ;
    OpenParen,  // (
    CloseParen, // )
    // Identifiers and literals (token types only)
    Identifier,
    Literal,

    Asc,
    Desc,

    Into,

    Text,
    Int,
    Float,
    Boolean,
    Default,
    Constraint,
    References,
    Begin,
    Commit,
    Rollback,
}

impl Syntax {
    pub fn to_string(self) -> String {
        match self {
            Syntax::Select => "SELECT".into(),
            Syntax::As => "AS".into(),
            Syntax::Insert => "INSERT".into(),
            Syntax::Update => "UPDATE".into(),
            Syntax::Delete => "DELETE".into(),
            Syntax::Drop => "DROP".into(),
            Syntax::With => "WITH".into(),
            Syntax::Create => "CREATE".into(),
            Syntax::From => "FROM".into(),
            Syntax::Where => "WHERE".into(),
            Syntax::By => "BY".into(),
            Syntax::Group => "GROUP".into(),
            Syntax::Order => "ORDER".into(),
            Syntax::Returning => "RETURNING".into(),
            Syntax::True => "TRUE".into(),
            Syntax::False => "FALSE".into(),
            Syntax::Values => "VALUES".into(),
            Syntax::Limit => "LIMIT".into(),
            Syntax::Offset => "OFFSET".into(),
            Syntax::Distinct => "DISTINCT".into(),
            Syntax::All => "ALL".into(),
            Syntax::On => "ON".into(),
            Syntax::And => "AND".into(),
            Syntax::Or => "OR".into(),
            Syntax::Not => "NOT".into(),
            Syntax::In => "IN".into(),
            Syntax::Exists => "EXISTS".into(),
            Syntax::Is => "IS".into(),
            Syntax::Null => "NULL".into(),
            Syntax::Like => "LIKE".into(),
            Syntax::ILike => "ILIKE".into(),
            Syntax::Key => "KEY".into(),
            Syntax::Primary => "PRIMARY".into(),
            Syntax::Foreign => "FOREIGN".into(),
            Syntax::Unique => "UNIQUE".into(),
            Syntax::Join => "JOIN".into(),
            Syntax::Left => "LEFT".into(),
            Syntax::Right => "RIGHT".into(),
            Syntax::Inner => "INNER".into(),
            Syntax::Full => "FULL".into(),
            Syntax::Eq => "=".into(),
            Syntax::Gt => ">".into(),
            Syntax::Lt => "<".into(),
            Syntax::Gte => ">=".into(),
            Syntax::Lte => "<=".into(),
            Syntax::Neq => "!=".into(),
            Syntax::Plus => "+".into(),
            Syntax::Minus => "-".into(),
            Syntax::Star => "*".into(),
            Syntax::Slash => "/".into(),
            Syntax::Percent => "%".into(),
            Syntax::Comma => ",".into(),
            Syntax::Dot => ".".into(),
            Syntax::Semicolon => ";".into(),
            Syntax::OpenParen => "(".into(),
            Syntax::CloseParen => ")".into(),
            Syntax::Identifier => "identifier".into(),
            Syntax::Literal => "literal".into(),
            Syntax::Asc => "asc".into(),
            Syntax::Desc => "desc".into(),
            Syntax::Into => "into".into(),
            Syntax::Set => "set".into(),
            Syntax::Text => "text".into(),
            Syntax::Int => "int".into(),
            Syntax::Float => "float".into(),
            Syntax::Boolean => "boolean".into(),
            Syntax::Default => "default".into(),
            Syntax::Constraint => "constraint".into(),
            Syntax::References => "references".into(),
            Syntax::Table => "table".into(),
            Syntax::Begin => "begin".into(),
            Syntax::Commit => "commit".into(),
            Syntax::Rollback => "rollback".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Text(String),
    Int(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Syntax,
    pub lexeme: String,
    pub data_type: Option<LiteralValue>,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(
        kind: Syntax,
        lexeme: String,
        data_type: Option<LiteralValue>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            kind,
            lexeme,
            data_type,
            line,
            column,
        }
    }
}
