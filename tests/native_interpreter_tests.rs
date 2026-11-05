use ailang::interpreter::ast::{BinaryOp, Expr, Literal, Stmt};
use ailang::interpreter::evaluator::Evaluator;
use ailang::interpreter::lexer::Lexer;
use ailang::interpreter::parser::Parser;
use ailang::interpreter::semantic::SemanticAnalyzer;
use ailang::interpreter::token::TokenType;

#[test]
fn test_lexer_tokenization() {
    let source = r#"
        // Single line comment
        let x = 42;
        let pi = 3.14;
        let msg = "Hello\nWorld";
        /* Multi-line
           comment */
        if (x >= 10 && !false) {
            print msg;
        }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization should succeed");

    assert!(tokens.iter().any(|t| t.token_type == TokenType::Let));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::Integer(42)));
    assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::Float(f) if (f - 3.14).abs() < 1e-6)));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::String("Hello\nWorld".to_string())));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::GreaterEqual));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::And));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::Bang));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::Print));
    assert_eq!(tokens.last().unwrap().token_type, TokenType::Eof);
}

#[test]
fn test_parser_operator_precedence() {
    let source = "5 + 3 * 2 == 11;";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();

    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Stmt::Expr { expr, .. } => match expr {
            Expr::Binary { left, op, right, .. } => {
                assert_eq!(*op, BinaryOp::Equal);
                assert_eq!(**right, Expr::Literal { value: Literal::Integer(11), span: right.span() });
                // left should be 5 + (3 * 2)
                match &**left {
                    Expr::Binary { op: add_op, right: mul_expr, .. } => {
                        assert_eq!(*add_op, BinaryOp::Add);
                        match &**mul_expr {
                            Expr::Binary { op: mul_op, .. } => {
                                assert_eq!(*mul_op, BinaryOp::Multiply);
                            }
                            _ => panic!("Expected multiplication expression"),
                        }
                    }
                    _ => panic!("Expected addition expression"),
                }
            }
            _ => panic!("Expected binary equality expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_semantic_analysis_constant_folding() {
    let source = "let result = 2 + 3 * 4;";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let mut stmts = parser.parse().unwrap();

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&mut stmts).unwrap();

    // The expression 2 + 3 * 4 should be folded at compile time to 14!
    match &stmts[0] {
        Stmt::Let { initializer: Some(expr), .. } => {
            match expr {
                Expr::Literal { value: Literal::Integer(val), .. } => {
                    assert_eq!(*val, 14, "Constant folding should evaluate 2 + 3 * 4 to 14");
                }
                _ => panic!("Expected constant folded literal"),
            }
        }
        _ => panic!("Expected let statement"),
    }
}

#[test]
fn test_evaluator_arithmetic_and_variables() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        let a = 10;
        let b = 25;
        let sum = a + b;
        let diff = b - a;
        let prod = a * b;
        let quot = b / a;
        let rem = b % a;
        print sum;
        print diff;
        print prod;
        print quot;
        print rem;
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(evaluator.output_buffer, vec!["35", "15", "250", "2", "5"]);
}

#[test]
fn test_evaluator_functions_and_recursion() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        fn fib(n) {
            if (n <= 1) {
                return n;
            }
            return fib(n - 1) + fib(n - 2);
        }

        print fib(0);
        print fib(1);
        print fib(6);
        print fib(10);
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(evaluator.output_buffer, vec!["0", "1", "8", "55"]);
}

#[test]
fn test_evaluator_closures_and_scope() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        fn make_counter(start) {
            let count = start;
            fn next() {
                count = count + 1;
                return count;
            }
            return next;
        }

        let counter = make_counter(100);
        print counter();
        print counter();
        print counter();
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(evaluator.output_buffer, vec!["101", "102", "103"]);
}

#[test]
fn test_evaluator_builtins() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        print abs(-50);
        print sqrt(25);
        print pow(2, 4);
        print min(12, 42);
        print max(12, 42);
        print len("sourcebae");
        print len([1, 2, 3, 4, 5]);
        print type_of(3.14);
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(
        evaluator.output_buffer,
        vec!["50", "5", "16", "12", "42", "9", "5", "float"]
    );
}

#[test]
fn test_evaluator_arrays_and_indexing() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        let fruits = ["apple", "banana", "cherry"];
        print fruits[0];
        print fruits[1];
        print fruits[-1];
        push(fruits, "date");
        print len(fruits);
        print fruits[-1];
        let popped = pop(fruits);
        print popped;
        print len(fruits);
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(
        evaluator.output_buffer,
        vec!["apple", "banana", "cherry", "4", "date", "date", "3"]
    );
}

#[test]
fn test_while_and_for_loops() {
    let mut evaluator = Evaluator::new();
    let source = r#"
        let total = 0;
        for (let i = 1; i <= 5; i = i + 1) {
            total = total + i;
        }
        print total;

        let countdown = 3;
        while (countdown > 0) {
            print countdown;
            countdown = countdown - 1;
        }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    evaluator.evaluate_program(&stmts).unwrap();

    assert_eq!(evaluator.output_buffer, vec!["15", "3", "2", "1"]);
}
