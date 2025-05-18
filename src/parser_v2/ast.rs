use super::tokens::Types;

#[derive(Debug)]
pub enum SQLStatement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Create(CreateStatement),
    Drop(DropStatement),
}

#[derive(Debug)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Expression>,
}

#[derive(Debug)]
pub struct UpdateStatement {
    pub table: String,
    pub assignments: Vec<Assignment>,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug)]
pub struct Assignment {
    pub column: String,
    pub value: Expression,
}

#[derive(Debug)]
pub struct DeleteStatement {
    pub table: String,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug)]
pub struct CreateStatement {
    pub table: String,
    pub columns: Vec<ColumnDefinition>,
}

#[derive(Debug)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub constraints: Vec<ColumnConstraint>,
}

#[derive(Debug)]
pub enum DataType {
    Integer,
    Decimal,
    Text,
    Boolean,
}

#[derive(Debug)]
pub enum ColumnConstraint {
    PrimaryKey,
    NotNull,
    UniqueKey,
}

#[derive(Debug)]
pub struct DropStatement {
    pub table: String,
}

#[derive(Debug)]
pub struct SelectStatement {
    pub columns: Vec<SelectColumn>,
    pub from: String,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<Vec<OrderByClause>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug)]
pub struct OrderByClause {
    pub column_name: String,
    pub is_asec: bool,
}

#[derive(Debug)]
pub enum SelectColumn {
    All,
    Column(String),
}

#[derive(Debug)]
pub struct WhereClause {
    pub condition: Condition,
}

#[derive(Debug)]
pub enum Condition {
    Comparison(ComparisonCondition),
    Logical(LogicalCondition),
    Not(Box<Condition>),
    NullCheck(NullCheckCondition),
    In(InCondition),
}

#[derive(Debug)]
pub struct InCondition {
    pub left: Expression,
    pub values: InValues,
}

#[derive(Debug)]
pub enum InValues {
    List(Option<Vec<Expression>>),
    Subquery(Option<Box<SelectStatement>>),
}

#[derive(Debug)]
pub struct ComparisonCondition {
    pub operator: ComparisonOperator,
    pub left: Expression,
    pub right: Expression,
}

#[derive(Debug)]
pub enum NullCheckCondition {
    IsNull { identifier: String },
    IsNotNull { identifier: String },
}

#[derive(Debug)]
pub enum ComparisonOperator {
    EqualTo,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug)]
pub struct LogicalCondition {
    pub left: Box<Condition>,
    pub operator: LogicalOperator,
    pub right: Box<Condition>,
}

#[derive(Debug)]
pub enum LogicalOperator {
    And,
    Or,
}
impl LogicalOperator {
    pub fn match_sql_token_to_operator(sql_token: Types) -> Result<LogicalOperator, String> {
        match sql_token {
            Types::And => Ok(LogicalOperator::And),
            Types::Or => Ok(LogicalOperator::Or),
            _ => Err("unexpected token".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum Expression {
    Identifier(String),
    Literal(Literal),
}

#[derive(Debug)]
pub enum Literal {
    String(String),
    Number(i64),
    Decimal(f64),
    Boolean(bool),
}
