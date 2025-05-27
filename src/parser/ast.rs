#[derive(Debug, PartialEq)]
pub enum Statement {
    SelectStmt(Select),
    InsertStmt(Insert),
    UpdateStmt(Update),
    DeleteSmt(Delete),
    DropStmt(Drop),
    CreateStmt(Create),
    ExplicitTransaction,
    ExplicitTransactionCommit,
}

#[derive(Debug, PartialEq)]
pub struct Cte {
    pub name: String,
    pub query: Box<Statement>,
}

#[derive(Debug, PartialEq)]
pub enum Distinct {
    All,
    On(Vec<Expression>),
}

#[derive(Debug, PartialEq)]
pub struct Select {
    pub with: Option<Vec<Cte>>,
    pub distinct: Option<Distinct>,
    pub projection: Projections,
    pub from: TableRef,
    pub where_clause: Option<Condition>,
    pub group_by: Option<Vec<Expression>>,
    pub order_by: Option<Vec<OrderBy>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct ProjectionsItem {
    pub expr: Expression,
    pub alias: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Projections {
    All,
    Specified(Vec<ProjectionsItem>),
}

#[derive(Debug, PartialEq)]
pub enum LikeKind {
    Like,
    NotLike,
    ILike,
    NotILike,
}

#[derive(Debug, PartialEq)]
pub enum Condition {
    Comparison(Equality),
    Logical(Logical),
    In {
        expr: Expression,
        set: InList,
    },
    Exists(Box<Select>),
    Not(Box<Condition>),
    IsNull(Expression),
    IsNotNull(Expression),
    Like {
        expr: Expression,
        pattern: Expression,
        kind: LikeKind,
    },
}

#[derive(Debug, PartialEq)]
pub enum InList {
    Expressions(Vec<Expression>),
    SubQuery(Box<Select>),
}

#[derive(Debug, PartialEq)]
pub struct Logical {
    pub lhs: Box<Condition>,
    pub rhs: Box<Condition>,
    pub operator: LogicalOperator,
}

#[derive(Debug, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, PartialEq)]
pub struct Equality {
    pub lhs: Expression,
    pub rhs: Expression,
    pub operator: ComparisonOperator,
}

#[derive(Debug, PartialEq)]
pub enum ComparisonOperator {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    Neq,
}

#[derive(Debug, PartialEq)]
pub enum JoinType {
    Left,
    Right,
    Inner,
    FullOuter,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Identifier { table: Option<String>, name: String },
    Literal(DataType),
    BinaryOp(Box<BinaryOp>),
}

#[derive(Debug, PartialEq)]
pub enum ValueType {
    Text,
    Int,
    Float,
    Boolean,
}

#[derive(Debug, PartialEq)]
pub enum DataType {
    Text(String),
    Int(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, PartialEq)]
pub struct BinaryOp {
    pub lhs: Expression,
    pub rhs: Expression,
    pub operator: BinaryOperator,
}

#[derive(Debug, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Multi,
    Div,
    Modulo,
}

#[derive(Debug, PartialEq)]
pub enum TableRef {
    Table {
        name: String,
        alias: Option<String>,
    },
    SubQuery {
        query: Box<Select>,
        alias: String,
    },

    Join {
        left: Box<TableRef>,
        right: Box<TableRef>,
        join_type: JoinType,
        on: Condition,
    },
}

#[derive(Debug, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, PartialEq)]
pub struct OrderBy {
    pub expr: Expression,
    pub order: OrderDirection,
}

#[derive(Debug, PartialEq)]
pub struct Insert {
    pub table: String,
    pub columns: Vec<String>,
    pub sub_select: Option<Select>,
    pub values: Option<Vec<Expression>>,
    pub returning: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq)]
pub struct Update {
    pub table: String,
    pub assignments: Vec<(String, Expression)>,
    pub where_clause: Option<Condition>,
    pub returning: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq)]
pub struct Delete {
    pub table: TableRef,
    pub where_clause: Option<Condition>,
    pub returning: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq)]
pub struct Create {
    pub table: String,
    pub columns: Vec<ColumnDeclaration>,
    pub constraints: Option<Vec<TableLevelConstraints>>,
}

#[derive(Debug, PartialEq)]
pub struct ColumnDeclaration {
    pub name: String,
    pub data_type: ValueType,
    pub is_nullable: bool,
    pub default: Option<DataType>,
}

#[derive(Debug, PartialEq)]
pub enum TableLevelConstraints {
    PrimaryKey {
        column_name: String,
        constraint_name: Option<String>,
    },
    ForeignKey {
        column_name: String,
        references_table: String,
        references_column: Option<String>,
        constraint_name: Option<String>,
    },
    Unique {
        column_name: String,
        constraint_name: Option<String>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Drop {
    pub table: String,
}
