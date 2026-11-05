use super::ast::{BinaryOp, Expr, Literal, Stmt, UnaryOp};
use anyhow::{anyhow, Result};
use std::collections::HashSet;

pub struct SemanticAnalyzer {
    scopes: Vec<HashSet<String>>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut global_scope = HashSet::new();
        // Register default global builtins
        for builtin in &[
            "print", "println", "len", "abs", "sqrt", "pow", "min", "max",
            "round", "floor", "ceil", "str", "clock", "type_of", "push", "pop",
        ] {
            global_scope.insert(builtin.to_string());
        }

        Self {
            scopes: vec![global_scope],
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains(name) {
                return true;
            }
        }
        false
    }

    pub fn analyze(&mut self, statements: &mut [Stmt]) -> Result<()> {
        for stmt in statements {
            self.analyze_stmt(stmt)?;
        }
        Ok(())
    }

    fn analyze_stmt(&mut self, stmt: &mut Stmt) -> Result<()> {
        match stmt {
            Stmt::Let { name, initializer, span: _ } => {
                if let Some(init) = initializer {
                    self.fold_constants(init);
                    self.analyze_expr(init)?;
                }
                self.declare(name);
                Ok(())
            }
            Stmt::Assign { name, value, span } => {
                self.fold_constants(value);
                self.analyze_expr(value)?;
                if !self.is_defined(name) {
                    return Err(anyhow!("Semantic error at {}: Assignment to undeclared variable '{}'", span, name));
                }
                Ok(())
            }
            Stmt::If { condition, then_branch, else_branch, span: _ } => {
                self.fold_constants(condition);
                self.analyze_expr(condition)?;
                self.enter_scope();
                self.analyze_stmt(then_branch)?;
                self.exit_scope();
                if let Some(else_b) = else_branch {
                    self.enter_scope();
                    self.analyze_stmt(else_b)?;
                    self.exit_scope();
                }
                Ok(())
            }
            Stmt::While { condition, body, span: _ } => {
                self.fold_constants(condition);
                self.analyze_expr(condition)?;
                self.enter_scope();
                self.analyze_stmt(body)?;
                self.exit_scope();
                Ok(())
            }
            Stmt::Function { name, params, body, span: _ } => {
                self.declare(name);
                self.enter_scope();
                for param in params {
                    self.declare(param);
                }
                for s in body {
                    self.analyze_stmt(s)?;
                }
                self.exit_scope();
                Ok(())
            }
            Stmt::Return { value, span: _ } => {
                if let Some(val) = value {
                    self.fold_constants(val);
                    self.analyze_expr(val)?;
                }
                Ok(())
            }
            Stmt::Print { expr, span: _ } => {
                self.fold_constants(expr);
                self.analyze_expr(expr)
            }
            Stmt::Block { statements, span: _ } => {
                self.enter_scope();
                for s in statements {
                    self.analyze_stmt(s)?;
                }
                self.exit_scope();
                Ok(())
            }
            Stmt::Expr { expr, span: _ } => {
                self.fold_constants(expr);
                self.analyze_expr(expr)
            }
        }
    }

    fn analyze_expr(&self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Literal { .. } => Ok(()),
            Expr::Assign { name, value, span } => {
                self.analyze_expr(value)?;
                if !self.is_defined(name) {
                    return Err(anyhow!("Semantic error at {}: Assignment to undeclared variable '{}'", span, name));
                }
                Ok(())
            }
            Expr::Variable { name, span } => {
                if !self.is_defined(name) {
                    return Err(anyhow!("Semantic error at {}: Use of undeclared variable '{}'", span, name));
                }
                Ok(())
            }
            Expr::Unary { right, .. } => self.analyze_expr(right),
            Expr::Binary { left, right, .. } => {
                self.analyze_expr(left)?;
                self.analyze_expr(right)
            }
            Expr::Grouping { expr, .. } => self.analyze_expr(expr),
            Expr::Call { callee, args, .. } => {
                self.analyze_expr(callee)?;
                for arg in args {
                    self.analyze_expr(arg)?;
                }
                Ok(())
            }
            Expr::Array { elements, .. } => {
                for el in elements {
                    self.analyze_expr(el)?;
                }
                Ok(())
            }
            Expr::Index { array, index, .. } => {
                self.analyze_expr(array)?;
                self.analyze_expr(index)
            }
        }
    }

    /// Constant folding optimization: evaluates constant sub-expressions at compile/analysis time
    pub fn fold_constants(&self, expr: &mut Expr) {
        match expr {
            Expr::Binary { left, op, right, span } => {
                self.fold_constants(left);
                self.fold_constants(right);

                if let (Expr::Literal { value: l_val, .. }, Expr::Literal { value: r_val, .. }) = (&**left, &**right) {
                    if let Some(folded) = fold_binary_literal(l_val, *op, r_val) {
                        *expr = Expr::Literal {
                            value: folded,
                            span: *span,
                        };
                    }
                }
            }
            Expr::Unary { op, right, span } => {
                self.fold_constants(right);
                if let Expr::Literal { value, .. } = &**right {
                    if let Some(folded) = fold_unary_literal(*op, value) {
                        *expr = Expr::Literal {
                            value: folded,
                            span: *span,
                        };
                    }
                }
            }
            Expr::Grouping { expr: inner, .. } => {
                self.fold_constants(inner);
                if let Expr::Literal { value, span } = &**inner {
                    *expr = Expr::Literal {
                        value: value.clone(),
                        span: *span,
                    };
                }
            }
            Expr::Call { callee, args, .. } => {
                self.fold_constants(callee);
                for arg in args {
                    self.fold_constants(arg);
                }
            }
            Expr::Array { elements, .. } => {
                for el in elements {
                    self.fold_constants(el);
                }
            }
            Expr::Index { array, index, .. } => {
                self.fold_constants(array);
                self.fold_constants(index);
            }
            Expr::Assign { value, .. } => {
                self.fold_constants(value);
            }
            _ => {}
        }
    }
}

