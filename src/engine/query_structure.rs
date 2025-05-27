pub enum QueryNodes {
    Sort(Sorting),
    Limit(Limit),
    Scan(Scan),
    Filter(Filter),
    Project(Project),
    Insert(Insert),
    Update(Update),
    Join(Join),
}

pub struct Sorting {
    pub column_to_sort: Vec<Expr>,
    pub ordering_type: Vec<Expr>,
    pub query_node: Box<QueryNodes>,
}

pub struct Limit {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub query_node: Box<QueryNodes>,
}

pub struct Scan {
    pub table_name: String,
}

pub struct Filter {
    pub conditions: Vec<Expr>,
    pub scan_node: Box<QueryNodes>,
}

pub struct Project {
    pub projections: Vec<String>,
    pub query_node: Box<QueryNodes>,
}

pub enum DataType {
    Int(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}

pub struct Insert {
    pub table_name: String,
    pub columns: Vec<Expr>,
    pub values: Option<Vec<DataType>>,
    pub sub_query: Option<Box<QueryNodes>>,
}

pub struct Update {
    pub table_name: String,
    pub columns: Option<Vec<(String, Expr)>>,
    pub filter: Option<Box<QueryNodes>>,
}

pub struct Join {
    join_string: JoinType,
    left_table: Box<QueryNodes>,
    right_table: Box<QueryNodes>,
    on: Expr,
}

pub enum Expr {
    Column(String),
    Const(DataType),
    BinaryOp(BinaryOp),
    LogicalOp(LogicalOp),
    List(Vec<Expr>),
    SubQuery(Box<QueryNodes>),
    SortingOrder(SortingOrder),
}

pub struct SortingOrder {
    pub is_asc: bool,
}

pub enum LogicalOp {
    And,
    Or,
    Not,
    IsNull,
    NotNull,
    In,
}

pub enum BinaryOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Le,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
}

pub enum JoinType {
    Full,
    Left,
    Right,
    Outer,
}
