// src/parser/swagger.rs

use serde_json::{Value, Error as JsonError};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] JsonError),

    #[error("Unsupported OpenAPI version")]
    UnsupportedVersion,
    
    #[error("Invalid OpenAPI specification: {0}")]
    InvalidSpec(String),
}

pub type Result<T> = std::result::Result<T, ParserError>;

/// Represents a parsed OpenAPI/Swagger specification
#[derive(Debug, Clone)]
pub struct SwaggerSpec {
    /// The raw JSON Value of the parsed specification
    pub raw_spec: Value,
    
    /// Base URL for the API derived from the specification
    pub base_url: String,
    
    /// All paths defined in the API
    pub paths: Vec<ApiPath>,
}

/// Represents an API path with its operations
#[derive(Debug, Clone)]
pub struct ApiPath {
    /// The path template (e.g., "/users/{id}")
    pub path: String,
    
    /// The operations available on this path
    pub operations: Vec<ApiOperation>,
}

/// Represents an API operation (HTTP method + path)
#[derive(Debug, Clone)]
pub struct ApiOperation {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    
    /// Operation ID from the spec, or generated if not present
    pub operation_id: String,
    
    /// Summary of what the operation does
    pub summary: Option<String>,
    
    /// Detailed description of the operation
    pub description: Option<String>,
    
    /// Path parameters required by this operation
    pub path_params: Vec<ApiParameter>,
    
    /// Query parameters accepted by this operation
    pub query_params: Vec<ApiParameter>,
    
    /// Body parameters (if any) for this operation
    pub body_param: Option<ApiParameter>,
    
    /// Possible responses returned by this operation
    pub responses: Vec<ApiResponse>,
}

/// Represents a parameter in an API operation
#[derive(Debug, Clone)]
pub struct ApiParameter {
    /// Name of the parameter
    pub name: String,
    
    /// Location of the parameter (path, query, body)
    pub location: String,
    
    /// Whether the parameter is required
    pub required: bool,
    
    /// Type of the parameter (string, integer, etc.)
    pub param_type: String,
    
    /// Schema definition for complex parameters
    pub schema: Option<Value>,
}

/// Represents a possible API response
#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// HTTP status code
    pub status_code: String,
    
    /// Description of the response
    pub description: Option<String>,
    
    /// Schema of the response body
    pub schema: Option<Value>,
}

/// Parse a Swagger/OpenAPI specification from a file
pub fn parse_swagger_file<P: AsRef<Path>>(path: P) -> Result<SwaggerSpec> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    parse_swagger_string(&content)
}

/// Parse a Swagger/OpenAPI specification from a string
pub fn parse_swagger_string(content: &str) -> Result<SwaggerSpec> {
    let spec: Value = serde_json::from_str(content)?;
    
    // Determine version and validate
    let swagger_version = spec.get("swagger").and_then(Value::as_str);
    let openapi_version = spec.get("openapi").and_then(Value::as_str);
    
    match (swagger_version, openapi_version) {
        (Some("2.0"), _) => parse_swagger_v2(spec),
        (_, Some(v)) if v.starts_with("3.") => parse_openapi_v3(spec),
        _ => Err(ParserError::UnsupportedVersion),
    }
}

