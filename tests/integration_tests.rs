// This file contains integration tests for the CLI application, ensuring that the application behaves as expected when interacting with the Swagger document and generating tests.

#[cfg(test)]
mod tests {
    use swagger_test_generator::{
        cli::TestFramework,
        parser::parse_swagger_file,
        generator::{create_generator, generate_axum_api},
    };
    use std::path::{Path, PathBuf};
    use std::fs;

    fn get_test_data_path(file_name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("samples");
        path.push(file_name);
        path
    }

    #[test]
    fn test_parse_swagger_file() {
        let swagger_path = get_test_data_path("sample_swagger.json");
        let result = parse_swagger_file(&swagger_path);
        assert!(result.is_ok());
        
        let spec = result.unwrap();
        assert_eq!(spec.base_url, "http://api.sample.com/v1");
        
        // Verify we parsed all endpoints
        let all_operations: usize = spec.paths
            .iter()
            .map(|p| p.operations.len())
            .sum();
        
        assert_eq!(all_operations, 5); // We have 5 operations in our sample: GET /users, POST /users, GET /users/{id}, PUT /users/{id}, DELETE /users/{id}
    }

    #[test]
    fn test_generate_reqwest_tests() {
        let swagger_path = get_test_data_path("sample_swagger.json");
        let spec = parse_swagger_file(&swagger_path).unwrap();
        
        let test_output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-output")
            .join("reqwest");
        
        // Clean previous test output
        if test_output_dir.exists() {
            fs::remove_dir_all(&test_output_dir).unwrap();
        }
        fs::create_dir_all(&test_output_dir).unwrap();
        
        // Generate tests
        let generator = create_generator(TestFramework::Reqwest).unwrap();
        let result = generator.generate_tests(&spec, &test_output_dir, "http://localhost:3000");
        
        assert!(result.is_ok());
        
        // Check that test files were created
        assert!(test_output_dir.join("api_tests.rs").exists());
        assert!(test_output_dir.join("main.rs").exists());
        assert!(test_output_dir.join("Cargo.toml").exists());
    }

    #[test]
    fn test_generate_pytest_tests() {
        let swagger_path = get_test_data_path("sample_swagger.json");
        let spec = parse_swagger_file(&swagger_path).unwrap();
        
        let test_output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-output")
            .join("pytest");
        
        // Clean previous test output
        if test_output_dir.exists() {
            fs::remove_dir_all(&test_output_dir).unwrap();
        }
        fs::create_dir_all(&test_output_dir).unwrap();
        
        // Generate tests
        let generator = create_generator(TestFramework::Pytest).unwrap();
        let result = generator.generate_tests(&spec, &test_output_dir, "http://localhost:3000");
        
        assert!(result.is_ok());
        
        // Check that test files were created
        assert!(test_output_dir.join("test_api.py").exists());
        assert!(test_output_dir.join("requirements.txt").exists());
    }

    #[test]
    fn test_generate_api_endpoints() {
        let swagger_path = get_test_data_path("sample_swagger.json");
        let spec = parse_swagger_file(&swagger_path).unwrap();
        
        let api_output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-output")
            .join("generated_api");
        
        // Clean previous test output
        if api_output_dir.exists() {
            fs::remove_dir_all(&api_output_dir).unwrap();
        }
        
        // Generate API
        let result = generate_axum_api(&spec, &api_output_dir);
        
        assert!(result.is_ok());
        
        // Check that API files were created
        assert!(api_output_dir.join("Cargo.toml").exists());
        assert!(api_output_dir.join("src").join("main.rs").exists());
        assert!(api_output_dir.join("src").join("models").exists());
        assert!(api_output_dir.join("src").join("handlers").exists());
        assert!(api_output_dir.join("src").join("routes").exists());
    }
}