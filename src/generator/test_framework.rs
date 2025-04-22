use std::path::Path;
use std::fs::{self, File};
use std::io::Write;

use crate::parser::{SwaggerSpec, ApiOperation};
use crate::cli::args::TestFramework;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Unsupported framework: {0:?}")]
    UnsupportedFramework(TestFramework),
}

pub type Result<T> = std::result::Result<T, GeneratorError>;

/// Base trait for all test generators
pub trait TestGenerator {
    /// Generate tests for all operations in the Swagger spec
    fn generate_tests(&self, spec: &SwaggerSpec, output_dir: &Path, base_url: &str) -> Result<()>;
}

/// Factory function to create a test generator based on the framework
pub fn create_generator(framework: TestFramework) -> Result<Box<dyn TestGenerator>> {
    match framework {
        TestFramework::Reqwest => Ok(Box::new(ReqwestGenerator::new())),
        TestFramework::Pytest => Ok(Box::new(PytestGenerator::new())),
        TestFramework::Jest => Ok(Box::new(JestGenerator::new())),
        TestFramework::Postman => Ok(Box::new(PostmanGenerator::new())),
    }
}

// Rust reqwest test generator
struct ReqwestGenerator;

impl ReqwestGenerator {
    pub fn new() -> Self {
        ReqwestGenerator
    }
    
    fn generate_operation_test(&self, operation: &ApiOperation, path: &str, base_url: &str) -> String {
        let method = operation.method.to_lowercase();
        let operation_id = &operation.operation_id;
        
        // Convert camelCase to snake_case for Rust function naming convention
        let snake_case_operation_id = operation_id.chars().fold(String::new(), |mut acc, c| {
            if c.is_uppercase() {
                // Add underscore before uppercase letters, but not at the beginning
                if !acc.is_empty() {
                    acc.push('_');
                }
                acc.push(c.to_lowercase().next().unwrap());
            } else {
                acc.push(c);
            }
            acc
        });
        
        let summary = operation.summary.as_deref().unwrap_or("");
        
        // Special handling for operations that require a specific user ID
        let needs_user_creation = operation.path_params.iter().any(|p| p.name == "id") && 
                                 (method == "get" || method == "put" || method == "delete");
        
        let path_params_decl = if needs_user_creation {
            // Create a test user first if this operation needs a specific user ID
            let test_name = match method.as_str() {
                "get" => "\"Get User Test\"",
                "put" => "\"Update Test\"",
                "delete" => "\"Delete Test\"",
                _ => "\"Test User\"",
            };
            
            let test_email = match method.as_str() {
                "get" => "\"get_test@example.com\"",
                "put" => "\"update_test@example.com\"",
                "delete" => "\"delete_test@example.com\"",
                _ => "\"test@example.com\"",
            };
            
            format!("    // Create a test user first\n    let id = create_test_user({}, {}).await;", test_name, test_email)
        } else {
            operation.path_params.iter()
                .map(|p| format!("    let {} = 1; // TODO: Replace with actual test value for {}", p.name, p.name))
                .collect::<Vec<_>>()
                .join("\n")
        };
            
        let query_params = if !operation.query_params.is_empty() {
            "    let query_params = [".to_string() + &operation.query_params.iter()
                .map(|p| format!(r#"        ("{}", "test_value")"#, p.name))
                .collect::<Vec<_>>()
                .join(",\n") + "\n    ];"
        } else {
            "".to_string()
        };
        
        let body_param = if method == "put" {
            r#"    let body = json!({
        "name": "Updated Name",
        "email": "updated@example.com"
    });"#.to_string()
        } else if operation.body_param.is_some() {
            r#"    let body = json!({
        "name": "Test User",
        "email": "test@example.com"
    });"#.to_string()
        } else {
            "".to_string()
        };
            
        // Create path with parameter interpolation
        let mut endpoint_path = path.to_string();
        for param in &operation.path_params {
            endpoint_path = endpoint_path.replace(&format!("{{{}}}", param.name), &format!("{{{}}}", param.name));
        }
        
        let client_method = match method.as_str() {
            "get" => "client.get(&url)",
            "post" => {
                if operation.body_param.is_some() {
                    "client.post(&url).json(&body)"
                } else {
                    "client.post(&url)"
                }
            },
            "put" => {
                if operation.body_param.is_some() {
                    "client.put(&url).json(&body)"
                } else {
                    "client.put(&url)"
                }
            },
            "delete" => "client.delete(&url)",
            _ => "client.get(&url)",
        };
        
        let query_params_apply = if !operation.query_params.is_empty() {
            ".query(&query_params)"
        } else {
            ""
        };
        
        let mut expected_status = "200";
        if method == "post" {
            expected_status = "201";
        } else if method == "delete" {
            expected_status = "204";
        }
        
        // Find the expected status from the responses
        for resp in &operation.responses {
            if resp.status_code.starts_with('2') {
                expected_status = &resp.status_code;
                break;
            }
        }
        
        // Additional verification for delete operation
        let additional_verification = if method == "delete" {
            format!(r#"
    // Verify the user is deleted by trying to get it
    let get_response = client.get(&url)
        .send()
        .await
        .expect("Failed to send GET request");
        
    assert_eq!(get_response.status().as_u16(), 404);"#)
        } else if method == "get" && operation.path_params.iter().any(|p| p.name == "id") {
            // Add verification for get user by ID
            r#"
    // Verify the response body contains the right data
    let user: User = response.json().await.expect("Failed to parse response");
    assert_eq!(user.id, id);"#.to_string()
        } else if method == "put" {
            // Add verification for update user
            r#"
    // Verify the response body
    let user: User = response.json().await.expect("Failed to parse response");
    assert_eq!(user.name, "Updated Name");
    assert_eq!(user.email, "updated@example.com");"#.to_string()
        } else if method == "post" && path.contains("users") && !path.contains("{") {
            // Add verification for create user
            r#"
    // Verify the response body
    let user: User = response.json().await.expect("Failed to parse response");
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email, "test@example.com");"#.to_string()
        } else if method == "get" && !path.contains("{") {
            // Add verification for get all users
            r#"
    // Verify the response body contains users
    let users: Vec<User> = response.json().await.expect("Failed to parse response");
    assert!(!users.is_empty(), "Expected users array to not be empty");"#.to_string()
        } else {
            "".to_string()
        };
        
        format!(
            r#"#[tokio::test]
async fn test_{snake_case_operation_id}() {{
    // {summary}
{path_params_decl}
{query_params}
{body_param}

    let client = reqwest::Client::new();
    let url = format!("{base_url}{endpoint_path}");
    
    let response = {client_method}{query_params_apply}
        .send()
        .await
        .expect("Failed to send request");
        
    assert_eq!(response.status().as_u16(), {expected_status});{additional_verification}
}}
"#
        )
    }
}

