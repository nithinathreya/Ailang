use super::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: Literal,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    Assign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Grouping {
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Array {
        elements: Vec<Expr>,
        span: Span,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
}

#[allow(dead_code)]
impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Variable { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Grouping { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Index { span, .. } => *span,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        initializer: Option<Expr>,
        span: Span,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Print {
        expr: Expr,
        span: Span,
    },
    Block {
        statements: Vec<Stmt>,
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
}

#[allow(dead_code)]
impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Assign { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::Function { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Print { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::Expr { span, .. } => *span,
        }
    }
}
