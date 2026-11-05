use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// RAII Guard that ensures child processes are killed if the test panics
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn wait_with_output(mut self) -> std::io::Result<std::process::Output> {
        let child = self.0.take().expect("Child already consumed");
        child.wait_with_output()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Helper to get an available ephemeral port from the OS
fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Wait until the HTTP server is listening on the given port
fn wait_for_server(port: u16, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn test_http_server_lifecycle() {
    let port = get_free_port();
    let instruction = format!("create a http server on port {}", port);

    let child = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args(["eval", &instruction, "--backend", "mock"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn ailang eval");

    let guard = ChildGuard::new(child);

    // Wait for the HTTP server to start listening
    let ready = wait_for_server(port, Duration::from_secs(8));
    assert!(ready, "HTTP server failed to bind and listen on port {}", port);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("Failed to build HTTP client");

    // 1. Test GET / (root endpoint)
    let root_url = format!("http://127.0.0.1:{}/", port);
    let root_resp = client.get(&root_url).send().expect("Failed to GET /");
    assert_eq!(root_resp.status(), reqwest::StatusCode::OK);
    let root_body = root_resp.text().unwrap();
    assert!(
        root_body.contains("Welcome to Ailang HTTP Server!"),
        "Unexpected body: {}",
        root_body
    );

    // 2. Test GET /health (JSON API endpoint)
    let health_url = format!("http://127.0.0.1:{}/health", port);
    let health_resp = client.get(&health_url).send().expect("Failed to GET /health");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
    let health_json: serde_json::Value = health_resp.json().expect("Failed to parse JSON response");
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["server"], "ailang");

    // 3. Test GET /hello (Plain text endpoint)
    let hello_url = format!("http://127.0.0.1:{}/hello", port);
    let hello_resp = client.get(&hello_url).send().expect("Failed to GET /hello");
    assert_eq!(hello_resp.status(), reqwest::StatusCode::OK);
    let hello_body = hello_resp.text().unwrap();
    assert!(hello_body.contains("Hello, World from Ailang HTTP Server!"));

    // 4. Test GET /shutdown (Clean graceful termination)
    let shutdown_url = format!("http://127.0.0.1:{}/shutdown", port);
    let shutdown_resp = client.get(&shutdown_url).send().expect("Failed to GET /shutdown");
    assert_eq!(shutdown_resp.status(), reqwest::StatusCode::OK);
    let shutdown_body = shutdown_resp.text().unwrap();
    assert!(shutdown_body.contains("Server shutting down"));

    // Verify ailang process exited cleanly with code 0
    let output = guard.wait_with_output().expect("Failed to wait on ailang process");
    assert!(output.status.success(), "ailang exited with non-zero status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("HTTP server listening on http://127.0.0.1:{}", port)));
    assert!(stdout.contains("Handling request: GET /"));
    assert!(stdout.contains("Handling request: GET /health"));
    assert!(stdout.contains("Shutdown request received. Server stopping."));
}

#[test]
fn test_http_server_from_script_file() {
    let port = get_free_port();

    let child = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args([
            "run",
            "examples/http_server.ail",
            "--backend",
            "mock",
            "--",
            &port.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run http_server.ail");

    let guard = ChildGuard::new(child);

    let ready = wait_for_server(port, Duration::from_secs(8));
    assert!(ready, "Server failed to start from script on port {}", port);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    // Verify /health endpoint
    let health_resp = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .expect("Failed to query /health");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);

    // Send /shutdown
    let shutdown_resp = client
        .get(format!("http://127.0.0.1:{}/shutdown", port))
        .send()
        .expect("Failed to send /shutdown");
    assert_eq!(shutdown_resp.status(), reqwest::StatusCode::OK);

    let output = guard.wait_with_output().expect("Failed to wait for script execution");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("HTTP server listening on http://127.0.0.1:{}", port)));
}

#[test]
fn test_http_server_transpile_and_run() {
    let temp_rs = std::env::temp_dir().join(format!("test_http_transpile_{}.rs", std::process::id()));
    let temp_bin = std::env::temp_dir().join(format!("test_http_transpile_bin_{}", std::process::id()));

    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args([
            "transpile",
            "create a http server on port 8080",
            "-o",
            temp_rs.to_str().unwrap(),
            "--backend",
            "mock",
        ])
        .output()
        .expect("Failed to transpile HTTP server instruction");

    assert!(output.status.success());
    assert!(temp_rs.exists());

    let rust_code = std::fs::read_to_string(&temp_rs).unwrap();
    assert!(rust_code.contains("TcpListener"));
    assert!(rust_code.contains("handle_client"));
    assert!(rust_code.contains("HTTP/1.1"));

    // Compile with rustc
    let rustc_status = Command::new("rustc")
        .arg(&temp_rs)
        .arg("-o")
        .arg(&temp_bin)
        .arg("--edition=2024")
        .status()
        .expect("Failed to compile generated Rust file with rustc");
    assert!(rustc_status.success());

    // Run compiled binary on a free port
    let port = get_free_port();
    let child = Command::new(&temp_bin)
        .arg(port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn transpiled HTTP server binary");

    let guard = ChildGuard::new(child);
    assert!(wait_for_server(port, Duration::from_secs(5)));

    let client = reqwest::blocking::Client::new();
    let resp = client.get(format!("http://127.0.0.1:{}/hello", port)).send().unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Shutdown
    let _ = client.get(format!("http://127.0.0.1:{}/shutdown", port)).send();

    let bin_output = guard.wait_with_output().unwrap();
    assert!(bin_output.status.success());

    let _ = std::fs::remove_file(temp_rs);
    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_http_server_concurrent_requests() {
    let port = get_free_port();

    let child = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args([
            "eval",
            &format!("create a http server on port {}", port),
            "--backend",
            "mock",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn ailang");

    let guard = ChildGuard::new(child);
    assert!(wait_for_server(port, Duration::from_secs(8)));

    let client = Arc::new(
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap(),
    );

    let mut handles = Vec::new();
    for i in 0..10 {
        let client_clone = Arc::clone(&client);
        let path = match i % 3 {
            0 => "/",
            1 => "/health",
            _ => "/hello",
        };
        let url = format!("http://127.0.0.1:{}{}", port, path);
        handles.push(thread::spawn(move || {
            let resp = client_clone.get(&url).send().expect("Request failed");
            assert_eq!(resp.status(), reqwest::StatusCode::OK);
        }));
    }

    for handle in handles {
        handle.join().expect("Concurrent request thread failed");
    }

    // Clean shutdown
    let _ = client
        .get(format!("http://127.0.0.1:{}/shutdown", port))
        .send();

    let output = guard.wait_with_output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_http_server_self_test_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_ailang"))
        .args([
            "eval",
            "create a http server and test it",
            "--backend",
            "mock",
        ])
        .output()
        .expect("Failed to execute ailang self-test");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("HTTP server listening on"));
    assert!(stdout.contains("Client received HTTP response"));
    assert!(stdout.contains("HTTP/1.1 200 OK"));
    assert!(stdout.contains("HTTP server self-test completed successfully!"));
}