impl TestGenerator for ReqwestGenerator {
    fn generate_tests(&self, spec: &SwaggerSpec, output_dir: &Path, base_url: &str) -> Result<()> {
        // Create the output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Extract the base path from the spec's base_url
        // The base_url in the spec contains something like "http://api.sample.com/v1"
        // We need to extract the "/v1" part to append to our custom base URL
        let base_path = if let Some(url_parts) = spec.base_url.split("://").nth(1) {
            // Get everything after the host (domain)
            if let Some(path) = url_parts.find('/') {
                let base_path = &url_parts[path..];
                if !base_path.is_empty() {
                    base_path
                } else {
                    ""
                }
            } else {
                ""
            }
        } else {
            ""
        };
        
        // Combine our command line base_url with the base path from the spec
        // Make sure we don't have double slashes
        let final_base_url = if base_url.ends_with('/') || base_path.starts_with('/') {
            format!("{}{}", base_url.trim_end_matches('/'), base_path)
        } else if !base_path.is_empty() {
            format!("{}/{}", base_url, base_path.trim_start_matches('/'))
        } else {
            base_url.to_string()
        };
        
        // Create a single test file for all operations
        let test_file_path = output_dir.join("api_tests.rs");
        let mut file = File::create(test_file_path)?;
        
        // Write the file header with common helpers and structs
        write!(file, r#"use serde_json::json;
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Serialize, Deserialize)]
struct User {{
    id: i64,
    name: String,
    email: String,
    created_at: String,
    updated_at: Option<String>,
}}

// Helper function to create a test user and return its ID
async fn create_test_user(name: &str, email: &str) -> i64 {{
    let body = json!({{
        "name": name,
        "email": email
    }});

    let client = reqwest::Client::new();
    let url = "{}/users";
    
    let response = client.post(url).json(&body)
        .send()
        .await
        .expect("Failed to create test user");
        
    assert_eq!(response.status().as_u16(), 201);
    
    let user: User = response.json().await.expect("Failed to parse user response");
    user.id
}}
"#, final_base_url)?;
        
        // Generate tests for each operation
        for path in &spec.paths {
            for operation in &path.operations {
                let test_code = self.generate_operation_test(operation, &path.path, &final_base_url);
                writeln!(file, "{}\n", test_code)?;
            }
        }
        