/// Parse Swagger 2.0 specification
fn parse_swagger_v2(spec: Value) -> Result<SwaggerSpec> {
    // Extract base URL components
    let scheme = spec
        .get("schemes")
        .and_then(|s| s.as_array())
        .and_then(|a| a.get(0))
        .and_then(Value::as_str)
        .unwrap_or("http");
        
    let host = spec
        .get("host")
        .and_then(Value::as_str)
        .unwrap_or("localhost");
        
    let base_path = spec
        .get("basePath")
        .and_then(Value::as_str)
        .unwrap_or("");
        
    let base_url = format!("{}://{}{}", scheme, host, base_path);
    
    // Extract paths
    let paths_obj = match spec.get("paths") {
        Some(paths) => paths,
        None => return Err(ParserError::InvalidSpec("No paths defined".into())),
    };
    
    let mut paths = Vec::new();
    
    if let Some(paths_map) = paths_obj.as_object() {
        for (path, path_item) in paths_map {
            let mut api_path = ApiPath {
                path: path.clone(),
                operations: Vec::new(),
            };
            
            if let Some(path_obj) = path_item.as_object() {
                for (method, operation) in path_obj {
                    // Skip non-HTTP method keys
                    if !["get", "post", "put", "delete", "patch", "options", "head"].contains(&method.as_str()) {
                        continue;
                    }
                    
                    if let Some(op_obj) = operation.as_object() {
                        let operation_id = op_obj
                            .get("operationId")
                            .and_then(Value::as_str)
                            .unwrap_or(&format!("{}_{}", method, sanitize_path(path)))
                            .to_string();
                            
                        let summary = op_obj
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(String::from);
                            
                        let description = op_obj
                            .get("description")
                            .and_then(Value::as_str)
                            .map(String::from);
                        
                        // Parse parameters
                        let mut path_params = Vec::new();
                        let mut query_params = Vec::new();
                        let mut body_param = None;
                        
                        if let Some(params) = op_obj.get("parameters").and_then(Value::as_array) {
                            for param in params {
                                if let Some(param_obj) = param.as_object() {
                                    let name = param_obj
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                        
                                    let location = param_obj
                                        .get("in")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                        
                                    let required = param_obj
                                        .get("required")
                                        .and_then(Value::as_bool)
                                        .unwrap_or(false);
                                        
                                    let param_type = param_obj
                                        .get("type")
                                        .and_then(Value::as_str)
                                        .unwrap_or_else(|| {
                                            param_obj
                                                .get("schema")
                                                .and_then(|s| s.get("type"))
                                                .and_then(Value::as_str)
                                                .unwrap_or("object")
                                        })
                                        .to_string();
                                    
                                    let schema = param_obj.get("schema").cloned();
                                    
                                    let api_param = ApiParameter {
                                        name,
                                        location: location.clone(),
                                        required,
                                        param_type,
                                        schema,
                                    };
                                    
                                    match location.as_str() {
                                        "path" => path_params.push(api_param),
                                        "query" => query_params.push(api_param),
                                        "body" => body_param = Some(api_param),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        
                        // Parse responses
                        let mut responses = Vec::new();
                        
                        if let Some(resp_obj) = op_obj.get("responses").and_then(Value::as_object) {
                            for (status_code, response) in resp_obj {
                                if let Some(resp_obj) = response.as_object() {
                                    let description = resp_obj
                                        .get("description")
                                        .and_then(Value::as_str)
                                        .map(String::from);
                                        
                                    let schema = resp_obj.get("schema").cloned();
                                    
                                    responses.push(ApiResponse {
                                        status_code: status_code.clone(),
                                        description,
                                        schema,
                                    });
                                }
                            }
                        }
                        
                        let api_operation = ApiOperation {
                            method: method.to_uppercase(),
                            operation_id,
                            summary,
                            description,
                            path_params,
                            query_params,
                            body_param,
                            responses,
                        };
                        
                        api_path.operations.push(api_operation);
                    }
                }
            }
            
            if !api_path.operations.is_empty() {
                paths.push(api_path);
            }
        }
    }
    
    Ok(SwaggerSpec {
        raw_spec: spec,
        base_url,
        paths,
    })
}

/// Parse OpenAPI 3.0 specification
fn parse_openapi_v3(spec: Value) -> Result<SwaggerSpec> {
    // Extract base URL components
    let mut base_url = "http://localhost".to_string();
    
    if let Some(servers) = spec.get("servers").and_then(Value::as_array) {
        if let Some(server) = servers.get(0) {
            if let Some(url) = server.get("url").and_then(Value::as_str) {
                base_url = url.to_string();
            }
        }
    }
    
    // Extract paths
    let paths_obj = match spec.get("paths") {
        Some(paths) => paths,
        None => return Err(ParserError::InvalidSpec("No paths defined".into())),
    };
    
    let mut paths = Vec::new();
    
    if let Some(paths_map) = paths_obj.as_object() {
        for (path, path_item) in paths_map {
            let mut api_path = ApiPath {
                path: path.clone(),
                operations: Vec::new(),
            };
            
            if let Some(path_obj) = path_item.as_object() {
                for (method, operation) in path_obj {
                    // Skip non-HTTP method keys
                    if !["get", "post", "put", "delete", "patch", "options", "head"].contains(&method.as_str()) {
                        continue;
                    }
                    
                    if let Some(op_obj) = operation.as_object() {
                        let operation_id = op_obj
                            .get("operationId")
                            .and_then(Value::as_str)
                            .unwrap_or(&format!("{}_{}", method, sanitize_path(path)))
                            .to_string();
                            
                        let summary = op_obj
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(String::from);
                            
                        let description = op_obj
                            .get("description")
                            .and_then(Value::as_str)
                            .map(String::from);
                        
                        // Parse parameters
                        let mut path_params = Vec::new();
                        let mut query_params = Vec::new();
                        
                        if let Some(params) = op_obj.get("parameters").and_then(Value::as_array) {
                            for param in params {
                                if let Some(param_obj) = param.as_object() {
                                    let name = param_obj
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                        
                                    let location = param_obj
                                        .get("in")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                        
                                    let required = param_obj
                                        .get("required")
                                        .and_then(Value::as_bool)
                                        .unwrap_or(false);
                                        
                                    let schema = param_obj.get("schema").cloned();
                                    
                                    let param_type = if let Some(schema_ref) = schema.as_ref() {
                                        schema_ref
                                            .get("type")
                                            .and_then(Value::as_str)
                                            .unwrap_or("object")
                                            .to_string()
                                    } else {
                                        "string".to_string()
                                    };
                                    
                                    let api_param = ApiParameter {
                                        name,
                                        location: location.clone(),
                                        required,
                                        param_type,
                                        schema,
                                    };
                                    
                                    match location.as_str() {
                                        "path" => path_params.push(api_param),
                                        "query" => query_params.push(api_param),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        
                        // Parse request body for OpenAPI 3.0
                        let body_param = op_obj.get("requestBody").and_then(|body| {
                            let required = body
                                .get("required")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                                
                            let content = body.get("content")?;
                            let json_content = content.get("application/json")?;
                            let schema = json_content.get("schema").cloned();
                            
                            Some(ApiParameter {
                                name: "body".to_string(),
                                location: "body".to_string(),
                                required,
                                param_type: "object".to_string(),
                                schema,
                            })
                        });
                        
                        // Parse responses
                        let mut responses = Vec::new();
                        
                        if let Some(resp_obj) = op_obj.get("responses").and_then(Value::as_object) {
                            for (status_code, response) in resp_obj {
                                if let Some(resp_obj) = response.as_object() {
                                    let description = resp_obj
                                        .get("description")
                                        .and_then(Value::as_str)
                                        .map(String::from);
                                    
                                    let schema = if let Some(content) = resp_obj.get("content") {
                                        if let Some(json_content) = content.get("application/json") {
                                            json_content.get("schema").cloned()
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    
                                    responses.push(ApiResponse {
                                        status_code: status_code.clone(),
                                        description,
                                        schema,
                                    });
                                }
                            }
                        }
                        
                        let api_operation = ApiOperation {
                            method: method.to_uppercase(),
                            operation_id,
                            summary,
                            description,
                            path_params,
                            query_params,
                            body_param,
                            responses,
                        };
                        
                        api_path.operations.push(api_operation);
                    }
                }
            }
            
            if !api_path.operations.is_empty() {
                paths.push(api_path);
            }
        }
    }
    
    Ok(SwaggerSpec {
        raw_spec: spec,
        base_url,
        paths,
    })
}

/// Helper function to sanitize path for use in operation IDs
fn sanitize_path(path: &str) -> String {
    path.replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .trim_start_matches('_')
        .to_string()
}