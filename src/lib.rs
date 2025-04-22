pub mod cli;
pub mod parser;
pub mod generator;
pub mod utils;

// Re-export frequently used items for easier access
pub use cli::args::TestFramework;
pub use parser::{parse_swagger_file, SwaggerSpec};
pub use generator::{create_generator, TestGenerator};

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Parser error: {0}")]
    ParserError(#[from] parser::ParserError),
    
    #[error("Generator error: {0}")]
    GeneratorError(#[from] generator::GeneratorError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

/// Generate tests from a Swagger/OpenAPI specification file
pub fn generate_tests_from_spec<P: AsRef<Path>, Q: AsRef<Path>>(
    input_file: P,
    output_dir: Q,
    framework: TestFramework,
    base_url: &str,
) -> Result<()> {
    // Parse the Swagger/OpenAPI specification
    let spec = parser::parse_swagger_file(input_file)?;
    
    // Create the appropriate test generator
    let generator = generator::create_generator(framework)?;
    
    // Generate tests
    generator.generate_tests(&spec, output_dir.as_ref(), base_url)?;
    
    Ok(())
}