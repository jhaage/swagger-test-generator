// This is the entry point for the CLI application.
// It parses command-line arguments and delegates to the appropriate module for further processing.

use std::process;
use clap::Parser;
use cli::Args;
use swagger_test_generator::{generate_tests_from_spec, TestFramework};

mod cli;

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Map the framework argument to the correct TestFramework variant
    let framework = match args.framework {
        cli::args::TestFramework::Reqwest => TestFramework::Reqwest,
        cli::args::TestFramework::Pytest => TestFramework::Pytest,
        cli::args::TestFramework::Jest => TestFramework::Jest,
        cli::args::TestFramework::Postman => TestFramework::Postman,
    };

    // Generate tests from the Swagger/OpenAPI specification
    if let Err(err) = generate_tests_from_spec(&args.input, &args.output_dir, framework, &args.base_url) {
        eprintln!("Error generating tests: {}", err);
        process::exit(1);
    }

    println!("Tests generated successfully in {}", args.output_dir.display());
}