fn fold_binary_literal(left: &Literal, op: BinaryOp, right: &Literal) -> Option<Literal> {
    match (left, op, right) {
        (Literal::Integer(a), BinaryOp::Add, Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_add(*b))),
        (Literal::Integer(a), BinaryOp::Subtract, Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_sub(*b))),
        (Literal::Integer(a), BinaryOp::Multiply, Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_mul(*b))),
        (Literal::Integer(a), BinaryOp::Divide, Literal::Integer(b)) if *b != 0 => Some(Literal::Integer(a / b)),
        (Literal::Integer(a), BinaryOp::Modulo, Literal::Integer(b)) if *b != 0 => Some(Literal::Integer(a % b)),

        (Literal::Float(a), BinaryOp::Add, Literal::Float(b)) => Some(Literal::Float(a + b)),
        (Literal::Float(a), BinaryOp::Subtract, Literal::Float(b)) => Some(Literal::Float(a - b)),
        (Literal::Float(a), BinaryOp::Multiply, Literal::Float(b)) => Some(Literal::Float(a * b)),
        (Literal::Float(a), BinaryOp::Divide, Literal::Float(b)) if *b != 0.0 => Some(Literal::Float(a / b)),

        (Literal::Integer(a), BinaryOp::Equal, Literal::Integer(b)) => Some(Literal::Boolean(a == b)),
        (Literal::Integer(a), BinaryOp::NotEqual, Literal::Integer(b)) => Some(Literal::Boolean(a != b)),
        (Literal::Integer(a), BinaryOp::Less, Literal::Integer(b)) => Some(Literal::Boolean(a < b)),
        (Literal::Integer(a), BinaryOp::LessEqual, Literal::Integer(b)) => Some(Literal::Boolean(a <= b)),
        (Literal::Integer(a), BinaryOp::Greater, Literal::Integer(b)) => Some(Literal::Boolean(a > b)),
        (Literal::Integer(a), BinaryOp::GreaterEqual, Literal::Integer(b)) => Some(Literal::Boolean(a >= b)),

        (Literal::Boolean(a), BinaryOp::And, Literal::Boolean(b)) => Some(Literal::Boolean(*a && *b)),
        (Literal::Boolean(a), BinaryOp::Or, Literal::Boolean(b)) => Some(Literal::Boolean(*a || *b)),

        (Literal::String(a), BinaryOp::Add, Literal::String(b)) => Some(Literal::String(format!("{}{}", a, b))),
        _ => None,
    }
}

fn fold_unary_literal(op: UnaryOp, val: &Literal) -> Option<Literal> {
    match (op, val) {
        (UnaryOp::Negate, Literal::Integer(n)) => Some(Literal::Integer(-n)),
        (UnaryOp::Negate, Literal::Float(f)) => Some(Literal::Float(-f)),
        (UnaryOp::Not, Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
        _ => None,
    }
}
