# Swagger Test Generator

A command-line tool written in Rust that generates test cases from Swagger/OpenAPI specifications.

## Features

- Parse Swagger 2.0 and OpenAPI 3.0 specifications
- Generate tests in multiple formats:
  - Rust (reqwest)
  - Python (pytest)
  - JavaScript (Jest)
  - Postman collections
- Generate API endpoint implementations for testing
- Validate tests against sample APIs

## Installation

### From crates.io

```bash
cargo install swagger-test-generator
```

### From source

```bash
git clone https://github.com/yourusername/swagger-test-generator.git
cd swagger-test-generator
cargo install --path .
```

## Usage

```bash
# Generate Rust tests
swagger-test-generator -i swagger.json -o ./test-output -f reqwest

# Generate Python tests
swagger-test-generator -i swagger.json -o ./test-output -f pytest

# Generate JavaScript tests
swagger-test-generator -i swagger.json -o ./test-output -f jest

# Generate Postman collection
swagger-test-generator -i swagger.json -o ./test-output -f postman

# Set the base URL for tests
swagger-test-generator -i swagger.json -o ./test-output -f reqwest --base-url https://api.example.com

# Enable verbose test generation
swagger-test-generator -i swagger.json -o ./test-output -f reqwest --verbose
```

### Base URL Handling

The `--base-url` parameter overrides the host part of the API URL while preserving any base path specified in the Swagger/OpenAPI document:

- If your Swagger spec defines `host: api.example.com` and `basePath: /v1`, and you pass `--base-url http://localhost:3000`
- Generated tests will use `http://localhost:3000/v1` as the base URL

This allows you to easily run tests against local development servers while preserving API versioning and paths.

## Example

For a Swagger specification with user CRUD operations, the tool generates test cases for:

- GET /users - List all users
- POST /users - Create a new user
- GET /users/{id} - Get a user by ID
- PUT /users/{id} - Update a user
- DELETE /users/{id} - Delete a user

## Validating with Sample API

The project includes a sample API implementation that matches the included Swagger specification:

### Manual Approach

1. Run the sample API:

```bash
cd examples/sample_api
cargo run
```

2. Generate tests against the specification:

```bash
cargo run -- -i tests/samples/sample_swagger.json -o ./output -f reqwest --base-url http://localhost:3000
```

3. Run the generated tests against the API:

```bash
cd output
cargo test
```

### One-Command Development Workflow

For convenience, a development script is provided that:
1. Starts the sample API server
2. Generates tests with the specified framework
3. Runs the tests against the local API
4. Reports the results

```bash
# Run with default settings (reqwest framework)
cargo run --bin run-dev

# Specify a different test framework
cargo run --bin run-dev -- --framework pytest

# Specify a different base URL
cargo run --bin run-dev -- --base-url http://localhost:8080
```

This is particularly useful for quick development and testing cycles.

## Development

### Architecture

The tool is structured into several modules:

- `cli`: Command-line interface and argument handling
- `parser`: Swagger/OpenAPI specification parsing
- `generator`: Test code generation for different frameworks
- `utils`: Helper utilities

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

## License

MIT