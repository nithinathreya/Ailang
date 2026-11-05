use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .arg("--help")
        .output()
        .expect("Failed to execute ailang --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AILang lets you write programs"));
    assert!(stdout.contains("repl"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("eval"));
    assert!(stdout.contains("transpile"));
}

#[test]
fn test_cli_check() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .arg("check")
        .output()
        .expect("Failed to execute ailang check");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust compiler"));
    assert!(stdout.contains("Backend health"));
}

#[test]
fn test_eval_hello_world() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", "print hello world", "--backend", "mock"])
        .output()
        .expect("Failed to evaluate hello world");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World from AILang & Rust!"));
}

#[test]
fn test_eval_fibonacci() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", "calculate first 10 fibonacci numbers", "--backend", "mock"])
        .output()
        .expect("Failed to evaluate fibonacci");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fibonacci sequence (first 10 numbers): 0 1 1 2 3 5 8 13 21 34"));
}

#[test]
fn test_eval_primes() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", "calculate prime numbers", "--backend", "mock"])
        .output()
        .expect("Failed to evaluate primes");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Prime numbers up to 50: [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]"));
}

#[test]
fn test_eval_sorting() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", "sort list", "--backend", "mock"])
        .output()
        .expect("Failed to evaluate sort");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sorted ascending: [3, 7, 13, 25, 42, 64, 89, 99]"));
}

#[test]
fn test_run_script_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["run", "examples/fibonacci.ail", "--backend", "mock"])
        .output()
        .expect("Failed to run script file");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fibonacci sequence"));
}

#[test]
fn test_transpile_to_file() {
    let temp_file = std::env::temp_dir().join("test_transpile_output.rs");
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args([
            "transpile",
            "calculate first 5 fibonacci numbers",
            "-o",
            temp_file.to_str().unwrap(),
            "--backend",
            "mock",
        ])
        .output()
        .expect("Failed to transpile");

    assert!(output.status.success());
    assert!(temp_file.exists());
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("fn main()"));
    assert!(content.contains("Fibonacci sequence"));
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_eval_native_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", "let x = 15; let y = 27; print x + y;"])
        .output()
        .expect("Failed to evaluate native code");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn test_run_native_script_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["run", "examples/interpreter_demo.ail"])
        .output()
        .expect("Failed to run native script file");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Factorial of 5 is:"));
    assert!(stdout.contains("120"));
    assert!(stdout.contains("Array length:"));
    assert!(stdout.contains("6"));
    assert!(stdout.contains("10"));
    assert!(stdout.contains("60"));
}
