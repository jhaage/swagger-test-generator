use clap::{Parser, ArgEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(
    name = "swagger-test-generator",
    about = "Generate tests from OpenAPI/Swagger specifications",
    version
)]
pub struct Args {
    /// Path to the Swagger/OpenAPI specification file
    #[clap(short, long, value_name = "FILE")]
    pub input: PathBuf,

    /// Output directory for generated tests
    #[clap(short, long, value_name = "DIRECTORY")]
    pub output_dir: PathBuf,

    /// Testing framework to generate tests for
    #[clap(short, long, value_enum)]
    pub framework: TestFramework,

    /// Base URL for the API
    #[clap(long, value_name = "URL", default_value = "http://localhost:3000")]
    pub base_url: String,

    /// Generate detailed test cases
    #[clap(long)]
    pub verbose: bool,
}

#[derive(Debug, Copy, Clone, ArgEnum)]
pub enum TestFramework {
    /// Generate tests for Rust's reqwest library
    Reqwest,
    /// Generate tests for Python's pytest & requests
    Pytest,
    /// Generate tests for JavaScript's Jest & axios
    Jest,
    /// Generate tests for Postman collections
    Postman,
}