        // Write a main test file that includes the test module
        let main_file_path = output_dir.join("main.rs");
        let mut main_file = File::create(main_file_path)?;
        
        writeln!(main_file, r#"#[cfg(test)]
mod api_tests;

fn main() {{
    println!("Run with 'cargo test' to execute the API tests");
}}
"#)?;
        
        // Create a Cargo.toml for the test project
        let cargo_file_path = output_dir.join("Cargo.toml");
        let mut cargo_file = File::create(cargo_file_path)?;
        
        writeln!(cargo_file, r#"[package]
name = "api_tests"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = {{ version = "0.11", features = ["json", "blocking"] }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#)?;
        
        Ok(())
    }
}

// Python pytest test generator
struct PytestGenerator;

impl PytestGenerator {
    pub fn new() -> Self {
        PytestGenerator
    }
    
    fn generate_operation_test(&self, operation: &ApiOperation, path: &str, base_url: &str) -> String {
        let method = operation.method.to_lowercase();
        let operation_id = &operation.operation_id;
        let summary = operation.summary.as_deref().unwrap_or("");
        
        // Parameter setup
        let path_params_setup = operation.path_params.iter()
            .map(|p| format!("    # Path parameter: {}\n    {} = 1  # Replace with actual test value", p.name, p.name))
            .collect::<Vec<_>>()
            .join("\n");
        
        let query_params = if !operation.query_params.is_empty() {
            "    params = {\n".to_string() + &operation.query_params.iter()
                .map(|p| format!(r#"        "{}": "test_value""#, p.name))
                .collect::<Vec<_>>()
                .join(",\n") + "\n    }"
        } else {
            "    params = {}".to_string()
        };
        
        let body_param = operation.body_param.as_ref()
            .map(|_| r#"    json_data = {
        "name": "Test User",
        "email": "test@example.com"
    }"#.to_string())
            .unwrap_or_else(|| "    json_data = None".to_string());
        
        // Create path with parameter interpolation
        let mut endpoint_path = path.to_string();
        for param in &operation.path_params {
            endpoint_path = endpoint_path.replace(&format!("{{{}}}", param.name), &format!("{{{}}}", param.name));
        }
        
        // Request construction
        let request_call = match method.as_str() {
            "get" => "response = requests.get(url, params=params)",
            "post" => "response = requests.post(url, json=json_data, params=params)",
            "put" => "response = requests.put(url, json=json_data, params=params)",
            "delete" => "response = requests.delete(url, params=params)",
            _ => "response = requests.get(url, params=params)",
        };
        
        // Expected status code
        let mut expected_status = "200";
        if method == "post" {
            expected_status = "201";
        } else if method == "delete" {
            expected_status = "204";
        }
        
        // Find the expected status from the responses
        for resp in &operation.responses {
            if resp.status_code.starts_with('2') {
                expected_status = &resp.status_code;
                break;
            }
        }
        
        format!(
            r#"def test_{operation_id}():
    """
    {summary}
    """
{path_params_setup}
{query_params}
{body_param}

    url = f"{base_url}{endpoint_path}"
    {request_call}
    
    # Verify status code
    assert response.status_code == {expected_status}
    
    # Verify the response body
    # response_json = response.json()
    # assert "id" in response_json
"#
        )
    }
}

impl TestGenerator for PytestGenerator {
    fn generate_tests(&self, spec: &SwaggerSpec, output_dir: &Path, base_url: &str) -> Result<()> {
        // Create the output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Create a single test file for all operations
        let test_file_path = output_dir.join("test_api.py");
        let mut file = File::create(test_file_path)?;
        
        // Write the file header
        writeln!(file, "import requests\nimport pytest\n")?;
        
        // Generate tests for each operation
        for path in &spec.paths {
            for operation in &path.operations {
                let test_code = self.generate_operation_test(operation, &path.path, base_url);
                writeln!(file, "{}\n", test_code)?;
            }
        }
        
        // Create a requirements.txt file
        let req_file_path = output_dir.join("requirements.txt");
        let mut req_file = File::create(req_file_path)?;
        
        writeln!(req_file, "requests==2.28.1\npytest==7.3.1")?;
        
        // Create a README.md file with instructions
        let readme_file_path = output_dir.join("README.md");
        let mut readme_file = File::create(readme_file_path)?;
        
        writeln!(readme_file, r#"# API Tests

Generated API tests for the Swagger/OpenAPI specification.

## Setup

Install the requirements:

```
pip install -r requirements.txt
```

## Running the tests

To run the tests:

```
pytest -v
```
"#)?;
        
        Ok(())
    }
}

// JavaScript Jest test generator
struct JestGenerator;

impl JestGenerator {
    pub fn new() -> Self {
        JestGenerator
    }
    
    fn generate_operation_test(&self, operation: &ApiOperation, path: &str, base_url: &str) -> String {
        let method = operation.method.to_lowercase();
        let operation_id = &operation.operation_id;
        let summary = operation.summary.as_deref().unwrap_or("");
        
        // Parameter setup
        let path_params_setup = operation.path_params.iter()
            .map(|p| format!("  // Path parameter: {}\n  const {} = 1; // Replace with actual test value", p.name, p.name))
            .collect::<Vec<_>>()
            .join("\n");
        
        let query_params = if !operation.query_params.is_empty() {
            "  const params = {\n".to_string() + &operation.query_params.iter()
                .map(|p| format!(r#"    {}: "test_value""#, p.name))
                .collect::<Vec<_>>()
                .join(",\n") + "\n  };"
        } else {
            "  const params = {};".to_string()
        };
        
        let body_param = operation.body_param.as_ref()
            .map(|_| r#"  const jsonData = {
    name: "Test User",
    email: "test@example.com"
  };"#.to_string())
            .unwrap_or_else(|| "  const jsonData = null;".to_string());
        
        // Create path with parameter interpolation
        let mut endpoint_path = path.to_string();
        for param in &operation.path_params {
            endpoint_path = endpoint_path.replace(&format!("{{{}}}", param.name), &format!("${{{}}}", param.name));
        }
        
        // Request method options
        let request_params = match method.as_str() {
            "get" | "delete" => "{ params }",
            _ => "jsonData, { params }",
        };
        
        // Expected status code
        let mut expected_status = "200";
        if method == "post" {
            expected_status = "201";
        } else if method == "delete" {
            expected_status = "204";
        }
        
        // Find the expected status from the responses
        for resp in &operation.responses {
            if resp.status_code.starts_with('2') {
                expected_status = &resp.status_code;
                break;
            }
        }
        
        format!(
            r#"test('{operation_id}', async () => {{
  // {summary}
{path_params_setup}
{query_params}
{body_param}

  const url = `{base_url}{endpoint_path}`;
  
  const response = await axios.{method}(url, {request_params});
  
  // Verify status code
  expect(response.status).toBe({expected_status});
  
  // Verify the response body
  // expect(response.data).toHaveProperty('id');
}});"#
        )
    }
}

impl TestGenerator for JestGenerator {
    fn generate_tests(&self, spec: &SwaggerSpec, output_dir: &Path, base_url: &str) -> Result<()> {
        // Create the output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Create a test file for each path
        for path in &spec.paths {
            let path_name = path.path
                .trim_start_matches('/')
                .replace('/', "_")
                .replace('{', "")
                .replace('}', "");
                
            let test_file_path = output_dir.join(format!("{}.test.js", path_name));
            let mut file = File::create(test_file_path)?;
            
            // Write the file header
            writeln!(file, "const axios = require('axios');\n")?;
            
            // Generate tests for each operation in this path
            for operation in &path.operations {
                let test_code = self.generate_operation_test(operation, &path.path, base_url);
                writeln!(file, "{}\n", test_code)?;
            }
        }
        
        // Create a package.json file
        let package_file_path = output_dir.join("package.json");
        let mut package_file = File::create(package_file_path)?;
        
        writeln!(package_file, r#"{{
  "name": "api-tests",
  "version": "1.0.0",
  "description": "Generated API tests for the Swagger/OpenAPI specification",
  "scripts": {{
    "test": "jest"
  }},
  "dependencies": {{
    "axios": "^1.3.4"
  }},
  "devDependencies": {{
    "jest": "^29.5.0"
  }}
}}
"#)?;
        
        // Create a README.md file with instructions
        let readme_file_path = output_dir.join("README.md");
        let mut readme_file = File::create(readme_file_path)?;
        
        writeln!(readme_file, r#"# API Tests

Generated API tests for the Swagger/OpenAPI specification.

## Setup

Install the dependencies:

```
npm install
```

## Running the tests

To run the tests:

```
npm test
```
"#)?;
        
        Ok(())
    }
}

// Postman collection generator
struct PostmanGenerator;

impl PostmanGenerator {
    pub fn new() -> Self {
        PostmanGenerator
    }
}

impl TestGenerator for PostmanGenerator {
    fn generate_tests(&self, spec: &SwaggerSpec, output_dir: &Path, base_url: &str) -> Result<()> {
        // Create the output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Create a Postman collection file
        let collection_file_path = output_dir.join("postman_collection.json");
        let mut file = File::create(collection_file_path)?;
        
        // Collection ID and metadata
        let collection_id = uuid::Uuid::new_v4().to_string();
        let collection_name = "API Tests";
        
        // Write collection header
        writeln!(file, r#"{{
  "info": {{
    "_postman_id": "{}",
    "name": "{}",
    "description": "Generated API tests for the Swagger/OpenAPI specification",
    "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
  }},
  "item": ["#, collection_id, collection_name)?;
        
        // Group requests by path
        let mut is_first_path = true;
        
        for path in &spec.paths {
            if !is_first_path {
                writeln!(file, ",")?;
            }
            
            // Sanitize path name for folder name
            let folder_name = path.path
                .trim_start_matches('/')
                .replace('/', " ")
                .replace('{', "")
                .replace('}', "");
                
            // Start path folder
            writeln!(file, r#"    {{
      "name": "{}",
      "item": ["#, folder_name)?;
                
            // Add requests for each operation
            let mut is_first_op = true;
            
            for operation in &path.operations {
                if !is_first_op {
                    writeln!(file, ",")?;
                }
                
                let method = operation.method.to_uppercase();
                let summary = operation.summary.as_deref().unwrap_or(&operation.operation_id);
                
                // Create URL with parameter placeholders
                let mut url = format!("{}{}", base_url, path.path);
                
                // Example path parameter values
                for param in &operation.path_params {
                    url = url.replace(&format!("{{{}}}", param.name), &format!(":{}", param.name));
                }
                
                // Query parameters
                let query_params = if !operation.query_params.is_empty() {
                    let params = operation.query_params.iter()
                        .map(|p| {
                            format!(
                                r#"            {{
              "key": "{}",
              "value": "test_value",
              "description": "{}"
            }}"#, 
                                p.name,
                                p.name
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",\n");
                        
                    format!(r#"          "query": [
{}
          ],"#, params)
                } else {
                    "".to_string()
                };
                
                // Request body
                let body = if operation.body_param.is_some() {
                    r#"          "body": {
            "mode": "raw",
            "raw": "{\n  \"name\": \"Test User\",\n  \"email\": \"test@example.com\"\n}",
            "options": {
              "raw": {
                "language": "json"
              }
            }
          },"#
                } else {
                    ""
                };
                
                // Tests for validating responses
                let mut expected_status = "200";
                if method == "POST" {
                    expected_status = "201";
                } else if method == "DELETE" {
                    expected_status = "204";
                }
                
                // Find the expected status from the responses
                for resp in &operation.responses {
                    if resp.status_code.starts_with('2') {
                        expected_status = &resp.status_code;
                        break;
                    }
                }
                
                let tests = format!(
                    r#"          "event": [
            {{
              "listen": "test",
              "script": {{
                "exec": [
                  "pm.test(\"Status code is {}\", function () {{",
                  "    pm.response.to.have.status({});",
                  "}})"
                ],
                "type": "text/javascript"
              }}
            }}
          ],"#, 
                    expected_status, expected_status
                );
                
                // Write the request
                writeln!(file, r#"        {{
          "name": "{} {}",
          "request": {{
            "method": "{}",
            "header": [],
{}
{}
            "url": {{
              "raw": "{}",
              "host": [
                "{}"
              ],
              "path": [{}
              ]
            }},
            "description": "{}"
          }},
{}
          "response": []
        }}"#,
                    method, summary,
                    method,
                    query_params,
                    body,
                    url,
                    base_url.replace("http://", "").replace("https://", "").split('/').next().unwrap_or("localhost"),
                    path.path.trim_start_matches('/').split('/').map(|p| format!("                \"{}\"", p.replace("{", ":").replace("}", ""))).collect::<Vec<_>>().join(",\n"),
                    operation.description.as_deref().unwrap_or(""),
                    tests
                )?;
                
                is_first_op = false;
            }
            
            // Close path folder
            writeln!(file, r#"
      ]
    }}"#)?;
            
            is_first_path = false;
        }
        
        // Close collection
        writeln!(file, r#"
  ],
  "event": []
}}"#)?;
        
        // Create a README.md file with instructions
        let readme_file_path = output_dir.join("README.md");
        let mut readme_file = File::create(readme_file_path)?;
        
        writeln!(readme_file, r#"# Postman API Tests

Generated Postman collection for testing the Swagger/OpenAPI specification.

## Setup

1. Import the `postman_collection.json` file into Postman
2. Create an environment and set the base URL if needed

## Running the tests

Run the collection in Postman and review the test results.
"#)?;
        
        Ok(())
    }
}