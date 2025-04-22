use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::error::Error;

// Function to check if the server is ready
fn check_server_ready(base_url: &str, timeout_secs: u64) -> Result<(), Box<dyn Error>> {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let client = reqwest::blocking::Client::new();
    
    // Use the /v1/users endpoint to check if the API is actually ready
    let health_endpoint = format!("{}/v1/users", base_url.trim_end_matches('/'));
    
    println!("Checking API health at: {}", health_endpoint);
    
    while start_time.elapsed() < timeout {
        match client.get(&health_endpoint).send() {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(());
                }
                println!("Server not ready yet, status: {}", response.status());
            },
            Err(e) => {
                println!("Server not ready yet: {}", e);
            }
        }
        
        thread::sleep(Duration::from_millis(500));
    }
    
    Err("Server did not become ready within the timeout period".into())
}

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let mut test_framework = "reqwest";
    let mut base_url = "http://localhost:8000";
    
    // Process command-line args
    for i in 1..args.len() {
        if args[i] == "--framework" && i + 1 < args.len() {
            test_framework = &args[i + 1];
        }
        if args[i] == "--base-url" && i + 1 < args.len() {
            base_url = &args[i + 1];
        }
    }
    
    // Print banner
    println!("=================================================================");
    println!("🚀 Swagger Test Generator - Development Environment");
    println!("=================================================================");
    
    // Start the sample API in the background
    println!("📡 Starting the sample API server...");
    let server_dir = "examples/sample_api";
    
    // Check if server directory exists
    if !Path::new(server_dir).exists() {
        eprintln!("❌ Sample API directory not found: {}", server_dir);
        return Ok(());
    }
    
    // Extract port from base_url for the API server
    let port = base_url.split(':').last().unwrap_or("8000");
    let port = port.split('/').next().unwrap_or("8000");
    
    let api_server = Command::new("cargo")
        .args(["run", "--quiet", "--", "--port", port])
        .current_dir(server_dir)
        .stdout(Stdio::piped())
        .spawn()?;
    
    // Wait for the server to start and validate it's responding
    println!("⏳ Waiting for the server to start and become ready...");
    match check_server_ready(base_url, 10) {
        Ok(()) => println!("✅ Server started and ready at {}", base_url),
        Err(e) => {
            eprintln!("❌ Server did not start properly: {}", e);
            return Ok(());
        }
    }
    
    // Create the output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    if output_dir.exists() {
        println!("🧹 Cleaning up previous output directory...");
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    
    // Generate tests
    println!("\n📝 Generating tests using '{}' framework...", test_framework);
    let test_generator = Command::new("cargo")
        .args([
            "run",
            "--bin", "swagger-test-generator",
            "--",
            "-i", "tests/samples/sample_swagger.json",
            "-o", "output",
            "-f", test_framework,
            "--base-url", base_url,
        ])
        .output()?;
    
    io::stdout().write_all(&test_generator.stdout)?;
    io::stderr().write_all(&test_generator.stderr)?;
    
    // Check if generation was successful
    if !test_generator.status.success() {
        eprintln!("❌ Failed to generate tests");
        return Ok(());
    }
    
    // Create proper Cargo project structure
    println!("📁 Creating proper Cargo project structure...");
    
    // Create src directory
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    
    // Move the api_tests.rs file to src/lib.rs
    if output_dir.join("api_tests.rs").exists() {
        fs::rename(
            output_dir.join("api_tests.rs"),
            src_dir.join("lib.rs"),
        )?;
    }
    
    // Create a simple main.rs
    let mut main_file = File::create(src_dir.join("main.rs"))?;
    writeln!(main_file, r#"fn main() {{
    println!("Run with 'cargo test' to execute the API tests");
}}"#)?;
    
    // Update the Cargo.toml file to include the tests as a library
    let mut cargo_file = File::create(output_dir.join("Cargo.toml"))?;
    writeln!(cargo_file, r#"[package]
name = "api_tests"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = {{ version = "0.11", features = ["json", "blocking"] }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

[lib]
name = "api_tests"
path = "src/lib.rs"

[[bin]]
name = "api_tests"
path = "src/main.rs"
"#)?;
    
    println!("✅ Tests generated in the 'output' directory");
    
    // Build dependencies first before running tests
    println!("\n📦 Building dependencies...");
    let build_result = Command::new("cargo")
        .args(["build", "--manifest-path", "output/Cargo.toml", "--quiet"])
        .output()?;
        
    if !build_result.status.success() {
        println!("❌ Failed to build test dependencies");
        io::stdout().write_all(&build_result.stdout)?;
        io::stderr().write_all(&build_result.stderr)?;
        return Ok(());
    }
    
    println!("✅ Dependencies built successfully");
    
    // Run the tests
    println!("\n🧪 Running tests against the API...");
    let test_runner = Command::new("cargo")
        .args(["test", "--manifest-path", "output/Cargo.toml"])
        .output()?;
    
    io::stdout().write_all(&test_runner.stdout)?;
    io::stderr().write_all(&test_runner.stderr)?;
    
    if test_runner.status.success() {
        println!("✅ All tests passed!");
    } else {
        println!("❌ Some tests failed");
    }
    
    // Clean up 
    println!("\n🧹 Cleaning up...");
    drop(api_server); // This will terminate the API server process
    
    println!("\n=================================================================");
    println!("🏁 Development session completed");
    println!("=================================================================");
    
    Ok(())
}