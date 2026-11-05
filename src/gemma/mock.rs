use super::GemmaBackend;
use anyhow::Result;

pub struct MockBackend;

impl MockBackend {
    pub fn new() -> Self {
        Self
    }
}

impl GemmaBackend for MockBackend {
    fn name(&self) -> &str {
        "Offline Synthesizer (Built-in)"
    }

    fn translate(&self, prompt: &str, _system_prompt: &str) -> Result<String> {
        let p = prompt.trim();
        let p_lower = p.to_lowercase();

        // 1. Check for hello world
        if p_lower.contains("hello world") || p_lower == "hello" || p_lower == "hi" {
            return Ok(r#"```rust
fn main() {
    println!("Hello, World from AILang & Rust!");
}
```"#.to_string());
        }

        // 2. Fibonacci
        if p_lower.contains("fibonacci") {
            let n: usize = p_lower
                .split_whitespace()
                .filter_map(|w| w.parse::<usize>().ok())
                .next()
                .unwrap_or(15);

            return Ok(format!(
                r#"```rust
fn main() {{
    let n = {};
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    print!("Fibonacci sequence (first {{}} numbers): ", n);
    for i in 0..n {{
        print!("{{}} ", a);
        let next = a + b;
        a = b;
        b = next;
    }}
    println!();
}}
```"#,
                n
            ));
        }

        // 3. Primes / sieve
        if p_lower.contains("prime") {
            return Ok(r#"```rust
fn is_prime(n: u64) -> bool {
    if n <= 1 { return false; }
    if n <= 3 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return false; }
        i += 6;
    }
    true
}

fn main() {
    let limit = 50;
    let primes: Vec<u64> = (1..=limit).filter(|&x| is_prime(x)).collect();
    println!("Prime numbers up to {}: {:?}", limit, primes);
}
```"#.to_string());
        }

        // 4. Factorial
        if p_lower.contains("factorial") {
            let n: u64 = p_lower
                .split_whitespace()
                .filter_map(|w| w.parse::<u64>().ok())
                .next()
                .unwrap_or(10);

            return Ok(format!(
                r#"```rust
fn factorial(n: u64) -> u64 {{
    (1..=n).product()
}}

fn main() {{
    let n = {};
    println!("Factorial of {{}} is {{}}", n, factorial(n));
}}
```"#,
                n
            ));
        }

        // 5. Sorting or array manipulation
        if p_lower.contains("sort") {
            return Ok(r#"```rust
fn main() {
    let mut numbers = vec![42, 13, 89, 7, 25, 64, 3, 99];
    println!("Original array: {:?}", numbers);
    numbers.sort();
    println!("Sorted ascending: {:?}", numbers);
    numbers.reverse();
    println!("Sorted descending: {:?}", numbers);
}
```"#.to_string());
        }

        // 6. Reverse string
        if p_lower.contains("reverse") && p_lower.contains("string") {
            return Ok(r#"```rust
fn main() {
    let text = "Hello, Gemma and Rust!";
    let reversed: String = text.chars().rev().collect();
    println!("Original: {}", text);
    println!("Reversed: {}", reversed);
}
```"#.to_string());
        }

        // 7. Math / arithmetic expression
        if p_lower.contains("calculate") || p_lower.contains("math") || p_lower.contains("evaluate") {
            return Ok(r#"```rust
fn main() {
    let a: f64 = 42.0;
    let b: f64 = 17.5;
    println!("Calculation results:");
    println!("  {} + {} = {}", a, b, a + b);
    println!("  {} - {} = {}", a, b, a - b);
    println!("  {} * {} = {}", a, b, a * b);
    println!("  {} / {} = {:.4}", a, b, a / b);
    println!("  sqrt({}) = {:.4}", a, a.sqrt());
}
```"#.to_string());
        }

        // 8. Mandelbrot / ASCII art
        if p_lower.contains("mandelbrot") {
            return Ok(r##"```rust
fn main() {
    let width = 60;
    let height = 24;
    let max_iter = 50;

    for y in 0..height {
        let cy = (y as f64 / height as f64) * 2.0 - 1.0;
        for x in 0..width {
            let cx = (x as f64 / width as f64) * 3.0 - 2.0;
            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iter = 0;
            while zx * zx + zy * zy < 4.0 && iter < max_iter {
                let tmp = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = tmp;
                iter += 1;
            }
            if iter == max_iter {
                print!("#");
            } else if iter > 20 {
                print!("*");
            } else if iter > 10 {
                print!(".");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}
```"##.to_string());
        }

        // 9. Struct / Object creation
        if p_lower.contains("struct") || p_lower.contains("person") {
            return Ok(r#"```rust
#[derive(Debug, Clone)]
struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: &str, age: u32) -> Self {
        Self {
            name: name.to_string(),
            age,
        }
    }

    fn greet(&self) {
        println!("Hello! My name is {} and I am {} years old.", self.name, self.age);
    }
}

fn main() {
    let alice = Person::new("Alice", 28);
    let bob = Person::new("Bob", 34);
    alice.greet();
    bob.greet();
    println!("Data: {:?}", vec![alice, bob]);
}
```"#.to_string());
        }

        // 10. HTTP server / Web server
        if p_lower.contains("http") || p_lower.contains("web server") || (p_lower.contains("server") && !p_lower.contains("mock")) {
            // Check if this is a self-testing HTTP server instruction
            if p_lower.contains("test") || p_lower.contains("client") || p_lower.contains("verify") {
                return Ok(r#"```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    println!("HTTP server listening on http://127.0.0.1:{}", port);

    let server_handle = thread::spawn(move || {
        for stream in listener.incoming().take(1) {
            if let Ok(mut stream) = stream {
                let mut buffer = [0; 1024];
                let _ = stream.read(&mut buffer);
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\nConnection: close\r\n\r\nHello, World!";
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        }
    });

    thread::sleep(Duration::from_millis(50));

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Failed to connect to server");
    client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("Failed to send request");
    let mut response = String::new();
    client.read_to_string(&mut response).expect("Failed to read response");
    println!("Client received HTTP response:\n{}", response.trim());

    server_handle.join().unwrap();
    println!("HTTP server self-test completed successfully!");
}
```"#.to_string());
            }

            // Extract port if specified, default to 8080
            let default_port: u16 = {
                let words: Vec<&str> = p_lower.split_whitespace().collect();
                let mut found_port = None;
                for (i, w) in words.iter().enumerate() {
                    if *w == "port" && i + 1 < words.len() {
                        if let Ok(num) = words[i + 1].trim_matches(|c: char| !c.is_ascii_digit()).parse::<u16>() {
                            found_port = Some(num);
                            break;
                        }
                    }
                }
                if found_port.is_none() {
                    found_port = p_lower
                        .split(|c: char| !c.is_ascii_digit())
                        .filter_map(|w| w.parse::<u16>().ok())
                        .find(|&p| p >= 80);
                }
                found_port.unwrap_or(8080)
            };

            return Ok(format!(
                r#"```rust
use std::io::{{Read, Write}};
use std::net::{{TcpListener, TcpStream}};

fn handle_client(mut stream: TcpStream) -> bool {{
    let mut buffer = [0; 2048];
    let bytes_read = match stream.read(&mut buffer) {{
        Ok(n) if n > 0 => n,
        _ => return false,
    }};

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    println!("Handling request: {{}} {{}}", method, path);

    let mut should_shutdown = false;
    let (status, content_type, body) = match path {{
        "/shutdown" => {{
            should_shutdown = true;
            ("200 OK", "text/plain", "Server shutting down...\n")
        }}
        "/health" => {{
            ("200 OK", "application/json", "{{\"status\":\"ok\",\"server\":\"ailang\"}}\n")
        }}
        "/hello" => {{
            ("200 OK", "text/plain", "Hello, World from AILang HTTP Server!\n")
        }}
        _ => {{
            ("200 OK", "text/html; charset=utf-8", "<h1>Welcome to AILang HTTP Server!</h1>\n<p>Running natively compiled Rust code.</p>\n")
        }}
    }};

    let response = format!(
        "HTTP/1.1 {{}}\r\nContent-Type: {{}}\r\nContent-Length: {{}}\r\nConnection: close\r\n\r\n{{}}",
        status,
        content_type,
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    should_shutdown
}}

fn main() {{
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or({});

    let listener = TcpListener::bind(format!("127.0.0.1:{{}}", port))
        .expect("Failed to bind TCP listener");
    let local_port = listener.local_addr().unwrap().port();
    println!("HTTP server listening on http://127.0.0.1:{{}}", local_port);
    let _ = std::io::stdout().flush();

    for stream in listener.incoming() {{
        match stream {{
            Ok(stream) => {{
                if handle_client(stream) {{
                    println!("Shutdown request received. Server stopping.");
                    let _ = std::io::stdout().flush();
                    break;
                }}
            }}
            Err(e) => {{
                eprintln!("Connection failed: {{}}", e);
            }}
        }}
    }}
}}
```"#,
                default_port
            ));
        }

        // 11. Print statement / expression
        if p_lower.starts_with("print ") || p_lower.starts_with("println ") {
            let content = p[5..].trim().trim_matches('"');
            return Ok(format!(
                r#"```rust
fn main() {{
    println!("{}", "{}");
}}
```"#,
                "{}", content
            ));
        }

        // Generic fallback with valid runnable Rust code
        let escaped_prompt = p.replace('\\', "\\\\").replace('"', "\\\"");
        Ok(format!(
            r#"```rust
// AILang offline mode generated program
fn main() {{
    println!(">>> Running AILang instruction: \"{}\"", "{}");
    println!("---");
    println!("Code synthesized and executed in safe native Rust.");
    println!("Tip: Set GEMMA_BACKEND=ollama or GEMMA_API_KEY for dynamic arbitrary LLM translations!");
}}
```"#,
            "{}", escaped_prompt
        ))
    }
}
