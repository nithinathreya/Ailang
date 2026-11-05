use super::ast::{BinaryOp, Expr, Literal, Stmt, UnaryOp};
use super::token::{Token, TokenType};
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(statements)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(&token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            let peek_tok = self.peek();
            Err(anyhow!(
                "Syntax error at {}: Expected {}, found '{}'",
                peek_tok.span,
                message,
                peek_tok.lexeme
            ))
        }
    }

    // --- Grammar Rules ---

    fn declaration(&mut self) -> Result<Stmt> {
        if self.match_token(TokenType::Let) {
            self.var_declaration()
        } else if self.match_token(TokenType::Fn) {
            self.function_declaration()
        } else {
            self.statement()
        }
    }

    fn var_declaration(&mut self) -> Result<Stmt> {
        let let_span = self.previous().span;
        let name_tok = self.consume(TokenType::Ident(String::new()), "variable identifier")?.clone();
        let name = match name_tok.token_type {
            TokenType::Ident(ref id) => id.clone(),
            _ => unreachable!(),
        };

        let initializer = if self.match_token(TokenType::Equal) {
            Some(self.expression()?)
        } else {
            None
        };

        let _ = self.match_token(TokenType::Semicolon); // Semicolon is optional at EOF/REPL
        Ok(Stmt::Let {
            name,
            initializer,
            span: let_span,
        })
    }

    fn function_declaration(&mut self) -> Result<Stmt> {
        let fn_span = self.previous().span;
        let name_tok = self.consume(TokenType::Ident(String::new()), "function name")?.clone();
        let name = match name_tok.token_type {
            TokenType::Ident(ref id) => id.clone(),
            _ => unreachable!(),
        };

        self.consume(TokenType::LeftParen, "'(' after function name")?;
        let mut params = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_tok = self.consume(TokenType::Ident(String::new()), "parameter name")?;
                if let TokenType::Ident(ref p) = param_tok.token_type {
                    params.push(p.clone());
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "')' after parameters")?;

        self.consume(TokenType::LeftBrace, "'{' before function body")?;
        let body = self.block_statements()?;
        Ok(Stmt::Function {
            name,
            params,
            body,
            span: fn_span,
        })
    }

    fn statement(&mut self) -> Result<Stmt> {
        if self.match_token(TokenType::If) {
            self.if_statement()
        } else if self.match_token(TokenType::While) {
            self.while_statement()
        } else if self.match_token(TokenType::For) {
            self.for_statement()
        } else if self.match_token(TokenType::Return) {
            self.return_statement()
        } else if self.match_token(TokenType::Print) {
            self.print_statement()
        } else if self.match_token(TokenType::LeftBrace) {
            let span = self.previous().span;
            let statements = self.block_statements()?;
            Ok(Stmt::Block { statements, span })
        } else {
            self.assignment_or_expression_statement()
        }
    }

    fn if_statement(&mut self) -> Result<Stmt> {
        let span = self.previous().span;
        let has_paren = self.match_token(TokenType::LeftParen);
        let condition = self.expression()?;
        if has_paren {
            self.consume(TokenType::RightParen, "')' after if condition")?;
        }

        let then_branch = Box::new(self.statement()?);
        let else_branch = if self.match_token(TokenType::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt> {
        let span = self.previous().span;
        let has_paren = self.match_token(TokenType::LeftParen);
        let condition = self.expression()?;
        if has_paren {
            self.consume(TokenType::RightParen, "')' after while condition")?;
        }

        let body = Box::new(self.statement()?);
        Ok(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn for_statement(&mut self) -> Result<Stmt> {
        let span = self.previous().span;
        self.consume(TokenType::LeftParen, "'(' after 'for'")?;

        let initializer = if self.match_token(TokenType::Semicolon) {
            None
        } else if self.match_token(TokenType::Let) {
            Some(self.var_declaration()?)
        } else {
            Some(self.assignment_or_expression_statement()?)
        };

        let condition = if !self.check(&TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::Semicolon, "';' after loop condition")?;

        let increment = if !self.check(&TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RightParen, "')' after for clauses")?;

        let mut body = self.statement()?;

        // Desugar for-loop into a while-loop
        if let Some(inc) = increment {
            body = Stmt::Block {
                span,
                statements: vec![body, Stmt::Expr { expr: inc, span }],
            };
        }

        let cond = condition.unwrap_or(Expr::Literal {
            value: Literal::Boolean(true),
            span,
        });

        body = Stmt::While {
            condition: cond,
            body: Box::new(body),
            span,
        };

        if let Some(init) = initializer {
            body = Stmt::Block {
                span,
                statements: vec![init, body],
            };
        }

        Ok(body)
    }

    fn return_statement(&mut self) -> Result<Stmt> {
        let span = self.previous().span;
        let value = if !self.check(&TokenType::Semicolon) && !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            Some(self.expression()?)
        } else {
            None
        };
        let _ = self.match_token(TokenType::Semicolon);
        Ok(Stmt::Return { value, span })
    }

    fn print_statement(&mut self) -> Result<Stmt> {
        let span = self.previous().span;
        let expr = self.expression()?;
        let _ = self.match_token(TokenType::Semicolon);
        Ok(Stmt::Print { expr, span })
    }

    fn block_statements(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        self.consume(TokenType::RightBrace, "'}' after block")?;
        Ok(statements)
    }

    fn assignment_or_expression_statement(&mut self) -> Result<Stmt> {
        let span = self.peek().span;
        let expr = self.expression()?;
        let _ = self.match_token(TokenType::Semicolon);
        Ok(Stmt::Expr { expr, span })
    }

    pub fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.logical_or()?;

        if self.match_token(TokenType::Equal) {
            let equals_span = self.previous().span;
            let value = self.assignment()?;

            if let Expr::Variable { name, span } = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                    span,
                });
            }

            return Err(anyhow!("Invalid assignment target at {}", equals_span));
        }

        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Expr> {
        let mut expr = self.logical_and()?;
        while self.match_token(TokenType::Or) {
            let span = self.previous().span;
            let right = self.logical_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn logical_and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_token(TokenType::And) {
            let span = self.previous().span;
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.comparison()?;
        while self.match_token(TokenType::BangEqual) || self.match_token(TokenType::EqualEqual) {
            let op_tok = self.previous().clone();
            let op = if op_tok.token_type == TokenType::EqualEqual {
                BinaryOp::Equal
            } else {
                BinaryOp::NotEqual
            };
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: op_tok.span,
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr> {
        let mut expr = self.term()?;
        while self.match_token(TokenType::Greater)
            || self.match_token(TokenType::GreaterEqual)
            || self.match_token(TokenType::Less)
            || self.match_token(TokenType::LessEqual)
        {
            let op_tok = self.previous().clone();
            let op = match op_tok.token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: op_tok.span,
            };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr> {
        let mut expr = self.factor()?;
        while self.match_token(TokenType::Plus) || self.match_token(TokenType::Minus) {
            let op_tok = self.previous().clone();
            let op = if op_tok.token_type == TokenType::Plus {
                BinaryOp::Add
            } else {
                BinaryOp::Subtract
            };
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: op_tok.span,
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr> {
        let mut expr = self.unary()?;
        while self.match_token(TokenType::Star) || self.match_token(TokenType::Slash) || self.match_token(TokenType::Percent) {
            let op_tok = self.previous().clone();
            let op = match op_tok.token_type {
                TokenType::Star => BinaryOp::Multiply,
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span: op_tok.span,
            };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.match_token(TokenType::Bang) || self.match_token(TokenType::Minus) {
            let op_tok = self.previous().clone();
            let op = if op_tok.token_type == TokenType::Bang {
                UnaryOp::Not
            } else {
                UnaryOp::Negate
            };
            let right = self.unary()?;
            return Ok(Expr::Unary {
                op,
                right: Box::new(right),
                span: op_tok.span,
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenType::LeftParen) {
                let span = self.previous().span;
                let mut args = Vec::new();
                if !self.check(&TokenType::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }
                self.consume(TokenType::RightParen, "')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.match_token(TokenType::LeftBracket) {
                let span = self.previous().span;
                let index = self.expression()?;
                self.consume(TokenType::RightBracket, "']' after index")?;
                expr = Expr::Index {
                    array: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr> {
        let token = self.advance().clone();
        match token.token_type {
            TokenType::Integer(n) => Ok(Expr::Literal {
                value: Literal::Integer(n),
                span: token.span,
            }),
            TokenType::Float(n) => Ok(Expr::Literal {
                value: Literal::Float(n),
                span: token.span,
            }),
            TokenType::String(s) => Ok(Expr::Literal {
                value: Literal::String(s),
                span: token.span,
            }),
            TokenType::Boolean(b) => Ok(Expr::Literal {
                value: Literal::Boolean(b),
                span: token.span,
            }),
            TokenType::Nil => Ok(Expr::Literal {
                value: Literal::Nil,
                span: token.span,
            }),
            TokenType::Ident(name) => Ok(Expr::Variable {
                name,
                span: token.span,
            }),
            TokenType::LeftParen => {
                let expr = self.expression()?;
                self.consume(TokenType::RightParen, "')' after grouping expression")?;
                Ok(Expr::Grouping {
                    expr: Box::new(expr),
                    span: token.span,
                })
            }
            TokenType::LeftBracket => {
                let mut elements = Vec::new();
                if !self.check(&TokenType::RightBracket) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }
                self.consume(TokenType::RightBracket, "']' after array elements")?;
                Ok(Expr::Array {
                    elements,
                    span: token.span,
                })
            }
            _ => Err(anyhow!(
                "Syntax error at {}: Unexpected token '{}'",
                token.span,
                token.lexeme
            )),
        }
    }
}
