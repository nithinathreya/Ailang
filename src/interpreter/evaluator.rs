use super::ast::{BinaryOp, Expr, Literal, Stmt, UnaryOp};
use super::environment::Environment;
use super::value::Value;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum Signal {
    Return(Value),
}

pub struct Evaluator {
    #[allow(dead_code)]
    pub global_env: Rc<RefCell<Environment>>,
    pub env: Rc<RefCell<Environment>>,
    pub output_buffer: Vec<String>,
}

impl Evaluator {
    pub fn new() -> Self {
        let global = Rc::new(RefCell::new(Environment::new()));
        Self::register_builtins(&global);

        Self {
            global_env: Rc::clone(&global),
            env: global,
            output_buffer: Vec::new(),
        }
    }

    fn register_builtins(env: &Rc<RefCell<Environment>>) {
        let mut e = env.borrow_mut();
        let builtins = vec![
            "print", "println", "len", "abs", "sqrt", "pow", "min", "max",
            "round", "floor", "ceil", "str", "clock", "type_of", "push", "pop",
        ];
        for b in builtins {
            e.define(b.to_string(), Value::Builtin(b.to_string()));
        }
    }

    pub fn evaluate_program(&mut self, statements: &[Stmt]) -> Result<Option<Value>> {
        let mut last_value = None;
        for stmt in statements {
            let (val, signal) = self.execute_stmt(stmt)?;
            if let Some(Signal::Return(ret)) = signal {
                return Ok(Some(ret));
            }
            if let Some(v) = val {
                last_value = Some(v);
            }
        }
        Ok(last_value)
    }

