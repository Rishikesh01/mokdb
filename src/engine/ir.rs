use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum QueryIR {
    SelectStmt(Select),
    InsertStmt(Insert),
    UpdateStmt(Update),
    DeleteSmt(Delete),
    DropStmt(Drop),
    CreateStmt(Create),
}

#[derive(Debug, PartialEq)]
pub struct Cte {
    pub name: String,
    pub query: Box<QueryIR>,
}

#[derive(Debug, PartialEq)]
pub struct Select {
    pub with: Option<Vec<Cte>>,
    pub distinct: Option<Vec<Column>>,
    pub projection: Vec<ProjectionsItem>,
    pub from: TableRef,
    pub where_clause: Option<Condition>,
    pub group_by: Option<Vec<(Expression, ValueType)>>,
    pub order_by: Option<Vec<OrderBy>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct ProjectionsItem {
    pub expr: Expression,
    pub data_type: ValueType,
    pub alias: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum LikeKind {
    Like,
    NotLike,
    ILike,
    NotILike,
}

impl LikeKind {
    pub fn map_ast(like_kind: crate::parser::ast::LikeKind) -> LikeKind {
        match like_kind {
            crate::parser::ast::LikeKind::Like => LikeKind::Like,
            crate::parser::ast::LikeKind::NotLike => LikeKind::NotLike,
            crate::parser::ast::LikeKind::ILike => LikeKind::ILike,
            crate::parser::ast::LikeKind::NotILike => LikeKind::NotILike,
        }
    }
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
    IsNull((Expression, ValueType)),
    IsNotNull((Expression, ValueType)),
    Like {
        expr: Expression,
        pattern: Expression,
        kind: LikeKind,
    },
}

#[derive(Debug, PartialEq)]
pub enum InList {
    Expressions(Vec<(Expression, ValueType)>),
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

impl LogicalOperator {
    pub fn map_ast(op: crate::parser::ast::LogicalOperator) -> LogicalOperator {
        match op {
            crate::parser::ast::LogicalOperator::And => LogicalOperator::And,
            crate::parser::ast::LogicalOperator::Or => LogicalOperator::Or,
        }
    }
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

impl ComparisonOperator {
    pub fn map_asst(op: crate::parser::ast::ComparisonOperator) -> ComparisonOperator {
        match op {
            crate::parser::ast::ComparisonOperator::Eq => ComparisonOperator::Eq,
            crate::parser::ast::ComparisonOperator::Gt => ComparisonOperator::Gt,
            crate::parser::ast::ComparisonOperator::Lt => ComparisonOperator::Lt,
            crate::parser::ast::ComparisonOperator::Gte => ComparisonOperator::Gte,
            crate::parser::ast::ComparisonOperator::Lte => ComparisonOperator::Lte,
            crate::parser::ast::ComparisonOperator::Neq => ComparisonOperator::Neq,
        }
    }

    pub fn is_valid_operand(&self, lhs: &ValueType, rhs: &ValueType) -> (bool, bool) {
        match self {
            ComparisonOperator::Eq => todo!(),
            ComparisonOperator::Gt => todo!(),
            ComparisonOperator::Lt => todo!(),
            ComparisonOperator::Gte => todo!(),
            ComparisonOperator::Lte => todo!(),
            ComparisonOperator::Neq => todo!(),
        }
    }
}

impl Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOperator::Eq => write!(f, "="),
            ComparisonOperator::Gt => write!(f, ">"),
            ComparisonOperator::Lt => write!(f, "<"),
            ComparisonOperator::Gte => write!(f, ">="),
            ComparisonOperator::Lte => write!(f, "<="),
            ComparisonOperator::Neq => write!(f, "!="),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum JoinType {
    Left,
    Right,
    Inner,
    FullOuter,
}

impl JoinType {
    pub fn map_ast(join_type: crate::parser::ast::JoinType) -> JoinType {
        match join_type {
            crate::parser::ast::JoinType::Left => JoinType::Left,
            crate::parser::ast::JoinType::Right => JoinType::Right,
            crate::parser::ast::JoinType::Inner => JoinType::Inner,
            crate::parser::ast::JoinType::FullOuter => JoinType::FullOuter,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Column {
    pub table: Option<String>,
    pub cte: Option<String>,
    pub name: String,
    pub data_type: ValueType,
    pub value: Option<DataType>,
    pub constraints: Option<TableLevelConstraints>,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Identifier {
        table: String,
        name: String,
        constraints: Option<TableLevelConstraints>,
    },
    Literal {
        alias: Option<String>,
        data_type: DataType,
    },
    BinaryOp(Box<BinaryOp>),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ValueType {
    Text,
    Int,
    Float,
    Boolean,
}

#[derive(Debug, PartialEq)]
pub struct BinaryOp {
    pub lhs: (Expression, ValueType),
    pub rhs: (Expression, ValueType),
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

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Add => write!(f, "+"),
            BinaryOperator::Sub => write!(f, "-"),
            BinaryOperator::Multi => write!(f, "*"),
            BinaryOperator::Div => write!(f, "/"),
            BinaryOperator::Modulo => write!(f, "%"),
        }
    }
}

impl BinaryOperator {
    pub fn map_ast(op: crate::parser::ast::BinaryOperator) -> BinaryOperator {
        match op {
            crate::parser::ast::BinaryOperator::Add => BinaryOperator::Add,
            crate::parser::ast::BinaryOperator::Sub => BinaryOperator::Sub,
            crate::parser::ast::BinaryOperator::Multi => BinaryOperator::Multi,
            crate::parser::ast::BinaryOperator::Div => BinaryOperator::Div,
            crate::parser::ast::BinaryOperator::Modulo => BinaryOperator::Modulo,
        }
    }

    pub fn is_valid_operand(&self, lhs: &ValueType, rhs: &ValueType) -> (bool, bool) {
        match self {
            BinaryOperator::Add => {
                let is_valid_lhs =
                    matches!(lhs, ValueType::Text | ValueType::Int | ValueType::Float);
                let is_valid_rhs =
                    matches!(rhs, ValueType::Text | ValueType::Int | ValueType::Float);
                (is_valid_lhs, is_valid_rhs)
            }
            BinaryOperator::Sub
            | BinaryOperator::Multi
            | BinaryOperator::Div
            | BinaryOperator::Modulo => {
                let is_valid_lhs = matches!(lhs, ValueType::Int | ValueType::Float);
                let is_valid_rhs = matches!(rhs, ValueType::Int | ValueType::Float);
                (is_valid_lhs, is_valid_rhs)
            }
        }
    }
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
        on: Box<Condition>,
    },
}

#[derive(Debug, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    pub fn map_ast(od: crate::parser::ast::OrderDirection) -> OrderDirection {
        match od {
            crate::parser::ast::OrderDirection::Asc => OrderDirection::Asc,
            crate::parser::ast::OrderDirection::Desc => OrderDirection::Desc,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct OrderBy {
    pub expr: (Expression, ValueType),
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

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Text(String),
    Int(i64),
    Float(f64),
    Boolean(bool),
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Text(_) => write!(f, "text"),
            DataType::Int(_) => write!(f, "int"),
            DataType::Float(_) => write!(f, "float"),
            DataType::Boolean(_) => write!(f, "boolean"),
        }
    }
}

impl DataType {
    pub fn map_ast(data_type: crate::parser::ast::DataType) -> DataType {
        match data_type {
            crate::parser::ast::DataType::Text(e) => DataType::Text(e),
            crate::parser::ast::DataType::Int(e) => DataType::Int(e),
            crate::parser::ast::DataType::Float(e) => DataType::Float(e),
            crate::parser::ast::DataType::Boolean(e) => DataType::Boolean(e),
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Text => write!(f, "text"),
            ValueType::Int => write!(f, "int"),
            ValueType::Float => write!(f, "float"),
            ValueType::Boolean => write!(f, "boolean"),
        }
    }
}

impl ValueType {
    pub fn map_data_type(data_type: &DataType) -> ValueType {
        match data_type {
            DataType::Text(_) => ValueType::Text,
            DataType::Int(_) => ValueType::Int,
            DataType::Float(_) => ValueType::Float,
            DataType::Boolean(_) => ValueType::Boolean,
        }
    }
}