    pub fn execute_stmt(&mut self, stmt: &Stmt) -> Result<(Option<Value>, Option<Signal>)> {
        match stmt {
            Stmt::Let { name, initializer, .. } => {
                let value = if let Some(init) = initializer {
                    self.evaluate_expr(init)?
                } else {
                    Value::Nil
                };
                self.env.borrow_mut().define(name.clone(), value);
                Ok((None, None))
            }
            Stmt::Assign { name, value, span } => {
                let val = self.evaluate_expr(value)?;
                if !self.env.borrow_mut().assign(name, val.clone()) {
                    return Err(anyhow!("Runtime error at {}: Undefined variable '{}'", span, name));
                }
                Ok((Some(val), None))
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.evaluate_expr(condition)?;
                if cond_val.is_truthy() {
                    self.execute_stmt(then_branch)
                } else if let Some(else_b) = else_branch {
                    self.execute_stmt(else_b)
                } else {
                    Ok((None, None))
                }
            }
            Stmt::While { condition, body, .. } => {
                let mut last_body_val = None;
                while self.evaluate_expr(condition)?.is_truthy() {
                    let (val, signal) = self.execute_stmt(body)?;
                    if let Some(sig) = signal {
                        return Ok((None, Some(sig)));
                    }
                    if let Some(v) = val {
                        last_body_val = Some(v);
                    }
                }
                Ok((last_body_val, None))
            }
            Stmt::Function { name, params, body, .. } => {
                let func = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                };
                self.env.borrow_mut().define(name.clone(), func);
                Ok((None, None))
            }
            Stmt::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    self.evaluate_expr(expr)?
                } else {
                    Value::Nil
                };
                Ok((None, Some(Signal::Return(val))))
            }
            Stmt::Print { expr, .. } => {
                let val = self.evaluate_expr(expr)?;
                let output_str = format!("{}", val);
                self.output_buffer.push(output_str);
                Ok((None, None))
            }
            Stmt::Block { statements, .. } => {
                let prev_env = Rc::clone(&self.env);
                self.env = Rc::new(RefCell::new(Environment::with_enclosing(Rc::clone(&prev_env))));

                let mut last_block_val = None;
                let mut sig = None;
                for s in statements {
                    let (val, signal) = self.execute_stmt(s)?;
                    if let Some(s_val) = signal {
                        sig = Some(s_val);
                        break;
                    }
                    if let Some(v) = val {
                        last_block_val = Some(v);
                    }
                }

                self.env = prev_env;
                Ok((last_block_val, sig))
            }
            Stmt::Expr { expr, .. } => {
                let val = self.evaluate_expr(expr)?;
                Ok((Some(val), None))
            }
        }
    }

    pub fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Literal { value, .. } => Ok(match value {
                Literal::Integer(n) => Value::Integer(*n),
                Literal::Float(f) => Value::Float(*f),
                Literal::String(s) => Value::String(s.clone()),
                Literal::Boolean(b) => Value::Boolean(*b),
                Literal::Nil => Value::Nil,
            }),
            Expr::Variable { name, span } => {
                self.env.borrow().get(name).ok_or_else(|| {
                    anyhow!("Runtime error at {}: Undefined variable '{}'", span, name)
                })
            }
            Expr::Assign { name, value, span } => {
                let val = self.evaluate_expr(value)?;
                if !self.env.borrow_mut().assign(name, val.clone()) {
                    return Err(anyhow!("Runtime error at {}: Undefined variable '{}'", span, name));
                }
                Ok(val)
            }
            Expr::Grouping { expr, .. } => self.evaluate_expr(expr),
            Expr::Unary { op, right, span } => {
                let val = self.evaluate_expr(right)?;
                match op {
                    UnaryOp::Negate => match val {
                        Value::Integer(n) => Ok(Value::Integer(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(anyhow!("Runtime error at {}: Cannot negate type {}", span, val.type_name())),
                    },
                    UnaryOp::Not => Ok(Value::Boolean(!val.is_truthy())),
                }
            }
            Expr::Binary { left, op, right, span } => {
                // Short-circuiting for logical operators
                if *op == BinaryOp::And {
                    let l_val = self.evaluate_expr(left)?;
                    return if !l_val.is_truthy() {
                        Ok(l_val)
                    } else {
                        self.evaluate_expr(right)
                    };
                }
                if *op == BinaryOp::Or {
                    let l_val = self.evaluate_expr(left)?;
                    return if l_val.is_truthy() {
                        Ok(l_val)
                    } else {
                        self.evaluate_expr(right)
                    };
                }

                let l_val = self.evaluate_expr(left)?;
                let r_val = self.evaluate_expr(right)?;

                match op {
                    BinaryOp::Add => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
                        (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b))),
                        (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                        _ => Err(anyhow!("Runtime error at {}: Invalid operands for '+': {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::Subtract => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
                        _ => Err(anyhow!("Runtime error at {}: Invalid operands for '-': {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::Multiply => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
                        _ => Err(anyhow!("Runtime error at {}: Invalid operands for '*': {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::Divide => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if *b == 0 {
                                return Err(anyhow!("Runtime error at {}: Division by zero", span));
                            }
                            Ok(Value::Integer(a / b))
                        }
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a / *b as f64)),
                        _ => Err(anyhow!("Runtime error at {}: Invalid operands for '/': {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::Modulo => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if *b == 0 {
                                return Err(anyhow!("Runtime error at {}: Modulo by zero", span));
                            }
                            Ok(Value::Integer(a % b))
                        }
                        _ => Err(anyhow!("Runtime error at {}: Modulo operator only supports integers", span)),
                    },
                    BinaryOp::Equal => Ok(Value::Boolean(l_val == r_val)),
                    BinaryOp::NotEqual => Ok(Value::Boolean(l_val != r_val)),
                    BinaryOp::Less => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) < *b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a < (*b as f64))),
                        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
                        _ => Err(anyhow!("Runtime error at {}: Cannot compare {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::LessEqual => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) <= *b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a <= (*b as f64))),
                        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a <= b)),
                        _ => Err(anyhow!("Runtime error at {}: Cannot compare {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::Greater => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) > *b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a > (*b as f64))),
                        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a > b)),
                        _ => Err(anyhow!("Runtime error at {}: Cannot compare {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    BinaryOp::GreaterEqual => match (&l_val, &r_val) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) >= *b)),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a >= (*b as f64))),
                        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a >= b)),
                        _ => Err(anyhow!("Runtime error at {}: Cannot compare {} and {}", span, l_val.type_name(), r_val.type_name())),
                    },
                    _ => unreachable!(),
                }
            }
            Expr::Array { elements, .. } => {
                let mut vals = Vec::new();
                for el in elements {
                    vals.push(self.evaluate_expr(el)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(vals))))
            }
            Expr::Index { array, index, span } => {
                let arr_val = self.evaluate_expr(array)?;
                let idx_val = self.evaluate_expr(index)?;

                match (&arr_val, &idx_val) {
                    (Value::Array(elements), Value::Integer(idx)) => {
                        let elements = elements.borrow();
                        let real_idx = if *idx < 0 {
                            (elements.len() as i64 + idx) as usize
                        } else {
                            *idx as usize
                        };
                        elements.get(real_idx).cloned().ok_or_else(|| {
                            anyhow!("Runtime error at {}: Array index {} out of bounds (len {})", span, idx, elements.len())
                        })
                    }
                    (Value::String(s), Value::Integer(idx)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let real_idx = if *idx < 0 {
                            (chars.len() as i64 + idx) as usize
                        } else {
                            *idx as usize
                        };
                        chars.get(real_idx).map(|c| Value::String(c.to_string())).ok_or_else(|| {
                            anyhow!("Runtime error at {}: String index {} out of bounds (len {})", span, idx, chars.len())
                        })
                    }
                    _ => Err(anyhow!("Runtime error at {}: Cannot index {} with {}", span, arr_val.type_name(), idx_val.type_name())),
                }
            }
            Expr::Call { callee, args, span } => {
                let callee_val = self.evaluate_expr(callee)?;
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.evaluate_expr(arg)?);
                }

                match callee_val {
                    Value::Builtin(name) => self.call_builtin(&name, &evaluated_args, *span),
                    Value::Function { params, body, closure, .. } => {
                        if params.len() != evaluated_args.len() {
                            return Err(anyhow!(
                                "Runtime error at {}: Expected {} arguments, found {}",
                                span,
                                params.len(),
                                evaluated_args.len()
                            ));
                        }

                        let prev_env = Rc::clone(&self.env);
                        let fn_env = Rc::new(RefCell::new(Environment::with_enclosing(closure)));
                        for (param, arg) in params.iter().zip(evaluated_args.into_iter()) {
                            fn_env.borrow_mut().define(param.clone(), arg);
                        }

                        self.env = fn_env;
                        let mut ret_val = Value::Nil;
                        for stmt in &body {
                            let (val, signal) = self.execute_stmt(stmt)?;
                            if let Some(Signal::Return(v)) = signal {
                                ret_val = v;
                                break;
                            }
                            if let Some(v) = val {
                                ret_val = v;
                            }
                        }

                        self.env = prev_env;
                        Ok(ret_val)
                    }
                    _ => Err(anyhow!("Runtime error at {}: Cannot call non-function type {}", span, callee_val.type_name())),
                }
            }
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Value], span: super::token::Span) -> Result<Value> {
        match name {
            "print" => {
                let parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
                let out = parts.join(" ");
                self.output_buffer.push(out);
                Ok(Value::Nil)
            }
            "println" => {
                let parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
                let out = parts.join(" ");
                self.output_buffer.push(format!("{}\n", out));
                Ok(Value::Nil)
            }
            "len" => {
                if let Some(arg) = args.first() {
                    match arg {
                        Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                        Value::Array(a) => Ok(Value::Integer(a.borrow().len() as i64)),
                        _ => Err(anyhow!("len() requires string or array, got {}", arg.type_name())),
                    }
                } else {
                    Err(anyhow!("len() expects 1 argument"))
                }
            }
            "abs" => {
                if let Some(arg) = args.first() {
                    match arg {
                        Value::Integer(n) => Ok(Value::Integer(n.abs())),
                        Value::Float(f) => Ok(Value::Float(f.abs())),
                        _ => Err(anyhow!("abs() requires numeric argument")),
                    }
                } else {
                    Err(anyhow!("abs() expects 1 argument"))
                }
            }
            "sqrt" => {
                if let Some(arg) = args.first() {
                    let num = match arg {
                        Value::Integer(n) => *n as f64,
                        Value::Float(f) => *f,
                        _ => return Err(anyhow!("sqrt() requires numeric argument")),
                    };
                    Ok(Value::Float(num.sqrt()))
                } else {
                    Err(anyhow!("sqrt() expects 1 argument"))
                }
            }
            "pow" => {
                if args.len() == 2 {
                    let base = match &args[0] {
                        Value::Integer(n) => *n as f64,
                        Value::Float(f) => *f,
                        _ => return Err(anyhow!("pow() requires numeric base")),
                    };
                    let exp = match &args[1] {
                        Value::Integer(n) => *n as f64,
                        Value::Float(f) => *f,
                        _ => return Err(anyhow!("pow() requires numeric exponent")),
                    };
                    Ok(Value::Float(base.powf(exp)))
                } else {
                    Err(anyhow!("pow() expects 2 arguments"))
                }
            }
            "min" => {
                if args.len() == 2 {
                    match (&args[0], &args[1]) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.min(*b as f64))),
                        _ => Err(anyhow!("min() requires numbers")),
                    }
                } else {
                    Err(anyhow!("min() expects 2 arguments"))
                }
            }
            "max" => {
                if args.len() == 2 {
                    match (&args[0], &args[1]) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.max(*b as f64))),
                        _ => Err(anyhow!("max() requires numbers")),
                    }
                } else {
                    Err(anyhow!("max() expects 2 arguments"))
                }
            }
            "round" => {
                if let Some(arg) = args.first() {
                    match arg {
                        Value::Integer(n) => Ok(Value::Integer(*n)),
                        Value::Float(f) => Ok(Value::Integer(f.round() as i64)),
                        _ => Err(anyhow!("round() requires number")),
                    }
                } else {
                    Err(anyhow!("round() expects 1 argument"))
                }
            }
            "floor" => {
                if let Some(arg) = args.first() {
                    match arg {
                        Value::Integer(n) => Ok(Value::Integer(*n)),
                        Value::Float(f) => Ok(Value::Integer(f.floor() as i64)),
                        _ => Err(anyhow!("floor() requires number")),
                    }
                } else {
                    Err(anyhow!("floor() expects 1 argument"))
                }
            }
            "ceil" => {
                if let Some(arg) = args.first() {
                    match arg {
                        Value::Integer(n) => Ok(Value::Integer(*n)),
                        Value::Float(f) => Ok(Value::Integer(f.ceil() as i64)),
                        _ => Err(anyhow!("ceil() requires number")),
                    }
                } else {
                    Err(anyhow!("ceil() expects 1 argument"))
                }
            }
            "str" => {
                if let Some(arg) = args.first() {
                    Ok(Value::String(format!("{}", arg)))
                } else {
                    Ok(Value::String(String::new()))
                }
            }
            "type_of" => {
                if let Some(arg) = args.first() {
                    Ok(Value::String(arg.type_name().to_string()))
                } else {
                    Err(anyhow!("type_of() expects 1 argument"))
                }
            }
            "clock" => {
                let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                Ok(Value::Float(duration.as_secs_f64()))
            }
            "push" => {
                if args.len() == 2 {
                    if let Value::Array(ref arr) = args[0] {
                        arr.borrow_mut().push(args[1].clone());
                        Ok(Value::Nil)
                    } else {
                        Err(anyhow!("push() first argument must be an array"))
                    }
                } else {
                    Err(anyhow!("push() expects 2 arguments (array, value)"))
                }
            }
            "pop" => {
                if let Some(Value::Array(arr)) = args.first() {
                    Ok(arr.borrow_mut().pop().unwrap_or(Value::Nil))
                } else {
                    Err(anyhow!("pop() requires an array argument"))
                }
            }
            _ => Err(anyhow!("Unknown builtin '{}' at {}", name, span)),
        }
    }
}
